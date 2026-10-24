//! Background retry queue for failed persistence operations.
//!
//! When a persist operation fails after inline retries, the job is pushed
//! to a **bounded** channel. A background tokio task drains this channel,
//! re-reads the current in-memory state, and retries the persistence call
//! with exponential backoff.
//!
//! If the queue is full, new jobs are dropped and counted via
//! `bpmn_persistence_retry_dropped_total` (prevents OOM under NATS outage).

use super::super::runtime::*;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::domain::IncidentKind;
use crate::history::HistoryEntry;
use crate::persistence::WorkflowPersistence;

/// Maximum number of retry attempts per job before giving up.
const MAX_RETRIES: u32 = 50;

/// Initial backoff delay (doubles each attempt, capped at MAX_BACKOFF).
const INITIAL_BACKOFF_MS: u64 = 1_000;

/// Maximum backoff delay.
const MAX_BACKOFF_MS: u64 = 60_000;

/// Number of inline retry attempts (before queuing to background).
pub(crate) const INLINE_RETRIES: u32 = 2;

/// Delay between inline retries in milliseconds (doubles each attempt).
pub(crate) const INLINE_BACKOFF_MS: u64 = 50;

/// Default capacity of the background retry queue.
pub(crate) const DEFAULT_RETRY_QUEUE_CAPACITY: usize = 10_000;

/// Default time budget for draining the queue after `PersistJob::Shutdown`.
pub(crate) const DEFAULT_SHUTDOWN_DRAIN_MS: u64 = 5_000;

/// Resolves retry queue capacity from `PERSISTENCE_RETRY_QUEUE_CAPACITY` or default.
pub(crate) fn retry_queue_capacity_from_env() -> usize {
    std::env::var("PERSISTENCE_RETRY_QUEUE_CAPACITY")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_RETRY_QUEUE_CAPACITY)
}

/// Resolves the shutdown drain budget from `PERSISTENCE_SHUTDOWN_DRAIN_MS` or default.
pub(crate) fn shutdown_drain_budget_from_env() -> Duration {
    let ms = std::env::var("PERSISTENCE_SHUTDOWN_DRAIN_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SHUTDOWN_DRAIN_MS);
    Duration::from_millis(ms)
}

/// A persistence operation that can be retried.
#[derive(Debug, Clone)]
pub(crate) enum PersistJob {
    /// Re-read instance from InstanceStore and save to NATS.
    SaveInstance(Uuid),
    /// Re-read definition from DefinitionRegistry and save to NATS.
    SaveDefinition(Uuid),
    /// Re-read user task from pending_user_tasks and save to NATS.
    SaveUserTask(Uuid),
    /// Delete a user task from NATS.
    DeleteUserTask(Uuid),
    /// Re-read service task from pending_service_tasks and save to NATS.
    SaveServiceTask(Uuid),
    /// Delete a service task from NATS.
    DeleteServiceTask(Uuid),
    /// Re-read timer from pending_timers and save to NATS.
    SaveTimer(Uuid),
    /// Delete a timer from NATS.
    DeleteTimer(Uuid),
    /// Re-read message catch from pending_message_catches and save to NATS.
    SaveMessageCatch(Uuid),
    /// Delete a message catch from NATS.
    DeleteMessageCatch(Uuid),
    /// Append a history entry to NATS.
    AppendHistoryEntry(Box<HistoryEntry>),
    /// Instructs the worker to drain the remaining jobs and exit.
    Shutdown,
}

impl std::fmt::Display for PersistJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SaveInstance(id) => write!(f, "SaveInstance({})", id),
            Self::SaveDefinition(id) => write!(f, "SaveDefinition({})", id),
            Self::SaveUserTask(id) => write!(f, "SaveUserTask({})", id),
            Self::DeleteUserTask(id) => write!(f, "DeleteUserTask({})", id),
            Self::SaveServiceTask(id) => write!(f, "SaveServiceTask({})", id),
            Self::DeleteServiceTask(id) => write!(f, "DeleteServiceTask({})", id),
            Self::SaveTimer(id) => write!(f, "SaveTimer({})", id),
            Self::DeleteTimer(id) => write!(f, "DeleteTimer({})", id),
            Self::SaveMessageCatch(id) => write!(f, "SaveMessageCatch({})", id),
            Self::DeleteMessageCatch(id) => write!(f, "DeleteMessageCatch({})", id),
            Self::AppendHistoryEntry(e) => write!(f, "AppendHistoryEntry({})", e.instance_id),
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}

/// Sender half of the retry queue. Cheap to clone.
pub(crate) type RetryQueueTx = mpsc::Sender<PersistJob>;

/// Receiver half of the retry queue (consumed by the background task).
pub(crate) type RetryQueueRx = mpsc::Receiver<PersistJob>;

/// Creates a new bounded retry queue with default/env capacity.
pub(crate) fn create_retry_queue() -> (RetryQueueTx, RetryQueueRx) {
    create_retry_queue_with_capacity(retry_queue_capacity_from_env())
}

/// Creates a bounded retry queue with an explicit capacity (tests / custom sizing).
pub(crate) fn create_retry_queue_with_capacity(capacity: usize) -> (RetryQueueTx, RetryQueueRx) {
    mpsc::channel(capacity.max(1))
}

/// Spawns the background retry worker.
///
/// This task receives failed `PersistJob`s from the channel and retries them
/// with exponential backoff. It re-reads the current in-memory state from the
/// provided data sources (instance_store, definition_registry, pending maps)
/// so it always persists the *latest* state, not a stale snapshot.
///
/// On `PersistJob::Shutdown` the worker drains all jobs still queued in the
/// channel with a single attempt each, bounded by
/// `PERSISTENCE_SHUTDOWN_DRAIN_MS` (default 5s), before exiting.
///
/// The `persistence`, `instances`, and `definitions` parameters are shared
/// references from the WorkflowEngine.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_retry_worker(
    mut rx: RetryQueueRx,
    persistence: Arc<dyn WorkflowPersistence>,
    instances: crate::engine::instance_store::InstanceStore,
    definitions: crate::engine::registry::DefinitionRegistry,
    pending_user_tasks: Arc<DashMap<Uuid, PendingUserTask>>,
    pending_service_tasks: Arc<DashMap<Uuid, PendingServiceTask>>,
    pending_timers: Arc<DashMap<Uuid, PendingTimer>>,
    pending_message_catches: Arc<DashMap<Uuid, PendingMessageCatch>>,
    error_counter: Arc<std::sync::atomic::AtomicU64>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Persistence retry worker started");

        while let Some(job) = rx.recv().await {
            if matches!(job, PersistJob::Shutdown) {
                let budget = shutdown_drain_budget_from_env();
                tracing::info!(
                    "Retry worker received shutdown signal, draining queued jobs (budget: {}ms)",
                    budget.as_millis()
                );
                let ctx = DrainContext {
                    persistence: &persistence,
                    instances: &instances,
                    definitions: &definitions,
                    pending_user_tasks: &pending_user_tasks,
                    pending_service_tasks: &pending_service_tasks,
                    pending_timers: &pending_timers,
                    pending_message_catches: &pending_message_catches,
                    error_counter: &error_counter,
                };
                let drained = drain_retry_queue(&mut rx, budget, &ctx).await;

                if drained.remaining > 0 {
                    metrics::gauge!("bpmn_persistence_retry_drain_remaining")
                        .set(drained.remaining as f64);
                    tracing::error!(
                        processed = drained.processed,
                        failed = drained.failed,
                        remaining = drained.remaining,
                        "Persistence shutdown drain budget exhausted; {} job(s) left in queue",
                        drained.remaining
                    );
                } else if drained.failed > 0 {
                    tracing::warn!(
                        processed = drained.processed,
                        failed = drained.failed,
                        "Persistence shutdown drain finished with failed job(s)"
                    );
                } else {
                    tracing::info!(
                        processed = drained.processed,
                        "Persistence shutdown drain finished"
                    );
                }
                break;
            }

            let mut attempt = 0u32;
            let mut backoff_ms = INITIAL_BACKOFF_MS;

            loop {
                attempt += 1;
                let result = execute_job(
                    &job,
                    &persistence,
                    &instances,
                    &definitions,
                    &pending_user_tasks,
                    &pending_service_tasks,
                    &pending_timers,
                    &pending_message_catches,
                )
                .await;

                match result {
                    Ok(()) => {
                        tracing::info!("Retry succeeded for {} (attempt {})", job, attempt);
                        break;
                    }
                    Err(e) if attempt >= MAX_RETRIES => {
                        error_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        metrics::counter!("bpmn_persistence_retry_exhausted_total").increment(1);
                        record_exhausted_job(
                            &job,
                            &e,
                            &instances,
                            &pending_user_tasks,
                            &pending_service_tasks,
                            &pending_timers,
                            &pending_message_catches,
                        )
                        .await;
                        tracing::error!(
                            "PERMANENT PERSISTENCE FAILURE after {} retries for {}: {}",
                            MAX_RETRIES,
                            job,
                            e
                        );
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Retry {}/{} for {} failed: {} — backing off {}ms",
                            attempt,
                            MAX_RETRIES,
                            job,
                            e,
                            backoff_ms
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                        backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                    }
                }
            }
        }

        tracing::info!("Persistence retry worker stopped");
    })
}

/// Outcome of a shutdown drain.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DrainResult {
    /// Jobs executed successfully (single attempt, no backoff).
    pub(crate) processed: usize,
    /// Jobs whose single attempt failed or timed out.
    pub(crate) failed: usize,
    /// Jobs that were still queued when the budget expired and were not attempted.
    pub(crate) remaining: usize,
}

/// Borrowed dependencies required to execute queued persist jobs.
struct DrainContext<'a> {
    persistence: &'a Arc<dyn WorkflowPersistence>,
    instances: &'a crate::engine::instance_store::InstanceStore,
    definitions: &'a crate::engine::registry::DefinitionRegistry,
    pending_user_tasks: &'a Arc<DashMap<Uuid, PendingUserTask>>,
    pending_service_tasks: &'a Arc<DashMap<Uuid, PendingServiceTask>>,
    pending_timers: &'a Arc<DashMap<Uuid, PendingTimer>>,
    pending_message_catches: &'a Arc<DashMap<Uuid, PendingMessageCatch>>,
    error_counter: &'a Arc<std::sync::atomic::AtomicU64>,
}

impl DrainContext<'_> {
    async fn execute(&self, job: &PersistJob) -> Result<(), String> {
        execute_job(
            job,
            self.persistence,
            self.instances,
            self.definitions,
            self.pending_user_tasks,
            self.pending_service_tasks,
            self.pending_timers,
            self.pending_message_catches,
        )
        .await
    }
}

/// Drains the queue after a shutdown signal with a hard time budget.
///
/// Each job gets exactly one attempt (no exponential backoff) so the budget can
/// be honored. Jobs that cannot be started before the deadline are counted as
/// `remaining`; their count is logged by the worker.
async fn drain_retry_queue(
    rx: &mut RetryQueueRx,
    budget: Duration,
    ctx: &DrainContext<'_>,
) -> DrainResult {
    let deadline = tokio::time::Instant::now() + budget;
    let mut result = DrainResult::default();

    while let Ok(job) = rx.try_recv() {
        // A duplicate shutdown carries no work.
        if matches!(job, PersistJob::Shutdown) {
            continue;
        }

        if tokio::time::Instant::now() >= deadline {
            result.remaining += 1;
            continue;
        }

        match tokio::time::timeout_at(deadline, ctx.execute(&job)).await {
            Ok(Ok(())) => result.processed += 1,
            Ok(Err(e)) => {
                result.failed += 1;
                ctx.error_counter
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                metrics::counter!("bpmn_persistence_shutdown_drain_failed_total").increment(1);
                tracing::error!("Shutdown drain failed for {}: {}", job, e);
            }
            Err(_) => {
                result.failed += 1;
                ctx.error_counter
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                metrics::counter!("bpmn_persistence_shutdown_drain_failed_total").increment(1);
                tracing::error!("Shutdown drain timed out for {}", job);
            }
        }
    }

    result
}

/// Executes a single persist job against the persistence backend.
///
/// For "Save" jobs, re-reads the current state from in-memory stores.
/// If the entity no longer exists in memory (e.g. instance was deleted),
/// the job is silently skipped (returns Ok).
#[allow(clippy::too_many_arguments)]
async fn execute_job(
    job: &PersistJob,
    persistence: &Arc<dyn WorkflowPersistence>,
    instances: &crate::engine::instance_store::InstanceStore,
    definitions: &crate::engine::registry::DefinitionRegistry,
    pending_user_tasks: &Arc<DashMap<Uuid, PendingUserTask>>,
    pending_service_tasks: &Arc<DashMap<Uuid, PendingServiceTask>>,
    pending_timers: &Arc<DashMap<Uuid, PendingTimer>>,
    pending_message_catches: &Arc<DashMap<Uuid, PendingMessageCatch>>,
) -> Result<(), String> {
    match job {
        PersistJob::SaveInstance(id) => {
            if let Some(inst_arc) = instances.get(id).await {
                let inst = inst_arc.read().await;
                persistence
                    .save_instance(&inst)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                // Instance was deleted since the job was queued — skip.
                Ok(())
            }
        }
        PersistJob::SaveDefinition(id) => {
            if let Some(def) = definitions.get(id) {
                persistence
                    .save_definition(&def)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        }
        PersistJob::SaveUserTask(id) => {
            if let Some(task_ref) = pending_user_tasks.get(id) {
                persistence
                    .save_user_task(&task_ref)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        }
        PersistJob::DeleteUserTask(id) => persistence
            .delete_user_task(*id)
            .await
            .map_err(|e| e.to_string()),
        PersistJob::SaveServiceTask(id) => {
            if let Some(task_ref) = pending_service_tasks.get(id) {
                persistence
                    .save_service_task(&task_ref)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        }
        PersistJob::DeleteServiceTask(id) => persistence
            .delete_service_task(*id)
            .await
            .map_err(|e| e.to_string()),
        PersistJob::SaveTimer(id) => {
            if let Some(timer_ref) = pending_timers.get(id) {
                persistence
                    .save_timer(&timer_ref)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        }
        PersistJob::DeleteTimer(id) => persistence
            .delete_timer(*id)
            .await
            .map_err(|e| e.to_string()),
        PersistJob::SaveMessageCatch(id) => {
            if let Some(catch_ref) = pending_message_catches.get(id) {
                persistence
                    .save_message_catch(&catch_ref)
                    .await
                    .map_err(|e| e.to_string())
            } else {
                Ok(())
            }
        }
        PersistJob::DeleteMessageCatch(id) => persistence
            .delete_message_catch(*id)
            .await
            .map_err(|e| e.to_string()),
        PersistJob::AppendHistoryEntry(entry) => persistence
            .append_history_entry(entry)
            .await
            .map_err(|e| e.to_string()),
        PersistJob::Shutdown => Ok(()),
    }
}

/// Ermittelt die betroffene Prozessinstanz eines Persistenz-Jobs.
///
/// `Delete*`-Jobs lassen sich nur zuordnen, solange der Eintrag noch in der
/// Pending-Map liegt; nach dem Entfernen wird der Job sauber übersprungen.
fn job_instance_id(
    job: &PersistJob,
    pending_user_tasks: &DashMap<Uuid, PendingUserTask>,
    pending_service_tasks: &DashMap<Uuid, PendingServiceTask>,
    pending_timers: &DashMap<Uuid, PendingTimer>,
    pending_message_catches: &DashMap<Uuid, PendingMessageCatch>,
) -> Option<Uuid> {
    match job {
        PersistJob::SaveInstance(id) => Some(*id),
        PersistJob::SaveDefinition(_) | PersistJob::Shutdown => None,
        PersistJob::AppendHistoryEntry(entry) => Some(entry.instance_id),
        PersistJob::SaveUserTask(id) | PersistJob::DeleteUserTask(id) => {
            pending_user_tasks.get(id).map(|task| task.instance_id)
        }
        PersistJob::SaveServiceTask(id) | PersistJob::DeleteServiceTask(id) => {
            pending_service_tasks.get(id).map(|task| task.instance_id)
        }
        PersistJob::SaveTimer(id) | PersistJob::DeleteTimer(id) => {
            pending_timers.get(id).map(|timer| timer.instance_id)
        }
        PersistJob::SaveMessageCatch(id) | PersistJob::DeleteMessageCatch(id) => {
            pending_message_catches
                .get(id)
                .map(|catch| catch.instance_id)
        }
    }
}

/// Hält einen endgültig fehlgeschlagenen Persistenz-Job als Incident an der
/// betroffenen Instanz fest. Existiert die Instanz nicht mehr, passiert nichts.
#[allow(clippy::too_many_arguments)]
async fn record_exhausted_job(
    job: &PersistJob,
    error: &str,
    instances: &crate::engine::instance_store::InstanceStore,
    pending_user_tasks: &Arc<DashMap<Uuid, PendingUserTask>>,
    pending_service_tasks: &Arc<DashMap<Uuid, PendingServiceTask>>,
    pending_timers: &Arc<DashMap<Uuid, PendingTimer>>,
    pending_message_catches: &Arc<DashMap<Uuid, PendingMessageCatch>>,
) {
    let Some(instance_id) = job_instance_id(
        job,
        pending_user_tasks,
        pending_service_tasks,
        pending_timers,
        pending_message_catches,
    ) else {
        return;
    };

    super::incidents::record_incident_on(
        instances,
        instance_id,
        IncidentKind::Persistence,
        None,
        None,
        format!("Persistence job '{job}' failed after {MAX_RETRIES} retries"),
        Some(error.to_string()),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persist_job_display() {
        let id = Uuid::nil();
        assert_eq!(
            PersistJob::SaveInstance(id).to_string(),
            "SaveInstance(00000000-0000-0000-0000-000000000000)"
        );
        assert_eq!(
            PersistJob::DeleteServiceTask(id).to_string(),
            "DeleteServiceTask(00000000-0000-0000-0000-000000000000)"
        );
        assert_eq!(PersistJob::Shutdown.to_string(), "Shutdown");
    }

    #[tokio::test]
    async fn test_retry_worker_shutdown() {
        let (tx, rx) = create_retry_queue();
        let persistence = Arc::new(crate::adapter::in_memory::InMemoryPersistence::new());
        let instances = crate::engine::instance_store::InstanceStore::new();
        let definitions = crate::engine::registry::DefinitionRegistry::new();
        let pending_user_tasks = Arc::new(DashMap::new());
        let pending_service_tasks = Arc::new(DashMap::new());
        let pending_timers = Arc::new(DashMap::new());
        let pending_message_catches = Arc::new(DashMap::new());
        let error_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let handle = spawn_retry_worker(
            rx,
            persistence,
            instances,
            definitions,
            pending_user_tasks,
            pending_service_tasks,
            pending_timers,
            pending_message_catches,
            error_counter,
        );

        tx.send(PersistJob::Shutdown).await.unwrap();
        // Worker should exit cleanly
        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("worker did not shut down in time")
            .expect("worker panicked");
    }

    #[tokio::test]
    async fn test_retry_worker_skips_missing_instance() {
        let (tx, rx) = create_retry_queue();
        let persistence = Arc::new(crate::adapter::in_memory::InMemoryPersistence::new());
        let instances = crate::engine::instance_store::InstanceStore::new();
        let definitions = crate::engine::registry::DefinitionRegistry::new();
        let pending_user_tasks = Arc::new(DashMap::new());
        let pending_service_tasks = Arc::new(DashMap::new());
        let pending_timers = Arc::new(DashMap::new());
        let pending_message_catches = Arc::new(DashMap::new());
        let error_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let handle = spawn_retry_worker(
            rx,
            persistence,
            instances,
            definitions,
            pending_user_tasks,
            pending_service_tasks,
            pending_timers,
            pending_message_catches,
            error_counter.clone(),
        );

        // Send a SaveInstance for a non-existent ID — should succeed silently
        tx.send(PersistJob::SaveInstance(Uuid::new_v4()))
            .await
            .unwrap();
        tx.send(PersistJob::Shutdown).await.unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("worker did not shut down in time")
            .expect("worker panicked");

        // No permanent failures should have been recorded
        assert_eq!(error_counter.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_retry_queue_try_send_full_drops() {
        let (tx, _rx) = create_retry_queue_with_capacity(1);
        // Fill the single slot
        tx.try_send(PersistJob::SaveInstance(Uuid::new_v4()))
            .expect("first send should fit");
        let err = tx
            .try_send(PersistJob::SaveInstance(Uuid::new_v4()))
            .expect_err("second send must hit Full");
        assert!(matches!(err, mpsc::error::TrySendError::Full(_)));
    }

    #[tokio::test]
    async fn test_shutdown_drains_jobs_queued_behind_signal() {
        // Shutdown is queued before the work so the worker must drain afterwards.
        let (tx, rx) = create_retry_queue_with_capacity(16);
        let persistence = Arc::new(crate::adapter::in_memory::InMemoryPersistence::new());
        let instances = crate::engine::instance_store::InstanceStore::new();
        let definitions = crate::engine::registry::DefinitionRegistry::new();
        let pending_user_tasks = Arc::new(DashMap::new());
        let pending_service_tasks = Arc::new(DashMap::new());
        let pending_timers = Arc::new(DashMap::new());
        let pending_message_catches = Arc::new(DashMap::new());
        let error_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let instance_id = Uuid::new_v4();
        let task_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        for task_id in &task_ids {
            pending_user_tasks.insert(
                *task_id,
                PendingUserTask {
                    task_id: *task_id,
                    instance_id,
                    node_id: "ut".into(),
                    assignee: "alice".into(),
                    token_id: Uuid::new_v4(),
                    created_at: chrono::Utc::now(),
                    business_key: None,
                },
            );
        }

        tx.send(PersistJob::Shutdown).await.unwrap();
        for task_id in &task_ids {
            tx.send(PersistJob::SaveUserTask(*task_id)).await.unwrap();
        }

        let handle = spawn_retry_worker(
            rx,
            Arc::clone(&persistence) as Arc<dyn WorkflowPersistence>,
            instances,
            definitions,
            pending_user_tasks,
            pending_service_tasks,
            pending_timers,
            pending_message_catches,
            Arc::clone(&error_counter),
        );

        tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("worker did not shut down in time")
            .expect("worker panicked");

        let persisted = persistence.list_user_tasks().await.unwrap();
        assert_eq!(persisted.len(), task_ids.len());
        assert_eq!(error_counter.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_exhausted_persist_job_records_persistence_incident() {
        use crate::domain::{BpmnElement, IncidentKind, ProcessDefinitionBuilder};
        use crate::engine::WorkflowEngine;

        let engine = WorkflowEngine::new();
        let def = ProcessDefinitionBuilder::new("retry_incident")
            .node("start", BpmnElement::StartEvent)
            .node("end", BpmnElement::EndEvent)
            .flow("start", "end")
            .build()
            .unwrap();
        let (def_key, _) = engine.deploy_definition(def).await;
        let instance_id = engine.start_instance(def_key).await.unwrap();

        let instances = engine.instances.clone();
        let pending_user_tasks = Arc::new(DashMap::new());
        let pending_service_tasks = Arc::new(DashMap::new());
        let pending_timers = Arc::new(DashMap::new());
        let pending_message_catches = Arc::new(DashMap::new());

        record_exhausted_job(
            &PersistJob::SaveInstance(instance_id),
            "nats down",
            &instances,
            &pending_user_tasks,
            &pending_service_tasks,
            &pending_timers,
            &pending_message_catches,
        )
        .await;

        let incidents = engine.get_instance_incidents(instance_id).await.unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].kind, IncidentKind::Persistence);
        assert!(incidents[0].message.contains("SaveInstance"));
        assert_eq!(incidents[0].details.as_deref(), Some("nats down"));
        assert_eq!(incidents[0].occurrences, 1);

        // SaveUserTask lässt sich über die Pending-Map zuordnen und wird
        // mit demselben offenen Persistenz-Incident zusammengeführt.
        let task_id = Uuid::new_v4();
        pending_user_tasks.insert(
            task_id,
            PendingUserTask {
                task_id,
                instance_id,
                node_id: "ut".into(),
                assignee: "alice".into(),
                token_id: Uuid::new_v4(),
                created_at: chrono::Utc::now(),
                business_key: None,
            },
        );
        record_exhausted_job(
            &PersistJob::SaveUserTask(task_id),
            "nats still down",
            &instances,
            &pending_user_tasks,
            &pending_service_tasks,
            &pending_timers,
            &pending_message_catches,
        )
        .await;

        let incidents = engine.get_instance_incidents(instance_id).await.unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].occurrences, 2);

        // Unbekannte Instanz wird sauber übersprungen.
        record_exhausted_job(
            &PersistJob::SaveInstance(Uuid::new_v4()),
            "gone",
            &instances,
            &pending_user_tasks,
            &pending_service_tasks,
            &pending_timers,
            &pending_message_catches,
        )
        .await;
    }

    #[tokio::test]
    async fn test_drain_budget_exhaustion_counts_remaining_jobs() {
        let (tx, mut rx) = create_retry_queue_with_capacity(8);
        for _ in 0..3 {
            tx.try_send(PersistJob::SaveInstance(Uuid::new_v4()))
                .unwrap();
        }
        let persistence: Arc<dyn WorkflowPersistence> =
            Arc::new(crate::adapter::in_memory::InMemoryPersistence::new());
        let instances = crate::engine::instance_store::InstanceStore::new();
        let definitions = crate::engine::registry::DefinitionRegistry::new();
        let pending_user_tasks = Arc::new(DashMap::new());
        let pending_service_tasks = Arc::new(DashMap::new());
        let pending_timers = Arc::new(DashMap::new());
        let pending_message_catches = Arc::new(DashMap::new());
        let error_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        let ctx = DrainContext {
            persistence: &persistence,
            instances: &instances,
            definitions: &definitions,
            pending_user_tasks: &pending_user_tasks,
            pending_service_tasks: &pending_service_tasks,
            pending_timers: &pending_timers,
            pending_message_catches: &pending_message_catches,
            error_counter: &error_counter,
        };
        let drained = drain_retry_queue(&mut rx, Duration::ZERO, &ctx).await;

        assert_eq!(
            drained,
            DrainResult {
                processed: 0,
                failed: 0,
                remaining: 3,
            }
        );
    }
}
