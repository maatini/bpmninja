//! In-process service-task handlers (E1).
//!
//! Handlers registered per topic are executed inside the engine process by a
//! dispatcher that uses the regular fetch-and-lock path with a reserved worker
//! ID. Locks, retries, incidents, BPMN errors and persistence therefore behave
//! exactly as for external workers; topics without a handler stay available to
//! REST/Camunda workers.

use crate::engine::WorkflowEngine;
use crate::runtime::PendingServiceTask;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Worker ID under which the dispatcher locks tasks.
pub const IN_PROCESS_WORKER_ID: &str = "bpmninja-in-process";

/// Lock duration for in-process tasks. A handler running longer loses its lock.
const LOCK_DURATION_SECS: i64 = 300;
/// Upper bound for concurrently running handler invocations.
const MAX_CONCURRENT_HANDLERS: usize = 64;
/// Covers lock expiries and retry delays, which are time-based and not signalled.
const FALLBACK_TICK: Duration = Duration::from_secs(1);

/// Result of an in-process handler invocation.
#[derive(Debug, Clone)]
pub enum TaskOutcome {
    /// Completes the task; variables are merged into the token.
    Complete(HashMap<String, Value>),
    /// Reports a failure. `retries: None` decrements the remaining retries;
    /// `retry_timeout_ms` delays the next attempt.
    Fail {
        message: String,
        details: Option<String>,
        retries: Option<i32>,
        retry_timeout_ms: i64,
    },
    /// Throws a BPMN error (matched against error boundary events).
    BpmnError {
        code: String,
        message: Option<String>,
        variables: HashMap<String, Value>,
    },
}

/// Executes service tasks of one topic inside the engine process.
///
/// Handlers run on the Tokio runtime; CPU-heavy work belongs in
/// `tokio::task::spawn_blocking`. A panicking handler counts as a failure.
#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    async fn handle(&self, task: &PendingServiceTask) -> TaskOutcome;
}

impl WorkflowEngine {
    /// Registers (or replaces) the in-process handler for `topic`.
    ///
    /// Takes effect once [`Self::spawn_task_handler_dispatcher`] runs.
    pub fn register_task_handler(&self, topic: impl Into<String>, handler: Arc<dyn TaskHandler>) {
        self.task_handlers.insert(topic.into(), handler);
        // Wake the dispatcher so already pending tasks of the topic are picked up.
        self.notify_service_tasks_available();
    }

    /// Removes the handler for `topic`. Tasks already running are finished.
    pub fn unregister_task_handler(&self, topic: &str) -> bool {
        self.task_handlers.remove(topic).is_some()
    }

    /// Starts the background dispatcher for in-process handlers.
    ///
    /// The task holds an `Arc` to the engine until it is aborted.
    pub fn spawn_task_handler_dispatcher(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let engine = Arc::clone(self);
        tokio::spawn(async move { engine.run_task_handler_dispatcher().await })
    }

    async fn run_task_handler_dispatcher(self: Arc<Self>) {
        let permits = Arc::new(Semaphore::new(MAX_CONCURRENT_HANDLERS));
        loop {
            // Register interest before fetching so no notification is lost.
            let notified = self.service_tasks_changed();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let capacity = permits.available_permits();
            if capacity == 0 {
                // Wait until a running handler finishes.
                if permits.acquire().await.is_err() {
                    return;
                }
                continue;
            }

            // Snapshot: a task is executed by the handler registered when it was locked.
            let handlers: HashMap<String, Arc<dyn TaskHandler>> = self
                .task_handlers
                .iter()
                .map(|e| (e.key().clone(), Arc::clone(e.value())))
                .collect();
            let tasks = if handlers.is_empty() {
                Vec::new()
            } else {
                let topics: Vec<String> = handlers.keys().cloned().collect();
                self.fetch_and_lock_service_tasks(
                    IN_PROCESS_WORKER_ID,
                    capacity,
                    &topics,
                    LOCK_DURATION_SECS,
                )
                .await
            };

            if tasks.is_empty() {
                tokio::select! {
                    _ = &mut notified => {}
                    _ = tokio::time::sleep(FALLBACK_TICK) => {}
                }
                continue;
            }

            for task in tasks {
                let Some(handler) = handlers.get(&task.topic).map(Arc::clone) else {
                    continue;
                };
                let Ok(permit) = Arc::clone(&permits).try_acquire_owned() else {
                    // Unreachable: only this loop acquires permits and fetched ≤ capacity.
                    continue;
                };
                let engine = Arc::clone(&self);
                tokio::spawn(async move {
                    engine.execute_task_handler(handler, task).await;
                    drop(permit);
                });
            }
        }
    }

    async fn execute_task_handler(&self, handler: Arc<dyn TaskHandler>, task: PendingServiceTask) {
        let task_id = task.id;
        // Separate task so a panic is reported as a failure instead of leaving the lock.
        let outcome = match tokio::spawn(async move { handler.handle(&task).await }).await {
            Ok(outcome) => outcome,
            Err(e) => TaskOutcome::Fail {
                message: "In-process task handler panicked".into(),
                details: Some(e.to_string()),
                retries: None,
                retry_timeout_ms: 0,
            },
        };

        let result = match outcome {
            TaskOutcome::Complete(variables) => {
                self.complete_service_task(task_id, IN_PROCESS_WORKER_ID, variables)
                    .await
            }
            TaskOutcome::Fail {
                message,
                details,
                retries,
                retry_timeout_ms,
            } => {
                self.fail_service_task_with_retry_timeout(
                    task_id,
                    IN_PROCESS_WORKER_ID,
                    retries,
                    Some(message),
                    details,
                    retry_timeout_ms,
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
                    HashMap::new(),
                    message,
                )
                .await
            }
        };
        if let Err(e) = result {
            tracing::warn!("In-process handler result for service task {task_id} rejected: {e}");
        }
    }
}
