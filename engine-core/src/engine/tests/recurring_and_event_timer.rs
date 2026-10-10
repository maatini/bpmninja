//! Recurring timer (R3) and event-based sibling-timer tests.

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use crate::domain::{BpmnElement, TimerDefinition};
use crate::runtime::InstanceState;
use std::collections::HashMap;
use std::time::Duration;

/// ISO `R3` must fire exactly 3 times, then stop (no 4th pending timer).
#[tokio::test]
async fn r3_repeating_interval_stops_after_three_fires() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("r3_timer")
        .node("start", BpmnElement::StartEvent)
        .node(
            "timer",
            BpmnElement::TimerCatchEvent(TimerDefinition::RepeatingInterval {
                repetitions: Some(3),
                interval: Duration::from_millis(1),
            }),
        )
        .node("task", BpmnElement::UserTask("keep_alive".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "timer")
        .flow("timer", "task")
        .flow("task", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    {
        let pending = engine
            .pending_timers
            .iter()
            .find(|t| t.node_id == "timer")
            .map(|t| t.value().clone())
            .expect("timer catch should be pending");
        assert_eq!(pending.remaining_repetitions, Some(3));
    }

    let mut fires = 0usize;
    loop {
        let now = chrono::Utc::now();
        let due: Vec<_> = engine
            .pending_timers
            .iter()
            .filter(|t| t.node_id == "timer")
            .map(|t| t.id)
            .collect();
        for id in due {
            engine.set_timer_expiry(id, now);
        }
        let n = engine.process_timers().await.unwrap();
        if n == 0 {
            break;
        }
        fires += n;
        assert!(
            fires <= 3,
            "R3 must not fire more than 3 times, got {fires}"
        );
    }

    assert_eq!(fires, 3, "R3 must fire exactly 3 times");
    assert!(
        engine.pending_timers.iter().all(|t| t.node_id != "timer"),
        "no pending timer for catch node after 3 fires"
    );

    let task_id = engine
        .pending_user_tasks
        .iter()
        .find(|t| t.instance_id == inst_id)
        .map(|t| t.task_id)
        .expect("user task should keep the instance alive after R3");
    engine
        .complete_user_task(task_id, HashMap::new())
        .await
        .unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

/// Two event-based timers with the same duration: the sibling is cancelled
/// mid-round and must not abort `process_timers` with "Timer disappeared".
#[tokio::test]
async fn event_based_two_timers_same_duration_does_not_error() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("ebg_two_timers")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::EventBasedGateway)
        .node(
            "timer_a",
            BpmnElement::TimerCatchEvent(TimerDefinition::Duration(Duration::from_secs(0))),
        )
        .node(
            "timer_b",
            BpmnElement::TimerCatchEvent(TimerDefinition::Duration(Duration::from_secs(0))),
        )
        .node("end_a", BpmnElement::EndEvent)
        .node("end_b", BpmnElement::EndEvent)
        .flow("start", "gw")
        .flow("gw", "timer_a")
        .flow("gw", "timer_b")
        .flow("timer_a", "end_a")
        .flow("timer_b", "end_b")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    assert_eq!(engine.pending_timers.len(), 2);

    let result = engine.process_timers().await;
    assert!(
        result.is_ok(),
        "sibling timer must not abort the round: {result:?}"
    );

    assert_eq!(engine.pending_timers.len(), 0);

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert!(
        inst.current_node == "end_a" || inst.current_node == "end_b",
        "one of the two event-based paths must complete, got {}",
        inst.current_node
    );
}
