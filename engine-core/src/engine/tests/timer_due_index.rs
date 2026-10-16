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

// --- P5: event-driven timer scheduler ----------------------------------------

/// Scheduler loop as in `engine-server/main.rs`, with a long max idle so only
/// due-time scheduling and insert notifications can explain a prompt firing.
fn spawn_scheduler(engine: &Arc<WorkflowEngine>) -> tokio::task::JoinHandle<()> {
    let engine = engine.clone();
    tokio::spawn(async move {
        loop {
            engine.wait_for_timer_work(Duration::from_secs(60)).await;
            let _ = engine.process_timers().await;
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scheduler_fires_timer_when_due_not_after_max_idle() {
    let engine = Arc::new(WorkflowEngine::new());
    let def = ProcessDefinitionBuilder::new("timer_on_time")
        .node("start", BpmnElement::StartEvent)
        .node(
            "wait",
            BpmnElement::TimerCatchEvent(TimerDefinition::Duration(Duration::from_millis(300))),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "wait")
        .flow("wait", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // Scheduler starts idle (no timers) — the insert must wake it.
    let scheduler = spawn_scheduler(&engine);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let started = std::time::Instant::now();
    let id = engine.start_instance(key).await.unwrap();
    loop {
        if engine.get_instance_state(id).await.unwrap() == InstanceState::Completed {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "timer did not fire on time"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(started.elapsed() >= Duration::from_millis(300));
    scheduler.abort();
}

#[tokio::test]
async fn overdue_unfired_timer_does_not_cause_busy_wakeups() {
    let engine = WorkflowEngine::new();
    // Overdue timer that never fires (unknown instance) — like a suspended one.
    engine.restore_timer(PendingTimer {
        id: uuid::Uuid::new_v4(),
        instance_id: uuid::Uuid::new_v4(),
        node_id: "t".into(),
        expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
        token_id: uuid::Uuid::nil(),
        timer_def: None,
        remaining_repetitions: None,
    });
    let started = std::time::Instant::now();
    engine.wait_for_timer_work(Duration::from_millis(200)).await;
    assert!(
        started.elapsed() >= Duration::from_millis(190),
        "scheduler returned immediately for an overdue, unfired timer"
    );
}
