use chrono::Utc;
use uuid::Uuid;

use crate::domain::{BpmnElement, ProcessDefinition, Token};
use crate::runtime::{PendingMessageCatch, PendingSignalCatch, PendingTimer};

/// Wait-States, die beim Betreten einer Aktivität für ihre Boundary-Events
/// eingerichtet werden.
pub(crate) struct BoundaryEvents {
    pub(crate) timers: Vec<PendingTimer>,
    pub(crate) messages: Vec<PendingMessageCatch>,
    pub(crate) signals: Vec<PendingSignalCatch>,
}

/// Scans the process definition for boundary events attached to the given `node_id`
/// and creates the corresponding pending timers or other wait states.
pub(crate) fn setup_boundary_events(
    def: &ProcessDefinition,
    attached_node_id: &str,
    instance_id: Uuid,
    token: &Token,
) -> BoundaryEvents {
    let mut pending_timers = Vec::new();
    let mut pending_msgs = Vec::new();
    let mut pending_signals = Vec::new();

    let mut bounds_timers = Vec::new();
    let mut bounds_msgs = Vec::new();
    let mut bounds_signals = Vec::new();

    for (node_id, node) in &def.nodes {
        match node {
            BpmnElement::BoundaryTimerEvent {
                attached_to, timer, ..
            } if attached_to == attached_node_id => {
                bounds_timers.push((node_id.clone(), timer.clone()));
            }
            BpmnElement::BoundaryMessageEvent {
                attached_to,
                message_name,
                ..
            } if attached_to == attached_node_id => {
                bounds_msgs.push((node_id.clone(), message_name.clone()));
            }
            BpmnElement::BoundarySignalEvent {
                attached_to,
                signal_name,
                ..
            } if attached_to == attached_node_id => {
                bounds_signals.push((node_id.clone(), signal_name.clone()));
            }
            _ => {}
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
            remaining_repetitions: timer_def.initial_remaining_repetitions(),
            timer_def: Some(timer_def),
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

    for (node_id, signal_name) in bounds_signals {
        let pending = PendingSignalCatch {
            id: Uuid::new_v4(),
            instance_id,
            node_id,
            signal_name,
            token_id: token.id,
        };
        pending_signals.push(pending);
    }

    BoundaryEvents {
        timers: pending_timers,
        messages: pending_msgs,
        signals: pending_signals,
    }
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
        let events = setup_boundary_events(&def, "task", Uuid::new_v4(), &token);
        assert!(events.timers.is_empty());
        assert!(events.messages.is_empty());
        assert!(events.signals.is_empty());
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
        let events = setup_boundary_events(&def, "task", instance_id, &token);

        assert_eq!(events.timers.len(), 1);
        assert_eq!(events.timers[0].instance_id, instance_id);
        assert_eq!(events.timers[0].token_id, token.id);
        assert!(events.messages.is_empty());
        assert!(events.signals.is_empty());
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
        let events = setup_boundary_events(&def, "task", instance_id, &token);

        assert!(events.timers.is_empty());
        assert_eq!(events.messages.len(), 1);
        assert_eq!(events.messages[0].message_name, "cancel_order");
        assert_eq!(events.messages[0].instance_id, instance_id);
        assert_eq!(events.messages[0].token_id, token.id);
        assert!(events.signals.is_empty());
    }

    #[test]
    fn test_setup_boundary_signal_event() {
        let def = ProcessDefinitionBuilder::new("signal_boundary")
            .node("start", BpmnElement::StartEvent)
            .node("task", BpmnElement::UserTask("do".into()))
            .node(
                "sig_evt",
                BpmnElement::BoundarySignalEvent {
                    attached_to: "task".into(),
                    signal_name: "abort".into(),
                    cancel_activity: false,
                },
            )
            .node("cancel_end", BpmnElement::EndEvent)
            .node("end", BpmnElement::EndEvent)
            .flow("start", "task")
            .flow("task", "end")
            .flow("sig_evt", "cancel_end")
            .build()
            .unwrap();

        let instance_id = Uuid::new_v4();
        let token = Token::new("task");
        let events = setup_boundary_events(&def, "task", instance_id, &token);

        assert!(events.timers.is_empty());
        assert!(events.messages.is_empty());
        assert_eq!(events.signals.len(), 1);
        assert_eq!(events.signals[0].signal_name, "abort");
        assert_eq!(events.signals[0].node_id, "sig_evt");
        assert_eq!(events.signals[0].instance_id, instance_id);
        assert_eq!(events.signals[0].token_id, token.id);
    }
}
