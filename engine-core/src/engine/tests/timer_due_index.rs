//! Timer due-index: `process_timers` uses the secondary index, not a full DashMap scan.
//! Event-driven scheduler tests use `tokio::time::pause` and the virtual clock.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use crate::domain::TimerDefinition;
use crate::runtime::InstanceState;

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

fn virtual_engine() -> Arc<WorkflowEngine> {
    let engine = Arc::new(WorkflowEngine::new());
    engine.enable_virtual_clock();
    engine
}

async fn yield_n(n: usize) {
    for _ in 0..n {
        tokio::task::yield_now().await;
    }
}

async fn deploy_hour_timer(engine: &WorkflowEngine) -> uuid::Uuid {
    let def = ProcessDefinitionBuilder::new("sched_timer")
        .node("start", BpmnElement::StartEvent)
        .node(
            "wait",
            BpmnElement::TimerCatchEvent(TimerDefinition::Duration(Duration::from_secs(3_600))),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "wait")
        .flow("wait", "end")
        .build()
        .unwrap();
    engine.deploy_definition(def).await.0
}

fn timer_id_for(engine: &WorkflowEngine, instance_id: uuid::Uuid) -> uuid::Uuid {
    engine
        .pending_timers
        .iter()
        .find(|timer| timer.instance_id == instance_id)
        .map(|timer| timer.id)
        .expect("instance has a pending timer")
}

fn spawn_scheduler(engine: Arc<WorkflowEngine>, max_idle: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            engine.wait_for_timer_work(max_idle).await;
            let _ = engine.process_timers().await;
        }
    })
}

#[tokio::test]
async fn next_due_after_skips_due_and_returns_the_earliest_future_timer() {
    let engine = WorkflowEngine::new();
    let now = chrono::Utc::now();
    let overdue = now - chrono::Duration::hours(1);
    let future_near = now + chrono::Duration::seconds(10);
    let future_far = now + chrono::Duration::seconds(30);
    for expires_at in [overdue, now, future_far, future_near] {
        engine.restore_timer(PendingTimer {
            id: uuid::Uuid::new_v4(),
            instance_id: uuid::Uuid::new_v4(),
            node_id: "t".into(),
            expires_at,
            token_id: uuid::Uuid::nil(),
            timer_def: None,
            remaining_repetitions: None,
        });
    }
    assert_eq!(
        engine.timer_due_index.next_due_after(now),
        Some(future_near)
    );
}

#[tokio::test(start_paused = true)]
async fn scheduler_fires_timer_at_due_time() {
    let engine = virtual_engine();
    let key = deploy_hour_timer(&engine).await;
    let id = engine.start_instance(key).await.unwrap();
    let due = Duration::from_secs(5);
    engine.set_timer_expiry(
        timer_id_for(&engine, id),
        engine.now() + chrono::Duration::seconds(5),
    );

    let scheduler = spawn_scheduler(engine.clone(), Duration::from_secs(60));
    let start = tokio::time::Instant::now();
    yield_n(8).await;
    assert_eq!(start.elapsed(), Duration::ZERO);
    assert_ne!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );

    tokio::time::advance(due - Duration::from_millis(1)).await;
    yield_n(8).await;
    assert_ne!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed,
        "timer fired before its due time"
    );

    tokio::time::advance(Duration::from_millis(1)).await;
    yield_n(16).await;
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(start.elapsed(), due);
    scheduler.abort();
}

#[tokio::test(start_paused = true)]
async fn earlier_timer_wakes_scheduler_before_previous_deadline() {
    let engine = virtual_engine();
    let key = deploy_hour_timer(&engine).await;
    let late = engine.start_instance(key).await.unwrap();
    let early = engine.start_instance(key).await.unwrap();
    let now = engine.now();
    engine.set_timer_expiry(
        timer_id_for(&engine, late),
        now + chrono::Duration::seconds(10),
    );
    engine.set_timer_expiry(
        timer_id_for(&engine, early),
        now + chrono::Duration::seconds(30),
    );

    let scheduler = spawn_scheduler(engine.clone(), Duration::from_secs(120));
    let start = tokio::time::Instant::now();
    yield_n(8).await;
    assert_eq!(start.elapsed(), Duration::ZERO);

    // Move `early` in front of the deadline the scheduler is already sleeping on.
    engine.set_timer_expiry(
        timer_id_for(&engine, early),
        engine.now() + chrono::Duration::seconds(2),
    );
    yield_n(8).await;

    tokio::time::advance(Duration::from_secs(2) - Duration::from_millis(1)).await;
    yield_n(8).await;
    assert_ne!(
        engine.get_instance_state(early).await.unwrap(),
        InstanceState::Completed
    );
    assert_ne!(
        engine.get_instance_state(late).await.unwrap(),
        InstanceState::Completed
    );

    tokio::time::advance(Duration::from_millis(1)).await;
    yield_n(16).await;
    assert_eq!(
        engine.get_instance_state(early).await.unwrap(),
        InstanceState::Completed,
        "earlier timer was not fired when it came due"
    );
    assert_ne!(
        engine.get_instance_state(late).await.unwrap(),
        InstanceState::Completed
    );

    tokio::time::advance(Duration::from_secs(8)).await;
    yield_n(16).await;
    assert_eq!(
        engine.get_instance_state(late).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(start.elapsed(), Duration::from_secs(10));
    scheduler.abort();
}

#[tokio::test(start_paused = true)]
async fn inserting_a_timer_wakes_a_parked_waiter() {
    let engine = virtual_engine();
    let waiting = engine.clone();
    let start = tokio::time::Instant::now();
    let waiter = tokio::spawn(async move {
        let started = tokio::time::Instant::now();
        waiting.wait_for_timer_work(Duration::from_secs(60)).await;
        started.elapsed()
    });
    yield_n(8).await;
    assert!(!waiter.is_finished());
    assert_eq!(start.elapsed(), Duration::ZERO);

    engine.restore_timer(PendingTimer {
        id: uuid::Uuid::new_v4(),
        instance_id: uuid::Uuid::new_v4(),
        node_id: "t".into(),
        expires_at: engine.now() + chrono::Duration::seconds(30),
        token_id: uuid::Uuid::nil(),
        timer_def: None,
        remaining_repetitions: None,
    });

    let elapsed = waiter.await.unwrap();
    assert!(
        elapsed < Duration::from_millis(1),
        "insert did not wake the scheduler, slept {elapsed:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn removing_a_timer_wakes_a_parked_waiter() {
    let engine = virtual_engine();
    let id = uuid::Uuid::new_v4();
    engine.restore_timer(PendingTimer {
        id,
        instance_id: uuid::Uuid::new_v4(),
        node_id: "t".into(),
        expires_at: engine.now() + chrono::Duration::seconds(30),
        token_id: uuid::Uuid::nil(),
        timer_def: None,
        remaining_repetitions: None,
    });

    let waiting = engine.clone();
    let waiter = tokio::spawn(async move {
        let started = tokio::time::Instant::now();
        waiting.wait_for_timer_work(Duration::from_secs(60)).await;
        started.elapsed()
    });
    yield_n(8).await;
    assert!(!waiter.is_finished());

    assert!(engine.remove_pending_timer(&id).is_some());
    let elapsed = waiter.await.unwrap();
    assert!(
        elapsed < Duration::from_millis(1),
        "cancellation did not wake the scheduler, slept {elapsed:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn shutdown_cancels_the_wait_without_firing() {
    let engine = virtual_engine();
    let key = deploy_hour_timer(&engine).await;
    let id = engine.start_instance(key).await.unwrap();
    engine.set_timer_expiry(
        timer_id_for(&engine, id),
        engine.now() + chrono::Duration::seconds(30),
    );

    let (tx, mut rx) = tokio::sync::watch::channel(false);
    let scheduler_engine = engine.clone();
    let start = tokio::time::Instant::now();
    let scheduler = tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = rx.changed() => break,
                _ = scheduler_engine.wait_for_timer_work(Duration::from_secs(60)) => {
                    let _ = scheduler_engine.process_timers().await;
                }
            }
        }
    });

    yield_n(8).await;
    assert!(!scheduler.is_finished());
    assert_eq!(start.elapsed(), Duration::ZERO);
    tx.send(true).unwrap();
    yield_n(8).await;
    assert!(scheduler.is_finished(), "shutdown did not cancel the wait");
    assert_ne!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(start.elapsed(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn suspended_overdue_timer_does_not_busy_loop() {
    let engine = virtual_engine();
    let key = deploy_hour_timer(&engine).await;
    let id = engine.start_instance(key).await.unwrap();
    engine.suspend_instance(id).await.unwrap();
    engine.set_timer_expiry(
        timer_id_for(&engine, id),
        engine.now() - chrono::Duration::hours(1),
    );

    let start = tokio::time::Instant::now();
    engine.wait_for_timer_work(Duration::from_millis(200)).await;
    assert_eq!(start.elapsed(), Duration::from_millis(200));
    assert_eq!(engine.pending_timers.len(), 1);
    assert!(matches!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Suspended { .. }
    ));
}

#[tokio::test(start_paused = true)]
async fn already_due_timer_is_ready_without_waiting_for_max_idle() {
    let engine = virtual_engine();
    let key = deploy_hour_timer(&engine).await;
    let id = engine.start_instance(key).await.unwrap();
    engine.set_timer_expiry(
        timer_id_for(&engine, id),
        engine.now() - chrono::Duration::seconds(1),
    );

    let start = tokio::time::Instant::now();
    engine.wait_for_timer_work(Duration::from_secs(60)).await;
    assert_eq!(start.elapsed(), Duration::ZERO);
    assert_eq!(engine.process_timers().await.unwrap(), 1);
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test(start_paused = true)]
async fn missing_instance_timer_is_rescheduled_instead_of_spinning() {
    let engine = virtual_engine();
    engine.restore_timer(PendingTimer {
        id: uuid::Uuid::new_v4(),
        instance_id: uuid::Uuid::new_v4(),
        node_id: "t".into(),
        expires_at: engine.now() - chrono::Duration::hours(1),
        token_id: uuid::Uuid::nil(),
        timer_def: None,
        remaining_repetitions: None,
    });

    let scheduler = spawn_scheduler(engine.clone(), Duration::from_secs(60));
    let start = tokio::time::Instant::now();
    yield_n(16).await;

    assert_eq!(start.elapsed(), Duration::ZERO);
    assert_eq!(
        engine.timer_processing_errors.load(Ordering::Relaxed),
        1,
        "failed timer was retried without sleeping"
    );
    let expiry = engine
        .pending_timers
        .iter()
        .next()
        .expect("timer kept")
        .expires_at;
    let pushed = expiry - engine.now();
    assert!(
        pushed >= chrono::Duration::milliseconds(1_000),
        "retry delay missing: {pushed:?}"
    );
    scheduler.abort();
}

#[tokio::test(start_paused = true)]
async fn resuming_an_overdue_timer_fires_without_waiting_max_idle() {
    let engine = virtual_engine();
    let key = deploy_hour_timer(&engine).await;
    let id = engine.start_instance(key).await.unwrap();
    engine.suspend_instance(id).await.unwrap();
    engine.set_timer_expiry(
        timer_id_for(&engine, id),
        engine.now() - chrono::Duration::hours(1),
    );

    let scheduler = spawn_scheduler(engine.clone(), Duration::from_secs(60));
    let start = tokio::time::Instant::now();
    yield_n(8).await;
    tokio::time::advance(Duration::from_millis(1)).await;
    yield_n(8).await;
    assert!(matches!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Suspended { .. }
    ));

    engine.resume_instance(id).await.unwrap();
    yield_n(16).await;
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
    assert!(
        start.elapsed() < Duration::from_secs(1),
        "resume waited for max idle, elapsed {:?}",
        start.elapsed()
    );
    scheduler.abort();
}
