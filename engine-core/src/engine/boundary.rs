use chrono::Utc;
use uuid::Uuid;

use crate::domain::{BpmnElement, ProcessDefinition, Token};
use crate::runtime::PendingTimer;

/// Scans the process definition for boundary events attached to the given `node_id`
/// and creates the corresponding pending timers or other wait states.
pub(crate) fn setup_boundary_events(
    def: &ProcessDefinition,
    attached_node_id: &str,
    instance_id: Uuid,
    token: &Token,
) -> (Vec<PendingTimer>, Vec<crate::runtime::PendingMessageCatch>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ProcessDefinitionBuilder;

    #[test]
    fn test_setup_boundary_no_events() {
        let def = ProcessDefinitionBuilder::new("no_boundary")
            .node("start", BpmnElement::StartEvent)
            .node("task", BpmnElement::UserTask("do".into()))
            .node("end", BpmnElement::EndEvent)
            .flow("start", "task")
            .flow("task", "end")
            .build()
            .unwrap();

        let token = Token::new("task");
        let (timers, msgs) = setup_boundary_events(&def, "task", Uuid::new_v4(), &token);
        assert!(timers.is_empty());
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_setup_boundary_timer_event() {
        let def = ProcessDefinitionBuilder::new("timer_boundary")
            .node("start", BpmnElement::StartEvent)
            .node("task", BpmnElement::UserTask("do".into()))
            .node(
                "timer_evt",
                BpmnElement::BoundaryTimerEvent {
                    attached_to: "task".into(),
                    timer: crate::domain::TimerDefinition::Duration(
                        std::time::Duration::from_secs(60),
                    ),
                    cancel_activity: true,
                },
            )
            .node("timeout_end", BpmnElement::EndEvent)
            .node("end", BpmnElement::EndEvent)
            .flow("start", "task")
            .flow("task", "end")
            .flow("timer_evt", "timeout_end")
            .build()
            .unwrap();

        let instance_id = Uuid::new_v4();
        let token = Token::new("task");
        let (timers, msgs) = setup_boundary_events(&def, "task", instance_id, &token);

        assert_eq!(timers.len(), 1);
        assert_eq!(timers[0].instance_id, instance_id);
        assert_eq!(timers[0].token_id, token.id);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_setup_boundary_message_event() {
        let def = ProcessDefinitionBuilder::new("msg_boundary")
            .node("start", BpmnElement::StartEvent)
            .node("task", BpmnElement::UserTask("do".into()))
            .node(
                "msg_evt",
                BpmnElement::BoundaryMessageEvent {
                    attached_to: "task".into(),
                    message_name: "cancel_order".into(),
                    cancel_activity: true,
                },
            )
            .node("cancel_end", BpmnElement::EndEvent)
            .node("end", BpmnElement::EndEvent)
            .flow("start", "task")
            .flow("task", "end")
            .flow("msg_evt", "cancel_end")
            .build()
            .unwrap();

        let instance_id = Uuid::new_v4();
        let token = Token::new("task");
        let (timers, msgs) = setup_boundary_events(&def, "task", instance_id, &token);

        assert!(timers.is_empty());
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message_name, "cancel_order");
        assert_eq!(msgs[0].instance_id, instance_id);
        assert_eq!(msgs[0].token_id, token.id);
    }
}
