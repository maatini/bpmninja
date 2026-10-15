use crate::data::{Node, tree};
use engine_core::domain::{EventSubProcess, EventSubProcessTrigger, ProcessDefinition};
use engine_core::error::{EngineError, EngineResult};

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidDefinition(format!("Event subprocess: {}", message.into()))
}
fn event_subprocess(node: &Node) -> bool {
    node.name == "subProcess" && matches!(node.attr("triggeredByEvent"), Some("true" | "1"))
}
fn reject_nested(node: &Node) -> EngineResult<()> {
    for child in &node.children {
        if event_subprocess(child) {
            return Err(invalid("nested event subprocesses require X6"));
        }
        reject_nested(child)?;
    }
    Ok(())
}
fn serialize(node: &Node) -> String {
    if node.name.is_empty() {
        return node.children.iter().map(serialize).collect();
    }
    let attributes: String = node
        .attrs
        .iter()
        .map(|(name, value)| format!(" {name}=\"{}\"", quick_xml::escape::escape(value)))
        .collect();
    // Existing parser machinery uses local element names. Keep qualified attributes.
    format!(
        "<{}{attributes}>{}{}</{}>",
        node.name,
        quick_xml::escape::escape(&node.text),
        node.children.iter().map(serialize).collect::<String>(),
        node.name
    )
}
fn referenced(
    definitions: &Node,
    event: &Node,
    kind: &str,
    attribute: &str,
    code: bool,
) -> EngineResult<String> {
    let id = event
        .attr(attribute)
        .ok_or_else(|| invalid(format!("{kind} start requires {attribute}")))?;
    let declaration = definitions
        .children(kind)
        .find(|d| d.attr("id") == Some(id))
        .ok_or_else(|| invalid(format!("unknown {attribute} '{id}'")))?;
    Ok(if code {
        declaration
            .attr(if kind == "error" {
                "errorCode"
            } else {
                "escalationCode"
            })
            .or(declaration.attr("name"))
            .unwrap_or(id)
    } else {
        declaration.attr("name").unwrap_or(id)
    }
    .into())
}

pub(crate) struct ParsedHandler {
    handler: EventSubProcess,
    body: ProcessDefinition,
}

pub(crate) fn extract(xml: &str, process_id: &str) -> EngineResult<(String, Vec<ParsedHandler>)> {
    let mut document = tree(xml)?;
    let definitions = document
        .children
        .first()
        .ok_or_else(|| invalid("missing definitions"))?
        .clone();
    let process = definitions
        .children("process")
        .find(|p| p.attr("id") == Some(process_id))
        .ok_or_else(|| invalid("missing process"))?;
    let mut handlers = Vec::new();
    for node in &process.children {
        if !event_subprocess(node) {
            reject_nested(node)?;
            continue;
        }
        reject_nested(node)?;
        let id = node.id()?.to_string();
        if node.children("extensionElements").any(|extension| {
            extension
                .children
                .iter()
                .any(|child| child.name == "executionListener")
        }) {
            return Err(invalid(
                "handler-container execution listeners are not supported; attach listeners to handler body nodes",
            ));
        }
        if node.children.iter().any(|n| {
            matches!(
                n.name.as_str(),
                "property"
                    | "dataObject"
                    | "ioSpecification"
                    | "multiInstanceLoopCharacteristics"
                    | "standardLoopCharacteristics"
            )
        }) || node.children("extensionElements").any(|e| {
            e.children
                .iter()
                .any(|n| matches!(n.name.as_str(), "inputOutput" | "in" | "out"))
        }) {
            return Err(invalid(
                "handler data scopes and loop characteristics require X6",
            ));
        }
        if process.children("sequenceFlow").any(|flow| {
            flow.attr("sourceRef") == Some(id.as_str())
                || flow.attr("targetRef") == Some(id.as_str())
        }) {
            return Err(invalid(format!(
                "'{id}' cannot have incoming/outgoing sequence flows"
            )));
        }
        let starts: Vec<_> = node.children("startEvent").collect();
        let [start] = starts.as_slice() else {
            return Err(invalid(format!("'{id}' requires exactly one start event")));
        };
        let is_interrupting = match start.attr("isInterrupting") {
            None | Some("true" | "1") => true,
            Some("false" | "0") => false,
            Some(_) => return Err(invalid("invalid isInterrupting boolean")),
        };
        let events: Vec<_> = start
            .children
            .iter()
            .filter(|n| n.name.ends_with("EventDefinition"))
            .collect();
        let [event] = events.as_slice() else {
            return Err(invalid(
                "start requires exactly one supported event definition",
            ));
        };
        let trigger = match event.name.as_str() {
            "timerEventDefinition" => {
                let timer: crate::models::BpmnTimerEventDefinition =
                    quick_xml::de::from_str(&serialize(event))
                        .map_err(|e| invalid(e.to_string()))?;
                EventSubProcessTrigger::Timer(crate::parser::parse_timer_definition(&id, &timer)?)
            }
            "messageEventDefinition" => EventSubProcessTrigger::Message(referenced(
                &definitions,
                event,
                "message",
                "messageRef",
                false,
            )?),
            "signalEventDefinition" => EventSubProcessTrigger::Signal(referenced(
                &definitions,
                event,
                "signal",
                "signalRef",
                false,
            )?),
            "errorEventDefinition" => {
                if !is_interrupting {
                    return Err(invalid("error starts must be interrupting"));
                }
                EventSubProcessTrigger::Error(
                    event
                        .attr("errorRef")
                        .map(|_| referenced(&definitions, event, "error", "errorRef", true))
                        .transpose()?,
                )
            }
            "escalationEventDefinition" => EventSubProcessTrigger::Escalation(
                event
                    .attr("escalationRef")
                    .map(|_| referenced(&definitions, event, "escalation", "escalationRef", true))
                    .transpose()?,
            ),
            other => return Err(invalid(format!("'{other}' start is not supported in X9a"))),
        };
        let mut body_node = node.clone();
        body_node.name = "process".into();
        body_node.attrs.remove("triggeredByEvent");
        body_node.attrs.insert("isExecutable".into(), "true".into());
        for start in body_node
            .children
            .iter_mut()
            .filter(|n| n.name == "startEvent")
        {
            start.attrs.remove("isInterrupting");
            start
                .children
                .retain(|n| !n.name.ends_with("EventDefinition"));
        }
        // Task associations in the handler can reference the containing process's data.
        body_node.children.extend(
            process
                .children
                .iter()
                .filter(|n| {
                    matches!(
                        n.name.as_str(),
                        "property" | "dataObject" | "dataObjectReference"
                    )
                })
                .cloned(),
        );
        let mut body_document = definitions.clone();
        body_document.children.retain(|n| {
            n.name != "process" && n.name != "BPMNDiagram" && n.name != "collaboration"
        });
        body_document.attrs.insert(
            "xmlns:bpmninja".into(),
            "https://bpmninja.dev/schema/bpmn".into(),
        );
        body_document.children.push(body_node);
        let body = crate::parse_bpmn_xml(&serialize(&body_document))?;
        handlers.push(ParsedHandler {
            handler: EventSubProcess {
                id,
                start_node: start.id()?.into(),
                node_ids: body.nodes.keys().cloned().collect(),
                trigger,
                is_interrupting,
            },
            body,
        });
    }
    // Avoid rewriting XML when no event subprocess exists (preserve legacy parsing behavior).
    if handlers.is_empty() {
        return Ok((xml.into(), handlers));
    }
    let selected = document.children[0]
        .children
        .iter_mut()
        .find(|n| n.name == "process" && n.attr("id") == Some(process_id))
        .unwrap();
    selected.children.retain(|n| !event_subprocess(n));
    document.children[0].attrs.insert(
        "xmlns:bpmninja".into(),
        "https://bpmninja.dev/schema/bpmn".into(),
    );
    Ok((serialize(&document), handlers))
}

pub(crate) fn attach(
    definition: &mut ProcessDefinition,
    handlers: Vec<ParsedHandler>,
) -> EngineResult<()> {
    for parsed in handlers {
        if definition.event_subprocesses.iter().any(|other| {
            other.id == parsed.handler.id
                || (other.trigger == parsed.handler.trigger
                    && !matches!(other.trigger, EventSubProcessTrigger::Timer(_)))
        }) {
            return Err(invalid("duplicate or ambiguous handler trigger"));
        }
        for (id, node) in parsed.body.nodes {
            if definition.nodes.insert(id.clone(), node).is_some() {
                return Err(invalid(format!("duplicate node ID '{id}'")));
            }
        }
        definition.flows.extend(parsed.body.flows);
        definition.listeners.extend(parsed.body.listeners);
        definition.loops.extend(parsed.body.loops);
        definition.data_mappings.extend(parsed.body.data_mappings);
        definition.event_subprocesses.push(parsed.handler);
    }
    Ok(())
}
