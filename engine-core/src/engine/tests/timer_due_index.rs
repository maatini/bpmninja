//! Timer due-index: `process_timers` uses the secondary index, not a full DashMap scan.

use std::time::Duration;

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use crate::domain::TimerDefinition;

#[tokio::test]
async fn restore_timer_is_due_indexed() {
    let engine = WorkflowEngine::new();
    let past = chrono::Utc::now() - chrono::Duration::hours(1);
    let id = uuid::Uuid::new_v4();
    engine.restore_timer(PendingTimer {
        id,
        instance_id: uuid::Uuid::new_v4(),
        node_id: "t".into(),
        expires_at: past,
        token_id: uuid::Uuid::nil(),
        timer_def: None,
        remaining_repetitions: None,
    });
    assert!(engine.timer_due_index.contains(id, past));
    assert_eq!(engine.timer_due_index.len(), 1);
}

#[tokio::test]
async fn future_timer_is_not_due_until_expiry_updated() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("due_idx")
        .node("start", BpmnElement::StartEvent)
        .node(
            "timer",
            BpmnElement::TimerCatchEvent(TimerDefinition::Duration(Duration::from_secs(3600))),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "timer")
        .flow("timer", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    engine.start_instance(key).await.unwrap();

    assert_eq!(engine.pending_timers.len(), 1);
    assert_eq!(engine.timer_due_index.len(), 1);
    assert_eq!(engine.process_timers().await.unwrap(), 0);

    let tid = engine.pending_timers.iter().next().unwrap().id;
    engine.set_timer_expiry(tid, chrono::Utc::now() - chrono::Duration::seconds(1));
    assert_eq!(engine.process_timers().await.unwrap(), 1);
    assert_eq!(engine.pending_timers.len(), 0);
    assert_eq!(engine.timer_due_index.len(), 0);
}

#[tokio::test]
async fn delete_instance_drops_due_index_entries() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("due_del")
        .node("start", BpmnElement::StartEvent)
        .node(
            "timer",
            BpmnElement::TimerCatchEvent(TimerDefinition::Duration(Duration::from_secs(3600))),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "timer")
        .flow("timer", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst = engine.start_instance(key).await.unwrap();
    assert_eq!(engine.timer_due_index.len(), 1);
    engine.delete_instance(inst).await.unwrap();
    assert_eq!(engine.pending_timers.len(), 0);
    assert_eq!(engine.timer_due_index.len(), 0);
}
