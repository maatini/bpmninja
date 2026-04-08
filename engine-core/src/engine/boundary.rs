use chrono::Utc;
use uuid::Uuid;

use crate::runtime::PendingTimer;
use crate::domain::{BpmnElement, ProcessDefinition, Token};

/// Scans the process definition for boundary events attached to the given `node_id`
/// and creates the corresponding pending timers or other wait states.
pub(crate) fn setup_boundary_events(
    def: &ProcessDefinition,
    attached_node_id: &str,
    instance_id: Uuid,
    token: &Token,
) -> (
    Vec<PendingTimer>,
    Vec<crate::runtime::PendingMessageCatch>,
) {
    let mut pending_timers = Vec::new();
    let mut pending_msgs = Vec::new();

    let mut bounds_timers = Vec::new();
    let mut bounds_msgs = Vec::new();

    for (node_id, node) in &def.nodes {
        if let BpmnElement::BoundaryTimerEvent {
            attached_to, timer, ..
        } = node
        {
            // Rust clippy warning fix
            if attached_to == attached_node_id {
                bounds_timers.push((node_id.clone(), timer.clone()));
            }
        }

        if let BpmnElement::BoundaryMessageEvent {
            attached_to,
            message_name,
            ..
        } = node
        {
            // Rust clippy warning fix
            if attached_to == attached_node_id {
                bounds_msgs.push((node_id.clone(), message_name.clone()));
            }
        }
    }

    for (node_id, timer_def) in bounds_timers {
        let now = Utc::now();
        let expires_at = timer_def.next_expiry(now).unwrap_or(now);
        let pending = PendingTimer {
            id: Uuid::new_v4(),
            instance_id,
            node_id,
            expires_at,
            token_id: token.id,
            timer_def: Some(timer_def),
            remaining_repetitions: None,
        };
        pending_timers.push(pending);
    }

    for (node_id, message_name) in bounds_msgs {
        let pending = crate::runtime::PendingMessageCatch {
            id: Uuid::new_v4(),
            instance_id,
            node_id,
            message_name,
            token_id: token.id,
        };
        pending_msgs.push(pending);
    }

    (pending_timers, pending_msgs)
}
