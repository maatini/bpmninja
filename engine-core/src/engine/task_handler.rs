//! In-process service-task handlers.
//!
//! A handler registered for a topic runs inside the engine process. The
//! dispatcher locks those tasks with [`IN_PROCESS_WORKER_ID`] and applies the
//! handler's [`TaskOutcome`] through the same complete / fail / BPMN-error
//! path external workers use. Topics without a handler stay available to
//! external fetch-and-lock callers. A registered topic is never returned to
//! them, so a task is not executed twice.
//!
//! [`WorkflowEngine::shutdown`] stops the dispatcher, waits for invocations
//! that already started, then drains the persistence retry worker.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use serde_json::Value;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::AbortHandle;
use uuid::Uuid;

use crate::domain::EngineResult;
use crate::runtime::PendingServiceTask;

use super::WorkflowEngine;

/// Worker id under which the dispatcher locks tasks.
///
/// External `fetch_and_lock` calls do not receive handler topics, even when
/// they pass this id.
pub const IN_PROCESS_WORKER_ID: &str = "bpmninja-in-process";

/// Lock duration for an in-process invocation. Extended while the handler runs.
const IN_PROCESS_LOCK_SECONDS: i64 = 60;
/// How often a running invocation refreshes its lock.
const LOCK_EXTEND_INTERVAL: Duration = Duration::from_secs(20);
/// Upper bound for concurrently running handler invocations.
const MAX_CONCURRENT_HANDLERS: usize = 64;
/// Heartbeat so retry delays and expired locks are noticed without a signal.
const FALLBACK_TICK: Duration = Duration::from_secs(1);
/// Default time [`WorkflowEngine::shutdown`] waits for in-flight handlers.
const DEFAULT_SHUTDOWN_DRAIN_MS: u64 = 30_000;

/// Result of one in-process handler invocation.
#[derive(Debug, Clone)]
pub enum TaskOutcome {
    /// Completes the task. Variables are merged into the token.
    Complete { variables: HashMap<String, Value> },
    /// Technical failure.
    ///
    /// `retries: None` decrements the remaining count. `Some(0)` opens an
    /// incident and releases the lock. `retry_after` hides the task from the
    /// next fetch until the delay has elapsed; zero retries immediately.
    Failure {
        message: String,
        details: Option<String>,
        retries: Option<i32>,
        retry_after: Duration,
    },
    /// BPMN error, matched against an error boundary on the activity.
    ///
    /// Without a matching boundary the task becomes an incident. Variables are
    /// merged into the token before the boundary path runs.
    BpmnError {
        code: String,
        message: Option<String>,
        variables: HashMap<String, Value>,
    },
}

impl TaskOutcome {
    /// Successful completion without variables.
    pub fn success() -> Self {
        Self::Complete {
            variables: HashMap::new(),
        }
    }

    /// Successful completion, merging `variables` into the token.
    pub fn complete<I, K>(variables: I) -> Self
    where
        I: IntoIterator<Item = (K, Value)>,
        K: Into<String>,
    {
        Self::Complete {
            variables: variables
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        }
    }

    /// Technical failure. `retry_after` of zero makes the task fetchable again
    /// immediately when retries remain.
    pub fn failure(
        message: impl Into<String>,
        details: Option<String>,
        retries: Option<i32>,
        retry_after: Duration,
    ) -> Self {
        Self::Failure {
            message: message.into(),
            details,
            retries,
            retry_after,
        }
    }

    /// BPMN error with optional variables for the boundary path.
    pub fn bpmn_error<I, K>(code: impl Into<String>, message: Option<String>, variables: I) -> Self
    where
        I: IntoIterator<Item = (K, Value)>,
        K: Into<String>,
    {
        Self::BpmnError {
            code: code.into(),
            message,
            variables: variables
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        }
    }
}

/// Executes service tasks of one topic inside the engine process.
///
/// Handlers run on the Tokio runtime. CPU-heavy work belongs in
/// `tokio::task::spawn_blocking`. A panic is reported as a failure and does
/// not stop the engine. Do not complete, fail, or throw a BPMN error for the
/// same task from inside `handle`; return a [`TaskOutcome`] instead.
#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    /// Handles one locked service task.
    async fn handle(&self, task: &PendingServiceTask) -> TaskOutcome;
}

/// Aborts a nested handler task when the parent invocation is cancelled.
struct AbortOnDrop(AbortHandle);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct InFlightGuard {
    engine: Arc<WorkflowEngine>,
    task_id: Uuid,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.engine.in_process_inflight.remove(&self.task_id);
    }
}

struct RunningHandlers {
    joins: Vec<tokio::task::JoinHandle<()>>,
}

impl RunningHandlers {
    async fn reap(&mut self) {
        let pending = std::mem::take(&mut self.joins);
        for handle in pending {
            if handle.is_finished() {
                if let Err(err) = handle.await {
                    tracing::error!("In-process task handler task failed: {err}");
                }
            } else {
                self.joins.push(handle);
            }
        }
    }

    async fn join_with_budget(&mut self, budget: Duration) {
        let deadline = tokio::time::Instant::now() + budget;
        loop {
            self.reap().await;
            if self.joins.is_empty() {
                return;
            }
            if tokio::time::Instant::now() >= deadline {
                let left = self.joins.len();
                tracing::error!(
                    "In-process task handler shutdown budget exhausted; aborting {left} handler(s)"
                );
                for handle in self.joins.drain(..) {
                    handle.abort();
                }
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}

fn shutdown_drain_budget() -> Duration {
    let millis = std::env::var("TASK_HANDLER_SHUTDOWN_DRAIN_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SHUTDOWN_DRAIN_MS);
    Duration::from_millis(millis)
}

fn panic_outcome(err: &tokio::task::JoinError) -> TaskOutcome {
    TaskOutcome::failure(
        "In-process task handler panicked",
        Some(err.to_string()),
        None,
        Duration::ZERO,
    )
}

impl WorkflowEngine {
    /// Registers or replaces the in-process handler for `topic`.
    ///
    /// Returns `true` when a previous handler was replaced. An empty topic is
    /// rejected. Already pending tasks of the topic are picked up once
    /// [`Self::spawn_task_handler_dispatcher`] is running. External workers
    /// stop receiving the topic until [`Self::unregister_task_handler`].
    pub fn register_task_handler(
        &self,
        topic: impl Into<String>,
        handler: Arc<dyn TaskHandler>,
    ) -> bool {
        let topic = topic.into();
        if topic.is_empty() {
            tracing::warn!("Refusing in-process task handler with an empty topic");
            return false;
        }
        let replaced = self.task_handlers.insert(topic, handler).is_some();
        self.notify_service_tasks_available();
        replaced
    }

    /// Removes the handler for `topic`.
    ///
    /// Returns whether a handler was registered. An invocation that already
    /// holds the task runs to completion; later tasks become available to
    /// external workers.
    pub fn unregister_task_handler(&self, topic: &str) -> bool {
        let removed = self.task_handlers.remove(topic).is_some();
        if removed {
            self.notify_service_tasks_available();
        }
        removed
    }

    /// Starts the background dispatcher. Idempotent while one is running.
    ///
    /// [`Self::shutdown`] stops it and waits for invocations that have already
    /// started. Calling this again after shutdown starts a fresh dispatcher.
    pub async fn spawn_task_handler_dispatcher(self: &Arc<Self>) {
        let mut slot = self.task_handler_handle.lock().await;
        if slot.as_ref().is_some_and(|handle| !handle.is_finished()) {
            tracing::warn!("In-process task handler dispatcher is already running");
            return;
        }
        let _ = self.task_handler_shutdown.send(false);
        let engine = Arc::clone(self);
        *slot = Some(tokio::spawn(async move {
            engine.run_task_handler_dispatcher().await;
        }));
        tracing::info!("In-process task handler dispatcher started");
    }

    /// Stops the dispatcher and waits for in-flight handlers.
    ///
    /// The wait is bounded by `TASK_HANDLER_SHUTDOWN_DRAIN_MS` (default 30s).
    /// Past that budget the remaining invocations are aborted. Also invoked
    /// from [`Self::shutdown`] before the persistence retry worker drains.
    ///
    /// The dispatcher slot is held for the wait so a second dispatcher cannot
    /// start mid-shutdown. Handlers must not call this method or
    /// [`Self::spawn_task_handler_dispatcher`]: both need that slot.
    pub async fn shutdown_task_handler_dispatcher(&self) {
        let mut slot = self.task_handler_handle.lock().await;
        let _ = self.task_handler_shutdown.send(true);
        if let Some(handle) = slot.take() {
            tracing::info!("Waiting for in-process task handlers to finish");
            if let Err(err) = handle.await {
                tracing::error!("In-process task handler dispatcher failed: {err}");
            }
        }
        let _ = self.task_handler_shutdown.send(false);
    }

    /// Wakes the in-process dispatcher and every long-polling `fetchAndLock`.
    ///
    /// `notify_waiters` does not store a permit. Both loops enable the waiter
    /// before they fetch, so a task that arrives between the fetch and the
    /// sleep is not missed, and one that arrived earlier is still in the index.
    /// `notify_one` would leave every other waiter asleep.
    pub(crate) fn notify_service_tasks_available(&self) {
        self.service_task_notify.notify_waiters();
    }

    /// Completes when service tasks may have become fetchable.
    ///
    /// Create the future and call [`tokio::sync::futures::Notified::enable`]
    /// before fetching. Lock expiry and retry delays are time-based and are
    /// not signalled here.
    pub(crate) fn service_tasks_changed(&self) -> tokio::sync::futures::Notified<'_> {
        self.service_task_notify.notified()
    }

    async fn run_task_handler_dispatcher(self: Arc<Self>) {
        let mut shutdown_rx = self.task_handler_shutdown.subscribe();
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_HANDLERS));
        let mut running = RunningHandlers { joins: Vec::new() };
        tracing::info!("In-process task handler dispatcher loop running");

        loop {
            if *shutdown_rx.borrow() {
                break;
            }
            running.reap().await;

            let notified = self.service_task_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            if permits.available_permits() == 0 {
                tokio::select! {
                    biased;
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    permit = permits.acquire() => {
                        match permit {
                            Ok(permit) => drop(permit),
                            Err(_) => break,
                        }
                    }
                }
                continue;
            }

            let topics: Vec<String> = self
                .task_handlers
                .iter()
                .map(|entry| entry.key().clone())
                .collect();
            let capacity = permits.available_permits();
            let tasks = if topics.is_empty() {
                Vec::new()
            } else {
                self.fetch_and_lock_in_process(
                    IN_PROCESS_WORKER_ID,
                    capacity,
                    &topics,
                    IN_PROCESS_LOCK_SECONDS,
                )
                .await
            };

            if *shutdown_rx.borrow() {
                for task in tasks {
                    self.release_in_process_lock(task.id).await;
                }
                break;
            }

            if tasks.is_empty() {
                let wait_for = self.next_in_process_wait(&topics);
                tokio::select! {
                    biased;
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = &mut notified => {}
                    _ = tokio::time::sleep(wait_for) => {}
                }
                continue;
            }

            for task in tasks {
                let Some(handler) = self
                    .task_handlers
                    .get(&task.topic)
                    .map(|entry| Arc::clone(entry.value()))
                else {
                    self.release_in_process_lock(task.id).await;
                    continue;
                };
                let Ok(permit) = Arc::clone(&permits).try_acquire_owned() else {
                    self.release_in_process_lock(task.id).await;
                    continue;
                };
                self.spawn_handler_invocation(&mut running, handler, task, permit);
            }
        }

        running.join_with_budget(shutdown_drain_budget()).await;
        tracing::info!("In-process task handler dispatcher stopped");
    }

    fn spawn_handler_invocation(
        self: &Arc<Self>,
        running: &mut RunningHandlers,
        handler: Arc<dyn TaskHandler>,
        task: PendingServiceTask,
        permit: OwnedSemaphorePermit,
    ) {
        self.in_process_inflight.insert(task.id);
        let engine = Arc::clone(self);
        running.joins.push(tokio::spawn(async move {
            let _permit = permit;
            let _guard = InFlightGuard {
                engine: Arc::clone(&engine),
                task_id: task.id,
            };
            engine.execute_task_handler(handler, task).await;
        }));
    }

    async fn execute_task_handler(
        self: &Arc<Self>,
        handler: Arc<dyn TaskHandler>,
        task: PendingServiceTask,
    ) {
        let task_id = task.id;
        let outcome = self.invoke_handler(handler, task).await;
        let engine = Arc::clone(self);
        let apply = tokio::spawn(async move { engine.apply_task_outcome(task_id, outcome).await });
        let _cancel_apply = AbortOnDrop(apply.abort_handle());
        match apply.await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                tracing::warn!(
                    "In-process handler result for service task {task_id} was not applied: {err}"
                );
            }
            Err(err) if err.is_cancelled() => {
                tracing::warn!(
                    "Applying in-process handler result for service task {task_id} was cancelled"
                );
            }
            Err(err) => {
                tracing::error!(
                    "Applying in-process handler result for service task {task_id} panicked: {err}"
                );
            }
        }
    }

    async fn invoke_handler(
        self: &Arc<Self>,
        handler: Arc<dyn TaskHandler>,
        task: PendingServiceTask,
    ) -> TaskOutcome {
        let engine = Arc::clone(self);
        let task_id = task.id;
        let invocation = tokio::spawn(async move {
            let handle_fut = handler.handle(&task);
            tokio::pin!(handle_fut);
            loop {
                tokio::select! {
                    biased;
                    outcome = &mut handle_fut => return outcome,
                    _ = tokio::time::sleep(LOCK_EXTEND_INTERVAL) => {
                        if let Err(err) = engine
                            .extend_lock(task_id, IN_PROCESS_WORKER_ID, IN_PROCESS_LOCK_SECONDS)
                            .await
                        {
                            tracing::debug!(
                                "Could not extend in-process lock for service task {task_id}: {err}"
                            );
                        }
                    }
                }
            }
        });
        let _cancel_handler = AbortOnDrop(invocation.abort_handle());
        match invocation.await {
            Ok(outcome) => outcome,
            Err(err) if err.is_cancelled() => TaskOutcome::failure(
                "In-process task handler was cancelled",
                Some(err.to_string()),
                None,
                Duration::from_secs(1),
            ),
            Err(err) => panic_outcome(&err),
        }
    }

    async fn apply_task_outcome(&self, task_id: Uuid, outcome: TaskOutcome) -> EngineResult<()> {
        match outcome {
            TaskOutcome::Complete { variables } => {
                self.complete_service_task(task_id, IN_PROCESS_WORKER_ID, variables)
                    .await
            }
            TaskOutcome::Failure {
                message,
                details,
                retries,
                retry_after,
            } => {
                self.fail_service_task_with_retry_timeout(
                    task_id,
                    IN_PROCESS_WORKER_ID,
                    retries,
                    Some(message),
                    details,
                    retry_after,
                )
                .await
            }
            TaskOutcome::BpmnError {
                code,
                message,
                variables,
            } => {
                self.handle_bpmn_error_with_variables(
                    task_id,
                    IN_PROCESS_WORKER_ID,
                    &code,
                    variables,
                    message,
                )
                .await
            }
        }
    }

    async fn release_in_process_lock(&self, task_id: Uuid) {
        let released = {
            let Some(mut task) = self.pending_service_tasks.get_mut(&task_id) else {
                return;
            };
            if task.worker_id.as_deref() != Some(IN_PROCESS_WORKER_ID) {
                return;
            }
            task.worker_id = None;
            task.lock_expiration = None;
            true
        };
        if released {
            self.persist_service_task(task_id).await;
            self.notify_service_tasks_available();
        }
    }

    fn next_in_process_wait(&self, topics: &[String]) -> Duration {
        let now = Utc::now();
        let mut soonest: Option<Duration> = None;
        for topic in topics {
            let ids: Vec<Uuid> = {
                let Some(set) = self.service_task_topic_index.get(topic) else {
                    continue;
                };
                set.iter().copied().collect()
            };
            for id in ids {
                if self.in_process_inflight.contains(&id) {
                    continue;
                }
                let Some(task) = self.pending_service_tasks.get(&id) else {
                    continue;
                };
                if task.retries <= 0 {
                    continue;
                }
                let Some(expiration) = task.lock_expiration else {
                    continue;
                };
                if expiration <= now {
                    continue;
                }
                let Ok(wait) = (expiration - now).to_std() else {
                    continue;
                };
                let wait = wait + Duration::from_millis(5);
                soonest = Some(match soonest {
                    Some(current) if current <= wait => current,
                    _ => wait,
                });
            }
        }
        soonest.unwrap_or(FALLBACK_TICK).min(FALLBACK_TICK)
    }
}
