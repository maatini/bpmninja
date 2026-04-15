use std::collections::{HashSet, VecDeque};

use crate::condition::evaluate_condition;
use crate::domain::{BpmnElement, EngineError, EngineResult, ProcessDefinition};

pub(crate) fn resolve_next_target(
    def: &ProcessDefinition,
    from: &str,
    variables: &std::collections::HashMap<String, serde_json::Value>,
) -> EngineResult<String> {
    def.next_nodes(from)
        .iter()
        .find(|f| {
            f.condition
                .as_ref()
                .map(|c| evaluate_condition(c, variables))
                .unwrap_or(true)
        })
        .map(|f| f.target.clone())
        .ok_or_else(|| {
            EngineError::InvalidDefinition(format!("No matching outgoing flow from '{from}'"))
        })
}

pub(crate) fn find_boundary_error_event(
    def: &ProcessDefinition,
    attached_to_node: &str,
    error_code: &str,
) -> Option<String> {
    def.nodes.iter().find_map(|(node_id, node)| {
        if let BpmnElement::BoundaryErrorEvent {
            attached_to,
            error_code: bound_err,
        } = node
            && attached_to == attached_to_node
            && (bound_err.is_none() || bound_err.as_deref() == Some(error_code))
        {
            return Some(node_id.clone());
        }
        None
    })
}

/// Finds a boundary escalation event attached to the given node matching the escalation code.
/// Wildcard: `escalation_code: None` catches any escalation.
#[allow(dead_code)]
pub(crate) fn find_boundary_escalation_event(
    def: &ProcessDefinition,
    attached_to_node: &str,
    escalation_code: &str,
) -> Option<(String, bool)> {
    def.nodes.iter().find_map(|(node_id, node)| {
        if let BpmnElement::BoundaryEscalationEvent {
            attached_to,
            escalation_code: bound_esc,
            cancel_activity,
        } = node
            && attached_to == attached_to_node
            && (bound_esc.is_none() || bound_esc.as_deref() == Some(escalation_code))
        {
            return Some((node_id.clone(), *cancel_activity));
        }
        None
    })
}

/// Scans all nodes for a BoundaryEscalationEvent matching the escalation code (any attachment).
pub(crate) fn find_any_boundary_escalation_event(
    def: &ProcessDefinition,
    escalation_code: &str,
) -> Option<(String, String, bool)> {
    def.nodes.iter().find_map(|(node_id, node)| {
        if let BpmnElement::BoundaryEscalationEvent {
            attached_to,
            escalation_code: bound_esc,
            cancel_activity,
        } = node
            && (bound_esc.is_none() || bound_esc.as_deref() == Some(escalation_code))
        {
            return Some((node_id.clone(), attached_to.clone(), *cancel_activity));
        }
        None
    })
}

/// Finds the compensation handler (outgoing node from BoundaryCompensationEvent) for an activity.
pub(crate) fn find_compensation_handler(
    def: &ProcessDefinition,
    activity_id: &str,
) -> Option<String> {
    let boundary_id = def.nodes.iter().find_map(|(node_id, node)| {
        if let BpmnElement::BoundaryCompensationEvent { attached_to } = node
            && attached_to == activity_id
        {
            Some(node_id.clone())
        } else {
            None
        }
    })?;
    def.next_nodes(&boundary_id)
        .first()
        .map(|f| f.target.clone())
}

pub(crate) fn same_gateway_type(a: &BpmnElement, b: &BpmnElement) -> bool {
    matches!(
        (a, b),
        (BpmnElement::ExclusiveGateway { .. }, BpmnElement::ExclusiveGateway { .. })
            | (BpmnElement::InclusiveGateway, BpmnElement::InclusiveGateway)
            | (BpmnElement::ParallelGateway, BpmnElement::ParallelGateway)
    )
}

pub(crate) fn find_downstream_join(def: &ProcessDefinition, start_node: &str) -> Option<String> {
    let split_element = def.nodes.get(start_node)?;
    let mut visited = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();

    for flow in def.next_nodes(start_node) {
        queue.push_back((flow.target.clone(), 1));
    }

    while let Some((node, depth)) = queue.pop_front() {
        if visited.contains(&node) {
            continue;
        }
        visited.insert(node.clone());

        if let Some(element) = def.nodes.get(&node) {
            if def.is_join_gateway(&node) && same_gateway_type(split_element, element) {
                if depth == 1 {
                    return Some(node.clone());
                }
                for flow in def.next_nodes(&node) {
                    queue.push_back((flow.target.clone(), depth - 1));
                }
                continue;
            }

            if def.is_split_gateway(&node) && same_gateway_type(split_element, element) {
                for flow in def.next_nodes(&node) {
                    queue.push_back((flow.target.clone(), depth + 1));
                }
                continue;
            }
        }

        for flow in def.next_nodes(&node) {
            queue.push_back((flow.target.clone(), depth));
        }
    }
    None
}
