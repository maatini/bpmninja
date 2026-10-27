//! Explicit collaboration metadata. Parsing does not deploy pools or deliver messages.
use crate::data::{Node, tree};
use crate::parse_bpmn_processes;
use engine_core::domain::{BpmnElement, ProcessDefinition};
use engine_core::error::{EngineError, EngineResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationDocument {
    pub processes: Vec<ProcessDefinition>,
    pub collaborations: Vec<Collaboration>,
}

pub use engine_core::domain::collaboration::{
    Collaboration, MessageEndpoint, MessageFlow, Participant,
};

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidDefinition(format!("Collaboration: {}", message.into()))
}

fn required<'a>(node: &'a Node, attribute: &str) -> EngineResult<&'a str> {
    node.attr(attribute)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid(format!("{} requires {attribute}", node.name)))
}

// The existing process parser resolves unqualified IDs. Reject QNames here
// rather than producing routing metadata inconsistent with compiled processes.
fn reference<'a>(node: &'a Node, attribute: &str) -> EngineResult<&'a str> {
    let value = required(node, attribute)?;
    if value.contains(':') || value.trim() != value {
        return Err(invalid(format!(
            "{} {attribute} must be an unqualified local ID, got '{value}'",
            node.name
        )));
    }
    Ok(value)
}

fn check_ids<'a>(node: &'a Node, seen: &mut HashSet<&'a str>) -> EngineResult<()> {
    if let Some(id) = node.attr("id")
        && (id.trim().is_empty() || !seen.insert(id))
    {
        return Err(invalid(format!("empty or duplicate document ID '{id}'")));
    }
    for child in &node.children {
        check_ids(child, seen)?;
    }
    Ok(())
}

/// Parse executable processes plus validated, directed message-flow metadata.
///
/// Internal routes require top-level message send/catch elements in distinct
/// executable participant processes and matching declared message references.
/// Black-box/non-executable pools may be displayed, but cannot be flow endpoints.
/// Participant-level, nested-scope and loop endpoints require later increments.
pub fn parse_bpmn_collaboration(xml: &str) -> EngineResult<CollaborationDocument> {
    let document = tree(xml)?;
    let definitions = document
        .children("definitions")
        .next()
        .ok_or_else(|| invalid("missing definitions element"))?;
    check_ids(definitions, &mut HashSet::new())?;
    let processes = parse_bpmn_processes(xml)?;
    let process_nodes: HashMap<_, _> = definitions
        .children("process")
        .map(|node| Ok((required(node, "id")?, node)))
        .collect::<EngineResult<_>>()?;
    let compiled: HashMap<_, _> = processes
        .iter()
        .map(|process| (process.id.as_str(), process))
        .collect();
    let messages: HashMap<_, _> = definitions
        .children("message")
        .map(|node| Ok((required(node, "id")?, required(node, "name")?)))
        .collect::<EngineResult<_>>()?;
    let mut collaborations = Vec::new();
    for node in definitions.children("collaboration") {
        let id = required(node, "id")?.to_owned();
        let mut participants = Vec::new();
        let mut by_process = HashMap::new();
        for participant in node.children("participant") {
            let participant_id = required(participant, "id")?;
            let process_id = participant
                .attr("processRef")
                .map(|_| reference(participant, "processRef"))
                .transpose()?;
            if let Some(process_id) = process_id {
                if !process_nodes.contains_key(process_id) {
                    return Err(invalid(format!(
                        "participant '{participant_id}' references unknown process '{process_id}'"
                    )));
                }
                if by_process.insert(process_id, participant_id).is_some() {
                    return Err(invalid(format!(
                        "process '{process_id}' has multiple participants in '{id}'"
                    )));
                }
            }
            participants.push(Participant {
                id: participant_id.into(),
                process_id: process_id.map(str::to_owned),
                executable: process_id.is_some_and(|id| compiled.contains_key(id)),
            });
        }
        let mut message_flows = Vec::new();
        for flow in node.children("messageFlow") {
            let flow_id = required(flow, "id")?;
            let (source, source_message) = endpoint(
                flow,
                "sourceRef",
                true,
                &process_nodes,
                &compiled,
                &by_process,
            )?;
            let (target, target_message) = endpoint(
                flow,
                "targetRef",
                false,
                &process_nodes,
                &compiled,
                &by_process,
            )?;
            if source.process_id == target.process_id {
                return Err(invalid(format!(
                    "messageFlow '{flow_id}' must cross participant processes"
                )));
            }
            if source_message != target_message {
                return Err(invalid(format!(
                    "messageFlow '{flow_id}' endpoints reference different messages"
                )));
            }
            if flow.attr("messageRef").is_some() && reference(flow, "messageRef")? != source_message
            {
                return Err(invalid(format!(
                    "messageFlow '{flow_id}' messageRef differs from its endpoints"
                )));
            }
            let message_name = messages.get(source_message.as_str()).ok_or_else(|| {
                invalid(format!(
                    "unknown message '{source_message}' on flow '{flow_id}'"
                ))
            })?;
            // Process parsing has fallback message names for legacy inputs. Such
            // fallbacks must never change a validated internal route's meaning.
            for endpoint in [&source, &target] {
                let element = &compiled[endpoint.process_id.as_str()].nodes[&endpoint.element_id];
                let actual = match element {
                    BpmnElement::SendTask { message_name, .. }
                    | BpmnElement::MessageStartEvent { message_name }
                    | BpmnElement::MessageCatchEvent { message_name }
                    | BpmnElement::BoundaryMessageEvent { message_name, .. } => message_name,
                    _ => unreachable!("endpoint validated below"),
                };
                if actual != message_name {
                    return Err(invalid(format!(
                        "compiled message differs on endpoint '{}'",
                        endpoint.element_id
                    )));
                }
            }
            message_flows.push(MessageFlow {
                id: flow_id.into(),
                message_id: source_message,
                message_name: (*message_name).into(),
                source,
                target,
            });
        }
        collaborations.push(Collaboration {
            id,
            participants,
            message_flows,
        });
    }
    Ok(CollaborationDocument {
        processes,
        collaborations,
    })
}

fn endpoint(
    flow: &Node,
    attribute: &str,
    source: bool,
    process_nodes: &HashMap<&str, &Node>,
    compiled: &HashMap<&str, &ProcessDefinition>,
    participants: &HashMap<&str, &str>,
) -> EngineResult<(MessageEndpoint, String)> {
    let element_id = reference(flow, attribute)?;
    let (process_id, node) = process_nodes.iter().find_map(|(process_id, process)| {
        process.children.iter().find(|node| node.attr("id") == Some(element_id))
            .map(|node| (*process_id, node))
    }).ok_or_else(|| invalid(format!("{attribute} '{element_id}' must reference a top-level message element; participant and nested endpoints are unsupported")))?;
    let participant_id = participants.get(process_id).ok_or_else(|| {
        invalid(format!(
            "endpoint '{element_id}' has no participant in this collaboration"
        ))
    })?;
    let process = compiled.get(process_id).ok_or_else(|| {
        invalid(format!(
            "endpoint '{element_id}' belongs to a non-executable pool"
        ))
    })?;
    let element = process
        .nodes
        .get(element_id)
        .ok_or_else(|| invalid(format!("unknown message endpoint '{element_id}'")))?;
    let supported = if source {
        matches!(element, BpmnElement::SendTask { .. })
    } else {
        matches!(
            element,
            BpmnElement::MessageStartEvent { .. }
                | BpmnElement::MessageCatchEvent { .. }
                | BpmnElement::BoundaryMessageEvent { .. }
        )
    };
    if !supported {
        return Err(invalid(format!(
            "unsupported {attribute} kind on '{element_id}'"
        )));
    }
    let loop_attachment = match element {
        BpmnElement::BoundaryMessageEvent { attached_to, .. } => Some(attached_to.as_str()),
        _ => None,
    };
    if process.loops.contains_key(element_id)
        || loop_attachment.is_some_and(|id| process.loops.contains_key(id))
    {
        return Err(invalid(format!(
            "loop endpoint '{element_id}' requires occurrence-aware routing"
        )));
    }
    let nested = node.children("messageEventDefinition").next();
    let message_node = nested
        .filter(|node| node.attr("messageRef").is_some())
        .unwrap_or(node);
    let message_id = reference(message_node, "messageRef")?.to_owned();
    if let (Some(nested), Some(direct)) = (nested, node.attr("messageRef"))
        && nested
            .attr("messageRef")
            .is_some_and(|value| value != direct)
    {
        return Err(invalid(format!(
            "conflicting message references on '{element_id}'"
        )));
    }
    Ok((
        MessageEndpoint {
            participant_id: (*participant_id).into(),
            process_id: process_id.into(),
            element_id: element_id.into(),
        },
        message_id,
    ))
}
