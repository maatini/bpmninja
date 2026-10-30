use std::collections::{HashMap, HashSet};
use std::time::Duration;

use quick_xml::Reader;
use quick_xml::de::from_str;
use quick_xml::events::{BytesStart, Event};

use engine_core::error::{EngineError, EngineResult};
use engine_core::model::{BpmnElement, ListenerEvent, ProcessDefinition, ProcessDefinitionBuilder};
use engine_core::validate_condition;

use crate::models::{BpmnDefinitions, BpmnExtensionElements};

/// Helper to attach parsed listeners to the builder
fn add_listeners(
    mut builder: ProcessDefinitionBuilder,
    node_id: &str,
    ext_elements: Option<BpmnExtensionElements>,
) -> ProcessDefinitionBuilder {
    if let Some(exts) = ext_elements {
        for l in exts.execution_listeners {
            let evt = match l.event.as_str() {
                "start" => ListenerEvent::Start,
                "end" => ListenerEvent::End,
                _ => continue,
            };
            if let Some(s) = l.script {
                builder = builder.listener(node_id, evt, s.content.trim());
            }
        }
    }
    builder
}

use chrono::DateTime;
use chrono::Utc;
/// Parse ISO 8601 time-duration (PT subset: hours, minutes, seconds).
///
/// Supported formats: `PT5S`, `PT1H30M`, `PT10M`, `PT1H`.
/// Returns `Err` for invalid input (empty, no PT prefix, unknown units).
use engine_core::timer_definition::TimerDefinition;

/// Parse full ISO 8601 duration (P-prefix for date components, T-prefix for time).
///
/// Supported formats:
/// - PT30S, PT5M, PT1H30M (time-only, existing)
/// - P1D, P1DT12H (days + optional time)
/// - P1W (weeks)  
/// - P1M, P1Y, P1Y6M (months/years → converted to approximate days)
/// - P1DT2H30M15S (full mixed format)
fn parse_iso8601_duration(s: &str) -> EngineResult<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(EngineError::InvalidDefinition(
            "Timer duration is empty".into(),
        ));
    }
    if !s.starts_with('P') {
        return Err(EngineError::InvalidDefinition(format!(
            "Invalid ISO 8601 duration '{}': must start with 'P'",
            s
        )));
    }
    let body = &s[1..];
    if body.is_empty() {
        return Err(EngineError::InvalidDefinition(format!(
            "Invalid ISO 8601 duration '{}': no value after 'P'",
            s
        )));
    }

    let mut total_secs: u64 = 0;
    let mut current_num = String::new();
    let mut in_time_part = false;
    let mut has_value = false;

    for c in body.chars() {
        if c == 'T' {
            if !current_num.is_empty() {
                return Err(EngineError::InvalidDefinition(format!(
                    "Invalid ISO 8601 duration '{}': digits before 'T' without unit",
                    s
                )));
            }
            in_time_part = true;
            continue;
        }
        if c.is_ascii_digit() {
            current_num.push(c);
            continue;
        }
        if current_num.is_empty() {
            return Err(EngineError::InvalidDefinition(format!(
                "Invalid ISO 8601 duration '{}': missing number before '{}'",
                s, c
            )));
        }
        let val: u64 = current_num.parse().map_err(|_| {
            EngineError::InvalidDefinition(format!("Invalid number in duration '{}'", s))
        })?;
        let multiplier: u64 = match (in_time_part, c) {
            (false, 'Y') => 365 * 86400, // approximate
            (false, 'M') => 30 * 86400,  // approximate
            (false, 'W') => 7 * 86400,
            (false, 'D') => 86400,
            (true, 'H') => 3600,
            (true, 'M') => 60,
            (true, 'S') => 1,
            _ => {
                return Err(EngineError::InvalidDefinition(format!(
                    "Invalid ISO 8601 duration '{}': unknown unit '{}' (time_part={})",
                    s, c, in_time_part
                )));
            }
        };

        let added_secs = val.checked_mul(multiplier).ok_or_else(|| {
            EngineError::InvalidDefinition(format!(
                "Duration segment '{}{}' overflows seconds",
                val, c
            ))
        })?;

        total_secs = total_secs.checked_add(added_secs).ok_or_else(|| {
            EngineError::InvalidDefinition(format!(
                "Total duration '{}' overflows maximum allowed seconds",
                s
            ))
        })?;
        has_value = true;
        current_num.clear();
    }

    if !current_num.is_empty() {
        return Err(EngineError::InvalidDefinition(format!(
            "Invalid ISO 8601 duration '{}': trailing digits without unit",
            s
        )));
    }

    if !has_value {
        return Err(EngineError::InvalidDefinition(format!(
            "Invalid ISO 8601 duration '{}': no duration components found",
            s
        )));
    }

    Ok(Duration::from_secs(total_secs))
}

fn text_value(expr: &Option<crate::models::BpmnTextValue>) -> Option<String> {
    expr.as_ref()
        .and_then(|v| v.value.clone())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn nonempty_attr(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn first_attr(candidates: [Option<String>; 3]) -> Option<String> {
    candidates.into_iter().find_map(nonempty_attr)
}

fn service_task_topic(task: &crate::models::BpmnServiceTask) -> String {
    first_attr([
        task.topic.clone(),
        task.handler.clone(),
        task.camunda_topic.clone(),
    ])
    .unwrap_or_else(|| task.id.clone())
}

fn user_task_assignee(task: &crate::models::BpmnUserTask) -> String {
    first_attr([task.assignee.clone(), task.camunda_assignee.clone(), None])
        .unwrap_or_else(|| "unassigned".into())
}

fn resolve_message_ref(
    message_ref: Option<String>,
    lookup: &HashMap<String, String>,
) -> Option<String> {
    nonempty_attr(message_ref).map(|r| lookup.get(&r).cloned().unwrap_or(r))
}

/// Löst einen `signalRef` über die globalen `<signal>`-Definitionen auf.
///
/// Signale haben keine generische Ersatzdefinition: ein fehlender oder unbekannter
/// Verweis macht das Modell ungültig.
fn resolve_signal_ref(
    signal_ref: Option<String>,
    lookup: &HashMap<String, String>,
    element_kind: &str,
    element_id: &str,
) -> EngineResult<String> {
    let Some(signal_ref) = nonempty_attr(signal_ref) else {
        return Err(EngineError::InvalidDefinition(format!(
            "signalEventDefinition on {element_kind} '{element_id}' has no signalRef"
        )));
    };
    lookup.get(&signal_ref).cloned().ok_or_else(|| {
        EngineError::InvalidDefinition(format!(
            "signalEventDefinition on {element_kind} '{element_id}' references unknown signal '{signal_ref}'"
        ))
    })
}

/// Liest die Bedingung eines `<conditionalEventDefinition>` und validiert sie.
fn parse_conditional_condition(
    definition: &crate::models::BpmnConditionalEventDefinition,
    element_kind: &str,
    element_id: &str,
) -> EngineResult<String> {
    let condition = definition
        .condition
        .as_ref()
        .map(|c| c.value.trim())
        .filter(|c| !c.is_empty())
        .ok_or_else(|| {
            EngineError::InvalidDefinition(format!(
                "conditionalEventDefinition on {element_kind} '{element_id}' has no conditionExpression"
            ))
        })?;
    validate_condition(condition).map_err(|error| {
        EngineError::InvalidDefinition(format!(
            "invalid condition on conditional event '{element_id}': {error}"
        ))
    })?;
    Ok(condition.to_string())
}

fn send_task_message_name(
    task: &crate::models::BpmnSendTask,
    lookup: &HashMap<String, String>,
) -> String {
    let nested_ref = task
        .message_event_definition
        .as_ref()
        .and_then(|m| m.message_ref.clone());
    resolve_message_ref(nested_ref, lookup)
        .or_else(|| resolve_message_ref(task.message_ref.clone(), lookup))
        .or_else(|| nonempty_attr(task.name.clone()))
        .unwrap_or_else(|| format!("send_{}", task.id))
}

/// Parse a BpmnTimerEventDefinition into a TimerDefinition.
///
/// Priority: timeDuration > timeDate > timeCycle (per BPMN spec, only one should be set).
fn parse_timer_definition(
    node_id: &str,
    timer: &crate::models::BpmnTimerEventDefinition,
) -> EngineResult<TimerDefinition> {
    if let Some(dur_str) = text_value(&timer.time_duration) {
        let dur = parse_iso8601_duration(&dur_str)?;
        return Ok(TimerDefinition::Duration(dur));
    }

    if let Some(date_str) = text_value(&timer.time_date) {
        let dt = date_str.trim().parse::<DateTime<Utc>>().map_err(|e| {
            EngineError::InvalidDefinition(format!("Invalid timeDate '{}': {}", date_str, e))
        })?;
        return Ok(TimerDefinition::AbsoluteDate(dt));
    }

    if let Some(cycle_str) = text_value(&timer.time_cycle) {
        let s = cycle_str.trim();
        // Check for ISO 8601 repeating interval: R[n]/PT...
        if s.starts_with('R') {
            return parse_repeating_interval(s);
        }
        // Otherwise treat as cron expression
        // Validate by parsing
        s.parse::<croner::Cron>().map_err(|e| {
            EngineError::InvalidDefinition(format!("Invalid cron expression '{}': {}", s, e))
        })?;
        return Ok(TimerDefinition::CronCycle {
            expression: s.to_string(),
            max_repetitions: None,
        });
    }

    Err(EngineError::InvalidDefinition(format!(
        "timerEventDefinition on '{}' has no timeDuration, timeDate, or timeCycle",
        node_id
    )))
}

/// Parse ISO 8601 repeating interval: R[n]/PT..., R/PT..., or compact R[n]PT...
fn parse_repeating_interval(s: &str) -> EngineResult<TimerDefinition> {
    // Support both "R3/PT30S" (with slash) and "R3PT30S" (compact, no slash)
    let (r_part, dur_part) = if let Some(slash_pos) = s.find('/') {
        (&s[..slash_pos], &s[slash_pos + 1..])
    } else if let Some(pt_pos) = s[1..].find("PT").map(|p| p + 1) {
        // Compact form: "R3PT30S" → r_part="R3", dur_part="PT30S"
        (&s[..pt_pos], &s[pt_pos..])
    } else {
        return Err(EngineError::InvalidDefinition(format!(
            "Invalid repeating interval '{}': expected R[n]/duration or R[n]PTduration",
            s
        )));
    };

    let repetitions = if r_part == "R" {
        None // infinite
    } else {
        let count_str = &r_part[1..];
        let count: u32 = count_str.parse().map_err(|_| {
            EngineError::InvalidDefinition(format!(
                "Invalid repetition count in '{}': '{}' is not a number",
                s, count_str
            ))
        })?;
        Some(count)
    };

    let interval = parse_iso8601_duration(dur_part)?;
    Ok(TimerDefinition::RepeatingInterval {
        repetitions,
        interval,
    })
}

/// Baut eine Multi-Instance-Definition für Service Tasks.
///
/// Unterstützt werden paralleles und sequentielles MI mit genau einer Quelle:
/// `collection` (Variablenreferenz auf ein Array) oder `loopCardinality`.
fn parse_multi_instance(
    mi: Option<crate::models::BpmnMultiInstanceLoopCharacteristics>,
    element_id: &str,
) -> EngineResult<Option<engine_core::model::MultiInstanceDef>> {
    let Some(mi) = mi else {
        return Ok(None);
    };

    let is_sequential = mi.is_sequential.unwrap_or(false);

    let collection = mi
        .collection
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty());
    let loop_cardinality = mi
        .loop_cardinality
        .and_then(|lc| lc.value)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let element_variable = mi
        .element_variable
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let (collection, loop_cardinality) = match (collection, loop_cardinality) {
        (Some(_), Some(_)) => {
            return Err(EngineError::InvalidDefinition(format!(
                "Multi-instance element '{element_id}' must define either collection or loopCardinality, not both"
            )));
        }
        (None, None) => {
            return Err(EngineError::InvalidDefinition(format!(
                "Multi-instance element '{element_id}' requires either collection or loopCardinality"
            )));
        }
        (Some(collection), None) => (Some(collection), None),
        (None, Some(cardinality)) => {
            let value: u32 = cardinality.parse().map_err(|_| {
                EngineError::InvalidDefinition(format!(
                    "Multi-instance loopCardinality of element '{element_id}' must be a positive integer, got '{cardinality}'"
                ))
            })?;
            if value == 0 {
                return Err(EngineError::InvalidDefinition(format!(
                    "Multi-instance loopCardinality of element '{element_id}' must be a positive integer, got '{cardinality}'"
                )));
            }
            (None, Some(value.to_string()))
        }
    };

    Ok(Some(engine_core::model::MultiInstanceDef {
        is_sequential,
        loop_cardinality,
        collection,
        element_variable,
    }))
}

/// Lehnt `multiInstanceLoopCharacteristics` für Elemente ab, die kein
/// Service Task sind.
fn reject_multi_instance(
    mi: Option<crate::models::BpmnMultiInstanceLoopCharacteristics>,
    element_id: &str,
    element_kind: &str,
) -> EngineResult<()> {
    if mi.is_some() {
        return Err(EngineError::InvalidDefinition(format!(
            "Multi-instance is only supported for service tasks; {element_kind} '{element_id}' is not supported"
        )));
    }
    Ok(())
}

/// Auflösung der Link-Events eines Scopes (Prozess oder Subprozess).
struct LinkScope {
    /// Link-Name → Zielknoten des zugehörigen Catch-Events.
    targets: HashMap<String, String>,
    /// Catch-Events, die nicht materialisiert werden.
    catch_ids: HashSet<String>,
}

/// Baut die Link-Map eines Scopes (`name → catch-Ziel`) und validiert die Catch-Events.
///
/// Link-Events sind scope-lokal: Ein Throw findet nur Catches desselben Prozesses
/// bzw. Subprozesses.
fn resolve_link_scope(
    scope_label: &str,
    catches: &[crate::models::BpmnIntermediateCatchEvent],
    flows: &[crate::models::BpmnSequenceFlow],
) -> EngineResult<LinkScope> {
    let mut scope = LinkScope {
        targets: HashMap::new(),
        catch_ids: HashSet::new(),
    };

    for catch_evt in catches {
        let Some(link) = catch_evt.link_event_definition.as_ref() else {
            continue;
        };
        let name = link.name.trim();
        if name.is_empty() {
            return Err(EngineError::InvalidDefinition(format!(
                "link catch event '{}' in scope '{}' has an empty linkEventDefinition name",
                catch_evt.id, scope_label
            )));
        }
        if scope.targets.contains_key(name) {
            return Err(EngineError::InvalidDefinition(format!(
                "duplicate link name '{}' in scope '{}' (catch event '{}')",
                name, scope_label, catch_evt.id
            )));
        }
        if flows.iter().any(|flow| flow.target_ref == catch_evt.id) {
            return Err(EngineError::InvalidDefinition(format!(
                "link catch event '{}' in scope '{}' must not have incoming sequence flows",
                catch_evt.id, scope_label
            )));
        }
        let mut outgoing = flows
            .iter()
            .filter(|flow| flow.source_ref == catch_evt.id)
            .map(|flow| flow.target_ref.clone());
        let Some(target) = outgoing.next() else {
            return Err(EngineError::InvalidDefinition(format!(
                "link catch event '{}' in scope '{}' has no outgoing sequence flow",
                catch_evt.id, scope_label
            )));
        };
        if outgoing.next().is_some() {
            return Err(EngineError::InvalidDefinition(format!(
                "link catch event '{}' in scope '{}' must have exactly one outgoing sequence flow",
                catch_evt.id, scope_label
            )));
        }
        scope.targets.insert(name.to_string(), target);
        scope.catch_ids.insert(catch_evt.id.clone());
    }

    Ok(scope)
}

/// Liefert das Sprungziel eines Link-Throws und prüft die BPMN-Regeln dafür.
fn resolve_link_throw(
    scope: &LinkScope,
    scope_label: &str,
    throw_id: &str,
    link: &crate::models::BpmnLinkEventDefinition,
    flows: &[crate::models::BpmnSequenceFlow],
) -> EngineResult<String> {
    let name = link.name.trim();
    if name.is_empty() {
        return Err(EngineError::InvalidDefinition(format!(
            "link throw event '{}' in scope '{}' has an empty linkEventDefinition name",
            throw_id, scope_label
        )));
    }
    if flows.iter().any(|flow| flow.source_ref == throw_id) {
        return Err(EngineError::InvalidDefinition(format!(
            "link throw event '{}' in scope '{}' must not have outgoing sequence flows",
            throw_id, scope_label
        )));
    }
    scope.targets.get(name).cloned().ok_or_else(|| {
        EngineError::InvalidDefinition(format!(
            "link throw event '{}' references unknown link name '{}' in scope '{}': no matching link catch event",
            throw_id, name, scope_label
        ))
    })
}

/// Event-Arten, an denen eine Event-Definition hängen kann.
pub(crate) const EVENT_KINDS: [&str; 5] = [
    "boundaryEvent",
    "endEvent",
    "intermediateCatchEvent",
    "intermediateThrowEvent",
    "startEvent",
];

/// Event-Definitionen, die die Engine für die jeweilige Event-Art ausführt.
///
/// Alphabetisch. `docs/knowledge-base/modules/bpmn-parser/element-matrix.md`
/// (Fence `support-index`) muss dieselben Listen führen.
pub(crate) const SUPPORTED_EVENT_DEFINITIONS: &[(&str, &[&str])] = &[
    (
        "boundaryEvent",
        &[
            "compensateEventDefinition",
            "conditionalEventDefinition",
            "errorEventDefinition",
            "escalationEventDefinition",
            "messageEventDefinition",
            "signalEventDefinition",
            "timerEventDefinition",
        ],
    ),
    (
        "endEvent",
        &[
            "compensateEventDefinition",
            "errorEventDefinition",
            "escalationEventDefinition",
            "signalEventDefinition",
            "terminateEventDefinition",
        ],
    ),
    (
        "intermediateCatchEvent",
        &[
            "conditionalEventDefinition",
            "linkEventDefinition",
            "messageEventDefinition",
            "signalEventDefinition",
            "timerEventDefinition",
        ],
    ),
    (
        "intermediateThrowEvent",
        &[
            "compensateEventDefinition",
            "escalationEventDefinition",
            "linkEventDefinition",
            "messageEventDefinition",
            "signalEventDefinition",
        ],
    ),
    (
        "startEvent",
        &[
            "messageEventDefinition",
            "signalEventDefinition",
            "timerEventDefinition",
        ],
    ),
];

/// Elemente, deren Ausführungssemantik fehlt und die der Deploy deshalb ablehnt.
/// `task` und `transaction` haben eine eigene Fehlermeldung; die übrigen
/// nutzen „is not supported“.
pub(crate) const UNSUPPORTED_ELEMENTS: &[&str] = &[
    "adHocSubProcess",
    "standardLoopCharacteristics",
    "task",
    "transaction",
];

/// Erkannte, aber abgelehnte Event-Definitionen (nicht in
/// [`SUPPORTED_EVENT_DEFINITIONS`]). Der Scanner formuliert dafür die
/// bestehende Begründung (Cancel braucht eine Transaction, Conditional-Start
/// hat keinen globalen Variablenraum).
pub(crate) const REJECTED_EVENT_DEFINITIONS: &[(&str, &str)] = &[
    ("boundaryEvent", "cancelEventDefinition"),
    ("endEvent", "cancelEventDefinition"),
    ("startEvent", "conditionalEventDefinition"),
];

/// Attribute, die eine sonst unterstützte Elementart ununterstützt machen.
/// Muss mit `collect_for_element` übereinstimmen.
pub(crate) const REJECTED_ATTRIBUTES: &[&str] = &[
    "eventBasedGateway@eventGatewayType=Parallel",
    "eventBasedGateway@instantiate",
    "receiveTask@instantiate",
    "subProcess@triggeredByEvent",
];

struct XmlFrame {
    name: String,
    id: Option<String>,
    event_defs: usize,
    triggered_by_event: bool,
}

struct XmlScan {
    stack: Vec<XmlFrame>,
    processes: Vec<(String, bool)>,
    violations: Vec<Vec<String>>,
    current: Option<usize>,
}

fn flag_is_true(value: &str) -> bool {
    let value = value.trim();
    value.eq_ignore_ascii_case("true") || value == "1"
}

fn is_parallel_gateway_type(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("parallel")
}

fn line_of(xml: &str, pos: u64) -> usize {
    let end = usize::try_from(pos).unwrap_or(usize::MAX).min(xml.len());
    xml[..end].bytes().filter(|&byte| byte == b'\n').count() + 1
}

fn element_local_name(element: &BytesStart<'_>) -> String {
    element.local_name().as_ref().to_string()
}

fn xml_attr(element: &BytesStart<'_>, name: &str) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attr| attr.key.local_name().as_ref() == name)
        .and_then(|attr| {
            attr.normalized_value(quick_xml::XmlVersion::default())
                .ok()
                .map(|value| value.into_owned())
        })
}

fn nearest_id(own: Option<String>, stack: &[XmlFrame]) -> String {
    if let Some(id) = own.filter(|id| !id.is_empty()) {
        return id;
    }
    stack
        .iter()
        .rev()
        .find_map(|frame| frame.id.clone().filter(|id| !id.is_empty()))
        .unwrap_or_else(|| "?".to_string())
}

fn paren(label: &str, id: &str, line: Option<usize>) -> String {
    match line {
        Some(line) => format!("({label} '{id}', line {line})"),
        None => format!("({label} '{id}')"),
    }
}

fn transaction_violation(id: &str, line: Option<usize>) -> String {
    format!(
        "transaction sub-processes are not supported; cancel events cannot be deployed {}",
        paren("transaction", id, line)
    )
}

fn cancel_violation(kind_label: &str, id: &str, line: Option<usize>) -> String {
    format!(
        "cancel end events require a transaction sub-process; transactions are not supported {}",
        paren(kind_label, id, line)
    )
}

fn conditional_start_violation(id: &str, sub_process: Option<&str>, line: Option<usize>) -> String {
    let location = match sub_process {
        Some(sub_process) => format!("start event '{id}' in subProcess '{sub_process}'"),
        None => format!("start event '{id}'"),
    };
    let where_ = match line {
        Some(line) => format!("({location}, line {line})"),
        None => format!("({location})"),
    };
    format!("conditional start events are not supported; no global variable scope {where_}")
}

fn generic_task_violation(id: &str, line: Option<usize>) -> String {
    let base = format!(
        "unsupported element <task id='{id}'>: convert to userTask, serviceTask, scriptTask, or sendTask"
    );
    match line {
        Some(line) => format!("{base} (line {line})"),
        None => base,
    }
}

fn event_subprocess_violation(id: &str, line: Option<usize>) -> String {
    match line {
        Some(line) => {
            format!("event sub-process '{id}' is not supported (triggeredByEvent, line {line})")
        }
        None => format!("event sub-process '{id}' is not supported (triggeredByEvent)"),
    }
}

fn instantiating_receive_violation(id: &str, line: Option<usize>) -> String {
    match line {
        Some(line) => format!("instantiating receiveTask '{id}' is not supported (line {line})"),
        None => format!("instantiating receiveTask '{id}' is not supported"),
    }
}

fn instantiating_gateway_violation(id: &str, line: Option<usize>) -> String {
    match line {
        Some(line) => format!(
            "instantiating or parallel eventBasedGateway '{id}' is not supported (line {line})"
        ),
        None => format!("instantiating or parallel eventBasedGateway '{id}' is not supported"),
    }
}

fn supported_definitions(kind: &str) -> &'static [&'static str] {
    SUPPORTED_EVENT_DEFINITIONS
        .iter()
        .find(|(name, _)| *name == kind)
        .map(|(_, definitions)| *definitions)
        .unwrap_or(&[])
}

/// Nächster umschließender `subProcess`, der kein Event-Subprozess ist.
fn plain_subprocess_id(stack: &[XmlFrame]) -> Option<String> {
    let frame = stack
        .iter()
        .rev()
        .find(|frame| frame.name == "subProcess")?;
    if frame.triggered_by_event {
        None
    } else {
        Some(frame.id.clone().unwrap_or_else(|| "?".to_string()))
    }
}

fn classify_event_definition(name: &str, line: usize, stack: &mut [XmlFrame]) -> Vec<String> {
    let Some(idx) = stack
        .iter()
        .rposition(|frame| EVENT_KINDS.contains(&frame.name.as_str()))
    else {
        // `messageEventDefinition` an sendTask/receiveTask ist kein Event-Knoten.
        return Vec::new();
    };
    stack[idx].event_defs += 1;
    let count = stack[idx].event_defs;
    let kind = stack[idx].name.clone();
    let event_id = stack[idx].id.clone().unwrap_or_else(|| "?".to_string());
    if count == 2 {
        return vec![format!(
            "multiple event definitions on {kind} '{event_id}' (line {line})"
        )];
    }
    if count > 2 {
        return Vec::new();
    }
    if REJECTED_EVENT_DEFINITIONS
        .iter()
        .any(|(event_kind, definition)| *event_kind == kind && *definition == name)
    {
        if name == "conditionalEventDefinition" {
            let sub_process = plain_subprocess_id(stack);
            return vec![conditional_start_violation(
                &event_id,
                sub_process.as_deref(),
                Some(line),
            )];
        }
        let label = if kind == "endEvent" {
            "end event"
        } else {
            "boundary event"
        };
        return vec![cancel_violation(label, &event_id, Some(line))];
    }
    if supported_definitions(&kind).contains(&name) {
        if kind == "startEvent"
            && let Some(sub_process) = plain_subprocess_id(stack)
        {
            return vec![format!(
                "{name} on startEvent '{event_id}' inside subProcess '{sub_process}' is not executed (embedded sub-process start events are pass-through) (line {line})"
            )];
        }
        return Vec::new();
    }
    vec![format!(
        "{name} on {kind} '{event_id}' is not supported (line {line})"
    )]
}

fn matched_rejected_attribute(name: &str, element: &BytesStart<'_>) -> Option<&'static str> {
    if name == "subProcess"
        && xml_attr(element, "triggeredByEvent").is_some_and(|value| flag_is_true(&value))
    {
        return Some("subProcess@triggeredByEvent");
    }
    if name == "receiveTask"
        && xml_attr(element, "instantiate").is_some_and(|value| flag_is_true(&value))
    {
        return Some("receiveTask@instantiate");
    }
    if name == "eventBasedGateway"
        && xml_attr(element, "instantiate").is_some_and(|value| flag_is_true(&value))
    {
        return Some("eventBasedGateway@instantiate");
    }
    if name == "eventBasedGateway"
        && xml_attr(element, "eventGatewayType")
            .is_some_and(|value| is_parallel_gateway_type(&value))
    {
        return Some("eventBasedGateway@eventGatewayType=Parallel");
    }
    None
}

fn collect_for_element(
    name: &str,
    element: &BytesStart<'_>,
    line: usize,
    stack: &mut [XmlFrame],
) -> Vec<String> {
    let id = nearest_id(xml_attr(element, "id"), stack);
    if let Some(code) = matched_rejected_attribute(name, element)
        && REJECTED_ATTRIBUTES.contains(&code)
    {
        return vec![match code {
            "subProcess@triggeredByEvent" => event_subprocess_violation(&id, Some(line)),
            "receiveTask@instantiate" => instantiating_receive_violation(&id, Some(line)),
            _ => instantiating_gateway_violation(&id, Some(line)),
        }];
    }
    if UNSUPPORTED_ELEMENTS.contains(&name) {
        return vec![match name {
            "transaction" => transaction_violation(&id, Some(line)),
            "task" => generic_task_violation(&id, Some(line)),
            _ => format!("{name} '{id}' is not supported (line {line})"),
        }];
    }
    if name.ends_with("EventDefinition") {
        return classify_event_definition(name, line, stack);
    }
    Vec::new()
}

fn select_process_index(processes: &[(String, bool)]) -> Option<usize> {
    if processes.is_empty() {
        None
    } else {
        Some(
            processes
                .iter()
                .position(|(_, executable)| *executable)
                .unwrap_or(0),
        )
    }
}

fn on_xml_tag(scan: &mut XmlScan, element: &BytesStart<'_>, line: usize, open: bool) {
    let name = element_local_name(element);
    if name == "process" && scan.current.is_none() {
        let id = xml_attr(element, "id").unwrap_or_default();
        let executable = xml_attr(element, "isExecutable").as_deref() == Some("true");
        scan.processes.push((id.clone(), executable));
        scan.violations.push(Vec::new());
        if open {
            scan.current = Some(scan.processes.len() - 1);
            scan.stack.push(XmlFrame {
                name,
                id: Some(id).filter(|value| !value.is_empty()),
                event_defs: 0,
                triggered_by_event: false,
            });
        }
        return;
    }
    if let Some(idx) = scan.current {
        let found = collect_for_element(&name, element, line, &mut scan.stack);
        scan.violations[idx].extend(found);
    }
    if open {
        let triggered_by_event = name == "subProcess"
            && xml_attr(element, "triggeredByEvent").is_some_and(|v| flag_is_true(&v));
        let id = xml_attr(element, "id");
        scan.stack.push(XmlFrame {
            name,
            id,
            event_defs: 0,
            triggered_by_event,
        });
    }
}

fn on_xml_end(scan: &mut XmlScan, name: &str) {
    let frame = scan.stack.pop();
    if name == "process" && frame.is_some_and(|frame| frame.name == "process") {
        scan.current = None;
    }
}

/// Lehnt im deployten Prozess alle Elemente ab, deren Ausführungssemantik fehlt.
///
/// Der Scan läuft vor `serde`, damit unbekannte Tags nicht erst als generischer
/// Parsefehler oder — schlimmer — gar nicht auffallen. Es werden alle Treffer
/// gemeldet, nicht nur der erste. Dieselbe Prozessauswahl wie danach im
/// Deserializer: erster `isExecutable="true"`, sonst der erste Prozess.
/// Visuelle und Datenelemente bleiben erlaubt.
fn reject_unsupported_elements(xml: &str) -> EngineResult<()> {
    let mut reader = Reader::from_str(xml);
    let mut scan = XmlScan {
        stack: Vec::new(),
        processes: Vec::new(),
        violations: Vec::new(),
        current: None,
    };
    loop {
        let event = reader.read_event().map_err(|err| {
            EngineError::InvalidDefinition(format!("BPMN XML parsing failed: {err}"))
        })?;
        let line = line_of(xml, reader.buffer_position());
        match event {
            Event::Start(element) => on_xml_tag(&mut scan, &element, line, true),
            Event::Empty(element) => on_xml_tag(&mut scan, &element, line, false),
            Event::End(element) => {
                let name = element.local_name().as_ref().to_string();
                on_xml_end(&mut scan, &name);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let Some(selected) = select_process_index(&scan.processes) else {
        return Ok(());
    };
    let found = &scan.violations[selected];
    if found.is_empty() {
        Ok(())
    } else {
        Err(EngineError::InvalidDefinition(format!(
            "Unsupported BPMN element(s) in process '{}': {}",
            scan.processes[selected].0,
            found.join("; ")
        )))
    }
}

fn gateway_is_instantiating_or_parallel(gw: &crate::models::BpmnGateway) -> bool {
    gw.instantiate.as_deref().is_some_and(flag_is_true)
        || gw
            .event_gateway_type
            .as_deref()
            .is_some_and(is_parallel_gateway_type)
}

/// Parses a subset of BPMN 2.0 XML and builds a `ProcessDefinition`.
///
/// Note: Since `quick-xml` expects exact structure, the parsed XML must match
/// the structs above (elements rather than attributes where specified, etc.).
pub fn parse_bpmn_xml(xml: &str) -> EngineResult<ProcessDefinition> {
    // Sammelablehnung vor serde, damit ununterstützte Tags nicht still fallen.
    reject_unsupported_elements(xml)?;

    let mut defs: BpmnDefinitions = from_str(xml).map_err(|e| {
        EngineError::InvalidDefinition(format!(
            "BPMN XML parsing failed (hint: ensure the XML contains a valid <process> element and supported entities): {}",
            e
        ))
    })?;

    if defs.processes.is_empty() {
        return Err(EngineError::InvalidDefinition(
            "No <process> element found in BPMN XML".to_string(),
        ));
    }

    // Find the executable process, or fallback to the first one available
    let executable_idx = defs
        .processes
        .iter()
        .position(|p| p.is_executable == Some(true))
        .unwrap_or(0);
    let process = defs.processes.remove(executable_idx);

    let process_id = process.id.clone();

    if let Some(tx) = &process.transaction {
        return Err(EngineError::InvalidDefinition(transaction_violation(
            tx.id.as_deref().unwrap_or(&process_id),
            None,
        )));
    }

    let link_scope = resolve_link_scope(
        &process_id,
        &process.intermediate_catch_events,
        &process.sequence_flows,
    )?;

    let mut builder = ProcessDefinitionBuilder::new(process_id.clone());

    // Lookup maps for messages and errors
    let message_lookup: HashMap<String, String> = defs
        .messages
        .iter()
        .map(|m| (m.id.clone(), m.name.clone()))
        .collect();

    let signal_lookup: HashMap<String, String> = defs
        .signals
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    let error_lookup: HashMap<String, String> = defs
        .errors
        .iter()
        .map(|e| {
            let code = e
                .error_code
                .clone()
                .or_else(|| e.name.clone())
                .unwrap_or_else(|| e.id.clone());
            (e.id.clone(), code)
        })
        .collect();

    let escalation_lookup: HashMap<String, String> = defs
        .escalations
        .iter()
        .map(|e| {
            let code = e
                .escalation_code
                .clone()
                .or_else(|| e.name.clone())
                .unwrap_or_else(|| e.id.clone());
            (e.id.clone(), code)
        })
        .collect();

    // Process-level listeners
    builder = add_listeners(builder, &process_id, process.extension_elements);

    // 1. Process Start Events
    for start in process.start_events {
        let node_id = start.id.clone();
        if start.conditional_event_definition.is_some() {
            return Err(EngineError::InvalidDefinition(conditional_start_violation(
                &node_id, None, None,
            )));
        }
        if let Some(timer) = start.timer_event_definition {
            let timer_def = parse_timer_definition(&start.id, &timer)?;
            builder = builder.node(start.id, BpmnElement::TimerStartEvent(timer_def));
        } else if let Some(msg) = start.message_event_definition {
            let message_name = msg
                .message_ref
                .and_then(|ref_id| message_lookup.get(&ref_id).cloned())
                .unwrap_or_else(|| "generic_message".into());
            builder = builder.node(start.id, BpmnElement::MessageStartEvent { message_name });
        } else if let Some(sig) = start.signal_event_definition {
            let signal_name =
                resolve_signal_ref(sig.signal_ref, &signal_lookup, "startEvent", &start.id)?;
            builder = builder.node(start.id, BpmnElement::SignalStartEvent { signal_name });
        } else {
            builder = builder.node(start.id, BpmnElement::StartEvent);
        }
        builder = add_listeners(builder, &node_id, start.extension_elements);
    }

    // 2. Process End Events
    for end in process.end_events {
        let node_id = end.id.clone();
        if end.cancel_event_definition.is_some() {
            return Err(EngineError::InvalidDefinition(cancel_violation(
                "end event",
                &node_id,
                None,
            )));
        }
        if end.terminate_event_definition.is_some() {
            builder = builder.node(end.id, BpmnElement::TerminateEndEvent);
        } else if let Some(err) = end.error_event_definition {
            let error_code = err
                .error_ref
                .and_then(|ref_id| error_lookup.get(&ref_id).cloned())
                .unwrap_or_else(|| "generic_error".into());
            builder = builder.node(end.id, BpmnElement::ErrorEndEvent { error_code });
        } else if let Some(esc) = end.escalation_event_definition {
            let escalation_code = esc
                .escalation_ref
                .and_then(|ref_id| escalation_lookup.get(&ref_id).cloned())
                .unwrap_or_else(|| "generic_escalation".into());
            builder = builder.node(end.id, BpmnElement::EscalationEndEvent { escalation_code });
        } else if let Some(comp) = end.compensate_event_definition {
            builder = builder.node(
                end.id,
                BpmnElement::CompensationEndEvent {
                    activity_ref: comp.activity_ref,
                },
            );
        } else if let Some(sig) = end.signal_event_definition {
            let signal_name =
                resolve_signal_ref(sig.signal_ref, &signal_lookup, "endEvent", &end.id)?;
            builder = builder.node(end.id, BpmnElement::SignalEndEvent { signal_name });
        } else {
            builder = builder.node(end.id, BpmnElement::EndEvent);
        }
        builder = add_listeners(builder, &node_id, end.extension_elements);
    }

    // 3. Process Service Tasks (All now use external fetching API via topics)
    for task in process.service_tasks {
        let node_id = task.id.clone();
        // Fallback: data-topic, then data-handler, then camunda:topic, then node_id
        let topic = service_task_topic(&task);
        let multi_instance = parse_multi_instance(task.multi_instance, &task.id)?;
        builder = builder.node(
            task.id,
            BpmnElement::ServiceTask {
                topic,
                multi_instance,
            },
        );
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 4. Process User Tasks
    for task in process.user_tasks {
        let node_id = task.id.clone();
        let assignee = user_task_assignee(&task);
        reject_multi_instance(task.multi_instance, &node_id, "userTask")?;
        builder = builder.node(task.id, BpmnElement::UserTask(assignee));
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 5a. Script Tasks — execute inline Rhai script
    for task in process.script_tasks {
        let node_id = task.id.clone();
        // Priority: inline <script> element > data-script attribute
        let script_content = task
            .script
            .and_then(|s| s.content)
            .or(task.data_script)
            .unwrap_or_default();

        reject_multi_instance(task.multi_instance, &task.id, "scriptTask")?;

        if script_content.trim().is_empty() {
            return Err(EngineError::InvalidDefinition(format!(
                "scriptTask '{}' has an empty script",
                node_id
            )));
        }
        builder = builder.node(
            task.id,
            BpmnElement::ScriptTask {
                script: script_content,
                multi_instance: None,
            },
        );
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 5b. Send Tasks — fire-and-forget message publishers
    for task in process.send_tasks {
        let node_id = task.id.clone();
        let message_name = send_task_message_name(&task, &message_lookup);
        reject_multi_instance(task.multi_instance, &task.id, "sendTask")?;
        builder = builder.node(
            task.id,
            BpmnElement::SendTask {
                message_name,
                multi_instance: None,
            },
        );
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 5c. receiveTask waits for a message (same runtime as MessageCatchEvent)
    for task in process.receive_tasks {
        let node_id = task.id.clone();
        if task.instantiate.as_deref().is_some_and(flag_is_true) {
            return Err(EngineError::InvalidDefinition(
                instantiating_receive_violation(&node_id, None),
            ));
        }
        reject_multi_instance(task.multi_instance, &node_id, "receiveTask")?;
        let msg_ref = task.message_ref.clone().or_else(|| {
            task.message_event_definition
                .as_ref()
                .and_then(|m| m.message_ref.clone())
        });
        let Some(message_name) = resolve_message_ref(msg_ref, &message_lookup) else {
            return Err(EngineError::InvalidDefinition(format!(
                "receiveTask '{}' has no messageRef",
                node_id
            )));
        };
        builder = builder.node(task.id, BpmnElement::MessageCatchEvent { message_name });
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 5d. manualTask is human work without a form — same wait as UserTask
    for task in process.manual_tasks {
        let node_id = task.id.clone();
        let assignee = user_task_assignee(&task);
        reject_multi_instance(task.multi_instance, &node_id, "manualTask")?;
        builder = builder.node(task.id, BpmnElement::UserTask(assignee));
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 5e. businessRuleTask: external topic only (no DMN). Silent queue mapping is rejected.
    for task in process.business_rule_tasks {
        let node_id = task.id.clone();
        if first_attr([
            task.topic.clone(),
            task.handler.clone(),
            task.camunda_topic.clone(),
        ])
        .is_none()
        {
            return Err(EngineError::InvalidDefinition(format!(
                "businessRuleTask '{}' has no topic (data-topic, data-handler, or camunda:topic); DMN is not supported",
                node_id
            )));
        }
        let topic = service_task_topic(&task);
        reject_multi_instance(task.multi_instance, &task.id, "businessRuleTask")?;
        builder = builder.node(
            task.id,
            BpmnElement::ServiceTask {
                topic,
                multi_instance: None,
            },
        );
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 5f. Generic <task> from bpmn-js must be converted to a concrete type
    if let Some(task) = process.generic_tasks.first() {
        return Err(EngineError::InvalidDefinition(generic_task_violation(
            &task.id, None,
        )));
    }

    // 5d. Call Activities — mapped to BpmnElement::CallActivity with calledElement
    for task in process.call_activities {
        let node_id = task.id.clone();
        let called_element = task.called_element.unwrap_or_else(|| task.id.clone());
        builder = builder.node(task.id, BpmnElement::CallActivity { called_element });
        builder = add_listeners(builder, &node_id, task.extension_elements);
    }

    // 6. Build a flow lookup (flow-ID → target-ref) for resolving the
    //    `default` attribute on exclusive gateways.
    let flow_lookup: HashMap<String, String> = process
        .sequence_flows
        .iter()
        .map(|f| (f._id.clone(), f.target_ref.clone()))
        .collect();

    // 6a. Exclusive gateways — resolve `default` flow ID → target node ID
    for gw in process.exclusive_gateways {
        let node_id = gw.id.clone();
        let default_target = gw
            .default
            .and_then(|flow_id| flow_lookup.get(&flow_id).cloned());
        builder = builder.node(
            gw.id,
            BpmnElement::ExclusiveGateway {
                default: default_target,
            },
        );
        builder = add_listeners(builder, &node_id, gw.extension_elements);
    }

    // 6b. Inclusive gateways
    for gw in process.inclusive_gateways {
        let node_id = gw.id.clone();
        builder = builder.node(gw.id, BpmnElement::InclusiveGateway);
        builder = add_listeners(builder, &node_id, gw.extension_elements);
    }

    // 6c. Parallel gateways
    for gw in process.parallel_gateways {
        let node_id = gw.id.clone();
        builder = builder.node(gw.id, BpmnElement::ParallelGateway);
        builder = add_listeners(builder, &node_id, gw.extension_elements);
    }

    // 6d. Event-based gateways
    for gw in process.event_based_gateways {
        let node_id = gw.id.clone();
        if gateway_is_instantiating_or_parallel(&gw) {
            return Err(EngineError::InvalidDefinition(
                instantiating_gateway_violation(&node_id, None),
            ));
        }
        builder = builder.node(gw.id, BpmnElement::EventBasedGateway);
        builder = add_listeners(builder, &node_id, gw.extension_elements);
    }

    // 6e. Complex gateways
    for gw in process.complex_gateways {
        let node_id = gw.id.clone();
        let default_target = gw
            .default
            .and_then(|flow_id| flow_lookup.get(&flow_id).cloned());
        let join_condition = gw.activation_condition.map(|c| c.value.trim().to_string());
        builder = builder.node(
            gw.id,
            BpmnElement::ComplexGateway {
                default: default_target,
                join_condition,
            },
        );
        builder = add_listeners(builder, &node_id, gw.extension_elements);
    }

    // 7. Intermediate catch events
    for catch_evt in process.intermediate_catch_events {
        let node_id = catch_evt.id.clone();
        if catch_evt.link_event_definition.is_some() {
            // Link-Catches werden nicht materialisiert: der Throw springt auf ihr Ziel.
            continue;
        }
        if let Some(timer) = catch_evt.timer_event_definition {
            let timer_def = parse_timer_definition(&catch_evt.id, &timer)?;
            builder = builder.node(catch_evt.id, BpmnElement::TimerCatchEvent(timer_def));
        } else if let Some(msg) = catch_evt.message_event_definition {
            let message_name = resolve_message_ref(msg.message_ref, &message_lookup)
                .unwrap_or_else(|| "generic_message".into());
            builder = builder.node(
                catch_evt.id,
                BpmnElement::MessageCatchEvent { message_name },
            );
        } else if let Some(sig) = catch_evt.signal_event_definition {
            let signal_name = resolve_signal_ref(
                sig.signal_ref,
                &signal_lookup,
                "intermediateCatchEvent",
                &catch_evt.id,
            )?;
            builder = builder.node(catch_evt.id, BpmnElement::SignalCatchEvent { signal_name });
        } else if let Some(cond) = catch_evt.conditional_event_definition {
            let condition =
                parse_conditional_condition(&cond, "intermediateCatchEvent", &catch_evt.id)?;
            builder = builder.node(
                catch_evt.id,
                BpmnElement::ConditionalCatchEvent { condition },
            );
        } else {
            return Err(EngineError::InvalidDefinition(format!(
                "intermediateCatchEvent '{}' has no timerEventDefinition, messageEventDefinition, signalEventDefinition, or conditionalEventDefinition",
                node_id
            )));
        }
        builder = add_listeners(builder, &node_id, catch_evt.extension_elements);
    }

    // 8. Intermediate throw events
    for evt in process.intermediate_throw_events {
        let node_id = evt.id.clone();
        if let Some(link) = evt.link_event_definition {
            let target = resolve_link_throw(
                &link_scope,
                &process_id,
                &node_id,
                &link,
                &process.sequence_flows,
            )?;
            builder = builder.node(
                evt.id,
                BpmnElement::LinkThrowEvent {
                    target: target.clone(),
                },
            );
            // Synthetische Kante für die Graph-Validierung; der Handler springt direkt.
            builder = builder.flow(node_id.clone(), target);
            builder = add_listeners(builder, &node_id, evt.extension_elements);
            continue;
        }
        if let Some(msg) = evt.message_event_definition {
            let message_name = msg
                .message_ref
                .and_then(|ref_id| message_lookup.get(&ref_id).cloned())
                .unwrap_or_else(|| "generic_throw".into());
            builder = builder.node(
                evt.id,
                BpmnElement::SendTask {
                    message_name,
                    multi_instance: None,
                },
            );
        } else if let Some(sig) = evt.signal_event_definition {
            let signal_name = resolve_signal_ref(
                sig.signal_ref,
                &signal_lookup,
                "intermediateThrowEvent",
                &evt.id,
            )?;
            builder = builder.node(evt.id, BpmnElement::SignalThrowEvent { signal_name });
        } else if let Some(esc) = evt.escalation_event_definition {
            let escalation_code = esc
                .escalation_ref
                .and_then(|ref_id| escalation_lookup.get(&ref_id).cloned())
                .unwrap_or_else(|| "generic_escalation".into());
            builder = builder.node(
                evt.id,
                BpmnElement::EscalationThrowEvent { escalation_code },
            );
        } else if let Some(comp) = evt.compensate_event_definition {
            builder = builder.node(
                evt.id,
                BpmnElement::CompensationThrowEvent {
                    activity_ref: comp.activity_ref,
                },
            );
        } else {
            return Err(EngineError::InvalidDefinition(format!(
                "intermediateThrowEvent '{}' has no message, signal, escalation, or compensate definition",
                node_id
            )));
        }
        builder = add_listeners(builder, &node_id, evt.extension_elements);
    }

    // 9. Boundary Events
    for bd in process.boundary_events {
        let node_id = bd.id.clone();
        let attached_to = bd.attached_to_ref.clone();
        // cancelActivity is true by default
        let cancel_activity = bd.cancel_activity.unwrap_or(true);

        if bd.cancel_event_definition.is_some() {
            return Err(EngineError::InvalidDefinition(cancel_violation(
                "boundary event",
                &node_id,
                None,
            )));
        }
        if let Some(timer) = bd.timer_event_definition {
            let timer_def = parse_timer_definition(&bd.id, &timer)?;
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryTimerEvent {
                    attached_to,
                    timer: timer_def,
                    cancel_activity,
                },
            );
        } else if let Some(msg) = bd.message_event_definition {
            let message_name = msg
                .message_ref
                .map(|r| message_lookup.get(&r).cloned().unwrap_or(r))
                .unwrap_or_else(|| "unknown".into());
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryMessageEvent {
                    attached_to,
                    message_name,
                    cancel_activity,
                },
            );
        } else if let Some(sig) = bd.signal_event_definition {
            let signal_name =
                resolve_signal_ref(sig.signal_ref, &signal_lookup, "boundaryEvent", &bd.id)?;
            builder = builder.node(
                bd.id,
                BpmnElement::BoundarySignalEvent {
                    attached_to,
                    signal_name,
                    cancel_activity,
                },
            );
        } else if let Some(cond) = bd.conditional_event_definition {
            let condition = parse_conditional_condition(&cond, "boundaryEvent", &bd.id)?;
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryConditionalEvent {
                    attached_to,
                    condition,
                    cancel_activity,
                },
            );
        } else if let Some(err) = bd.error_event_definition {
            let error_code = err
                .error_ref
                .and_then(|ref_id| error_lookup.get(&ref_id).cloned());
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryErrorEvent {
                    attached_to,
                    error_code,
                },
            );
        } else if let Some(esc) = bd.escalation_event_definition {
            let escalation_code = esc
                .escalation_ref
                .and_then(|ref_id| escalation_lookup.get(&ref_id).cloned());
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryEscalationEvent {
                    attached_to,
                    escalation_code,
                    cancel_activity,
                },
            );
        } else if bd.compensate_event_definition.is_some() {
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryCompensationEvent { attached_to },
            );
        } else {
            return Err(EngineError::InvalidDefinition(format!(
                "boundaryEvent '{}' has no timer, message, signal, conditional, error, escalation, or compensate definition",
                node_id
            )));
        }
        builder = add_listeners(builder, &node_id, None);
    }

    // 10. Process Sequence Flows
    for flow in process.sequence_flows {
        if link_scope.catch_ids.contains(&flow.source_ref) {
            // Der Catch existiert nicht; sein Ziel wird über den Throw erreicht.
            continue;
        }
        if let Some(cond) = flow.condition_expression {
            builder = builder.conditional_flow(flow.source_ref, flow.target_ref, cond.value.trim());
        } else {
            builder = builder.flow(flow.source_ref, flow.target_ref);
        }
    }

    // 11. Flatten nested sub-processes (embedded scopes)
    for sp in process.sub_processes {
        builder = flatten_subprocess(
            sp,
            &process_id,
            builder,
            &message_lookup,
            &signal_lookup,
            &error_lookup,
            &escalation_lookup,
        )?;
    }

    let mut definition = builder.build()?;
    crate::data::apply(xml, &mut definition)?;
    Ok(definition)
}

fn resolve_sub_process_default(
    flow_lookup: &HashMap<String, String>,
    sub_process_id: &str,
    gateway_id: &str,
    default: Option<String>,
) -> EngineResult<Option<String>> {
    default
        .map(|flow_id| {
            flow_lookup.get(&flow_id).cloned().ok_or_else(|| {
                EngineError::InvalidDefinition(format!(
                    "gateway '{gateway_id}' in subProcess '{sub_process_id}' references unknown default flow '{flow_id}'"
                ))
            })
        })
        .transpose()
}

#[allow(clippy::only_used_in_recursion)]
fn flatten_subprocess(
    sp: crate::models::BpmnSubProcess,
    _parent_id: &str,
    mut builder: ProcessDefinitionBuilder,
    message_lookup: &HashMap<String, String>,
    signal_lookup: &HashMap<String, String>,
    error_lookup: &HashMap<String, String>,
    escalation_lookup: &HashMap<String, String>,
) -> EngineResult<ProcessDefinitionBuilder> {
    let sub_process_id = sp.id.clone();

    if sp.triggered_by_event == Some(true) {
        return Err(EngineError::InvalidDefinition(event_subprocess_violation(
            &sub_process_id,
            None,
        )));
    }

    if let Some(tx) = &sp.transaction {
        return Err(EngineError::InvalidDefinition(transaction_violation(
            tx.id.as_deref().unwrap_or(&sub_process_id),
            None,
        )));
    }

    let link_scope = resolve_link_scope(
        &sub_process_id,
        &sp.intermediate_catch_events,
        &sp.sequence_flows,
    )?;

    let start_event = sp.start_events.first().ok_or_else(|| {
        EngineError::InvalidDefinition(format!("subProcess '{sub_process_id}' has no startEvent"))
    })?;
    if sp.start_events.len() > 1 {
        return Err(EngineError::InvalidDefinition(format!(
            "subProcess '{sub_process_id}' must have exactly one startEvent"
        )));
    }
    let start_event_id = start_event.id.clone();
    if start_event.conditional_event_definition.is_some() {
        return Err(EngineError::InvalidDefinition(conditional_start_violation(
            &start_event_id,
            Some(&sub_process_id),
            None,
        )));
    }
    let mut start_targets: Vec<String> = sp
        .sequence_flows
        .iter()
        .filter(|flow| flow.source_ref == start_event_id)
        .map(|flow| flow.target_ref.clone())
        .collect();

    // Internal start events are pass-throughs: the EmbeddedSubProcess node continues at
    // the start event's target, so no node is materialised for it. Several outgoing
    // flows are all taken (BPMN implicit split) and stay in the graph as a parallel gateway.
    // Event-Definitionen am internen Start lehnt `reject_unsupported_elements` ab.
    let (start_node_id, start_forks) = match start_targets.len() {
        0 => {
            return Err(EngineError::InvalidDefinition(format!(
                "startEvent '{start_event_id}' in subProcess '{sub_process_id}' has no outgoing sequence flow"
            )));
        }
        1 => (start_targets.remove(0), false),
        _ => (start_event_id.clone(), true),
    };

    builder = builder.node(
        sub_process_id.clone(),
        BpmnElement::EmbeddedSubProcess { start_node_id },
    );
    if start_forks {
        builder = builder.node(start_event_id.clone(), BpmnElement::ParallelGateway);
    }

    for end in sp.end_events {
        // End-Events beenden den eingebetteten Scope. Signal-End-Events werden wie die
        // uebrigen End-Event-Definitionen geflattet und senden kein Signal.
        if end.cancel_event_definition.is_some() {
            return Err(EngineError::InvalidDefinition(cancel_violation(
                "end event",
                &end.id,
                None,
            )));
        }
        builder = builder.node(
            end.id,
            BpmnElement::SubProcessEndEvent {
                sub_process_id: sub_process_id.clone(),
            },
        );
    }

    for task in sp.service_tasks {
        let topic = service_task_topic(&task);
        let multi_instance = parse_multi_instance(task.multi_instance, &task.id)?;
        builder = builder.node(
            task.id,
            BpmnElement::ServiceTask {
                topic,
                multi_instance,
            },
        );
    }

    for task in sp.script_tasks {
        let script = task
            .script
            .and_then(|s| s.content)
            .or(task.data_script)
            .unwrap_or_default();
        if script.trim().is_empty() {
            return Err(EngineError::InvalidDefinition(format!(
                "scriptTask '{}' has an empty script",
                task.id
            )));
        }
        reject_multi_instance(task.multi_instance, &task.id, "scriptTask")?;
        builder = builder.node(
            task.id,
            BpmnElement::ScriptTask {
                script,
                multi_instance: None,
            },
        );
    }

    for task in sp.user_tasks {
        let assignee = user_task_assignee(&task);
        reject_multi_instance(task.multi_instance, &task.id, "userTask")?;
        builder = builder.node(task.id, BpmnElement::UserTask(assignee));
    }

    for task in sp.send_tasks {
        let message_name = send_task_message_name(&task, message_lookup);
        reject_multi_instance(task.multi_instance, &task.id, "sendTask")?;
        builder = builder.node(
            task.id,
            BpmnElement::SendTask {
                message_name,
                multi_instance: None,
            },
        );
    }

    for task in sp.receive_tasks {
        if task.instantiate.as_deref().is_some_and(flag_is_true) {
            return Err(EngineError::InvalidDefinition(
                instantiating_receive_violation(&task.id, None),
            ));
        }
        reject_multi_instance(task.multi_instance, &task.id, "receiveTask")?;
        let msg_ref = task.message_ref.clone().or_else(|| {
            task.message_event_definition
                .as_ref()
                .and_then(|m| m.message_ref.clone())
        });
        let Some(message_name) = resolve_message_ref(msg_ref, message_lookup) else {
            return Err(EngineError::InvalidDefinition(format!(
                "receiveTask '{}' has no messageRef",
                task.id
            )));
        };
        builder = builder.node(task.id, BpmnElement::MessageCatchEvent { message_name });
    }

    for task in sp.manual_tasks {
        let assignee = user_task_assignee(&task);
        reject_multi_instance(task.multi_instance, &task.id, "manualTask")?;
        builder = builder.node(task.id, BpmnElement::UserTask(assignee));
    }

    for task in sp.business_rule_tasks {
        if first_attr([
            task.topic.clone(),
            task.handler.clone(),
            task.camunda_topic.clone(),
        ])
        .is_none()
        {
            return Err(EngineError::InvalidDefinition(format!(
                "businessRuleTask '{}' has no topic (data-topic, data-handler, or camunda:topic); DMN is not supported",
                task.id
            )));
        }
        let topic = service_task_topic(&task);
        reject_multi_instance(task.multi_instance, &task.id, "businessRuleTask")?;
        builder = builder.node(
            task.id,
            BpmnElement::ServiceTask {
                topic,
                multi_instance: None,
            },
        );
    }

    if let Some(task) = sp.generic_tasks.first() {
        return Err(EngineError::InvalidDefinition(generic_task_violation(
            &task.id, None,
        )));
    }

    for task in sp.call_activities {
        let called_element = task.called_element.unwrap_or_else(|| task.id.clone());
        builder = builder.node(task.id, BpmnElement::CallActivity { called_element });
    }

    // Default-Flows werden wie am Top-Level von der Flow-ID auf den Zielknoten abgebildet.
    let flow_lookup: HashMap<String, String> = sp
        .sequence_flows
        .iter()
        .map(|f| (f._id.clone(), f.target_ref.clone()))
        .collect();

    for gw in sp.exclusive_gateways {
        let default_target =
            resolve_sub_process_default(&flow_lookup, &sub_process_id, &gw.id, gw.default)?;
        builder = builder.node(
            gw.id,
            BpmnElement::ExclusiveGateway {
                default: default_target,
            },
        );
    }
    for gw in sp.parallel_gateways {
        builder = builder.node(gw.id, BpmnElement::ParallelGateway);
    }
    for gw in sp.inclusive_gateways {
        builder = builder.node(gw.id, BpmnElement::InclusiveGateway);
    }
    for gw in sp.event_based_gateways {
        if gateway_is_instantiating_or_parallel(&gw) {
            return Err(EngineError::InvalidDefinition(
                instantiating_gateway_violation(&gw.id, None),
            ));
        }
        builder = builder.node(gw.id, BpmnElement::EventBasedGateway);
    }

    for gw in sp.complex_gateways {
        let default_target =
            resolve_sub_process_default(&flow_lookup, &sub_process_id, &gw.id, gw.default)?;
        let join_condition = gw.activation_condition.map(|c| c.value.trim().to_string());
        builder = builder.node(
            gw.id,
            BpmnElement::ComplexGateway {
                default: default_target,
                join_condition,
            },
        );
    }

    for catch_evt in sp.intermediate_catch_events {
        if catch_evt.link_event_definition.is_some() {
            // Link-Catches werden nicht materialisiert: der Throw springt auf ihr Ziel.
            continue;
        }
        if let Some(timer) = catch_evt.timer_event_definition {
            let timer_def = parse_timer_definition(&catch_evt.id, &timer)?;
            builder = builder.node(catch_evt.id, BpmnElement::TimerCatchEvent(timer_def));
        } else if let Some(msg) = catch_evt.message_event_definition {
            let message_name = resolve_message_ref(msg.message_ref, message_lookup)
                .unwrap_or_else(|| "generic_message".into());
            builder = builder.node(
                catch_evt.id,
                BpmnElement::MessageCatchEvent { message_name },
            );
        } else if let Some(sig) = catch_evt.signal_event_definition {
            let signal_name = resolve_signal_ref(
                sig.signal_ref,
                signal_lookup,
                "intermediateCatchEvent",
                &catch_evt.id,
            )?;
            builder = builder.node(catch_evt.id, BpmnElement::SignalCatchEvent { signal_name });
        } else if let Some(cond) = catch_evt.conditional_event_definition {
            let condition =
                parse_conditional_condition(&cond, "intermediateCatchEvent", &catch_evt.id)?;
            builder = builder.node(
                catch_evt.id,
                BpmnElement::ConditionalCatchEvent { condition },
            );
        } else {
            return Err(EngineError::InvalidDefinition(format!(
                "intermediateCatchEvent '{}' has no timerEventDefinition, messageEventDefinition, signalEventDefinition, or conditionalEventDefinition",
                catch_evt.id
            )));
        }
    }

    for evt in sp.intermediate_throw_events {
        if let Some(link) = evt.link_event_definition {
            let target = resolve_link_throw(
                &link_scope,
                &sub_process_id,
                &evt.id,
                &link,
                &sp.sequence_flows,
            )?;
            let throw_id = evt.id.clone();
            builder = builder.node(
                throw_id.clone(),
                BpmnElement::LinkThrowEvent {
                    target: target.clone(),
                },
            );
            // Synthetische Kante für die Graph-Validierung; der Handler springt direkt.
            builder = builder.flow(throw_id, target);
            continue;
        }
        if let Some(msg) = evt.message_event_definition {
            let message_name = resolve_message_ref(msg.message_ref, message_lookup)
                .unwrap_or_else(|| "generic_throw".into());
            builder = builder.node(
                evt.id,
                BpmnElement::SendTask {
                    message_name,
                    multi_instance: None,
                },
            );
        } else if let Some(sig) = evt.signal_event_definition {
            let signal_name = resolve_signal_ref(
                sig.signal_ref,
                signal_lookup,
                "intermediateThrowEvent",
                &evt.id,
            )?;
            builder = builder.node(evt.id, BpmnElement::SignalThrowEvent { signal_name });
        } else if let Some(esc) = evt.escalation_event_definition {
            let escalation_code = esc
                .escalation_ref
                .and_then(|ref_id| escalation_lookup.get(&ref_id).cloned())
                .unwrap_or_else(|| "generic_escalation".into());
            builder = builder.node(
                evt.id,
                BpmnElement::EscalationThrowEvent { escalation_code },
            );
        } else if let Some(comp) = evt.compensate_event_definition {
            builder = builder.node(
                evt.id,
                BpmnElement::CompensationThrowEvent {
                    activity_ref: comp.activity_ref,
                },
            );
        } else {
            return Err(EngineError::InvalidDefinition(format!(
                "intermediateThrowEvent '{}' has no message, signal, escalation, or compensate definition",
                evt.id
            )));
        }
    }

    for bd in sp.boundary_events {
        let attached_to = bd.attached_to_ref.clone();
        let cancel_activity = bd.cancel_activity.unwrap_or(true);

        if bd.cancel_event_definition.is_some() {
            return Err(EngineError::InvalidDefinition(cancel_violation(
                "boundary event",
                &bd.id,
                None,
            )));
        }
        if let Some(timer) = bd.timer_event_definition {
            let timer_def = parse_timer_definition(&bd.id, &timer)?;
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryTimerEvent {
                    attached_to,
                    timer: timer_def,
                    cancel_activity,
                },
            );
        } else if let Some(msg) = bd.message_event_definition {
            let message_name = resolve_message_ref(msg.message_ref, message_lookup)
                .unwrap_or_else(|| "unknown".into());
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryMessageEvent {
                    attached_to,
                    message_name,
                    cancel_activity,
                },
            );
        } else if let Some(sig) = bd.signal_event_definition {
            let signal_name =
                resolve_signal_ref(sig.signal_ref, signal_lookup, "boundaryEvent", &bd.id)?;
            builder = builder.node(
                bd.id,
                BpmnElement::BoundarySignalEvent {
                    attached_to,
                    signal_name,
                    cancel_activity,
                },
            );
        } else if let Some(cond) = bd.conditional_event_definition {
            let condition = parse_conditional_condition(&cond, "boundaryEvent", &bd.id)?;
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryConditionalEvent {
                    attached_to,
                    condition,
                    cancel_activity,
                },
            );
        } else if let Some(err) = bd.error_event_definition {
            let error_code = err
                .error_ref
                .and_then(|ref_id| error_lookup.get(&ref_id).cloned());
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryErrorEvent {
                    attached_to,
                    error_code,
                },
            );
        } else if let Some(esc) = bd.escalation_event_definition {
            let escalation_code = esc
                .escalation_ref
                .and_then(|ref_id| escalation_lookup.get(&ref_id).cloned());
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryEscalationEvent {
                    attached_to,
                    escalation_code,
                    cancel_activity,
                },
            );
        } else if bd.compensate_event_definition.is_some() {
            builder = builder.node(
                bd.id,
                BpmnElement::BoundaryCompensationEvent { attached_to },
            );
        } else {
            return Err(EngineError::InvalidDefinition(format!(
                "boundaryEvent '{}' has no timer, message, signal, conditional, error, escalation, or compensate definition",
                bd.id
            )));
        }
    }

    for flow in sp.sequence_flows {
        if flow.source_ref == start_event_id && !start_forks {
            // Flows out of the internal start event define the sub-process entry point only.
            continue;
        }
        if link_scope.catch_ids.contains(&flow.source_ref) {
            // Der Catch existiert nicht; sein Ziel wird über den Throw erreicht.
            continue;
        }
        if let Some(cond) = flow.condition_expression {
            builder = builder.conditional_flow(
                flow.source_ref,
                flow.target_ref,
                cond.value.trim().to_string(),
            );
        } else {
            builder = builder.flow(flow.source_ref, flow.target_ref);
        }
    }

    for nested_sp in sp.sub_processes {
        builder = flatten_subprocess(
            nested_sp,
            &sub_process_id,
            builder,
            message_lookup,
            signal_lookup,
            error_lookup,
            escalation_lookup,
        )?;
    }

    Ok(builder)
}
