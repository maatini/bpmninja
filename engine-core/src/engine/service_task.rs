//! Service task operations (Camunda-style fetch-and-lock pattern).
//!
//! These are `impl WorkflowEngine` methods extracted into a separate file
//! for maintainability. The public API is unchanged.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeDelta, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{BpmnElement, EngineError, EngineResult, IncidentKind};

use super::WorkflowEngine;
use super::topic_index;
use crate::runtime::{InstanceState, PendingServiceTask};

/// Lower bound for a service-task lock duration in seconds.
const MIN_LOCK_SECONDS: i64 = 1;
/// Upper bound for a service-task lock duration in seconds (1 h).
const MAX_LOCK_SECONDS: i64 = 3_600;
/// Duration used when a requested topic carries no explicit duration.
const FALLBACK_LOCK_SECONDS: i64 = 30;
/// Upper bound for a service-task retry delay. Larger values are capped.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// Who is allowed to lock a topic in this fetch.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ServiceTaskFetchAudience {
    /// External worker. Topics with an in-process handler are skipped.
    ExternalWorker,
    /// In-process dispatcher. Only topics with a registered handler are locked.
    InProcessDispatcher,
}

/// `lock_expiration` for a retry delay. `None` means the task is fetchable immediately.
///
/// The delay reuses the existing lock-expiration field with `worker_id == None`,
/// which `fetch_and_lock` already treats as "not yet available".
fn retry_lock_expiration(retry_after: Duration) -> Option<chrono::DateTime<Utc>> {
    if retry_after.is_zero() {
        return None;
    }
    let capped = retry_after.min(MAX_RETRY_AFTER);
    let millis = i64::try_from(capped.as_millis()).ok()?;
    let delta = TimeDelta::try_milliseconds(millis)?;
    Utc::now().checked_add_signed(delta)
}

/// Upper bound for long-polling fetch-and-lock (`asyncResponseTimeout`).
///
/// Callers that ask for longer are clamped. Zero still performs a single fetch
/// and returns, which keeps the previous non-blocking default.
pub const MAX_SERVICE_TASK_LONG_POLL: std::time::Duration = std::time::Duration::from_secs(30);

/// Converts a requested lock duration (seconds) into a safe `TimeDelta`.
///
/// Values are clamped to [`MIN_LOCK_SECONDS`]..=[`MAX_LOCK_SECONDS`] because
/// `TimeDelta`/`DateTime` arithmetic panics on extreme inputs.
fn lock_time_delta(seconds: i64) -> TimeDelta {
    TimeDelta::try_seconds(seconds.clamp(MIN_LOCK_SECONDS, MAX_LOCK_SECONDS))
        .unwrap_or(TimeDelta::seconds(MAX_LOCK_SECONDS))
}

/// Verifies that the given worker holds the lock on an service task.
///
/// Returns `Ok(())` if `locked_worker` matches `worker_id`.
/// Returns an error if the task is locked by a different worker or not locked at all.
fn verify_lock_ownership(
    task_id: Uuid,
    locked_worker: &Option<String>,
    worker_id: &str,
) -> EngineResult<()> {
    match locked_worker {
        Some(locked_by) if locked_by != worker_id => Err(EngineError::ServiceTaskLocked {
            task_id,
            worker_id: locked_by.clone(),
        }),
        None => Err(EngineError::ServiceTaskNotLocked(task_id)),
        _ => Ok(()),
    }
}

impl WorkflowEngine {
    /// Fetches and locks service tasks matching the requested topics.
    ///
    /// Returns up to `max_tasks` unlocked tasks whose topic appears in
    /// `topics`. Each returned task is locked for `lock_duration` seconds
    /// and assigned to `worker_id`.
    pub async fn fetch_and_lock_service_tasks(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[String],
        lock_duration: i64,
    ) -> Vec<PendingServiceTask> {
        let topics: Vec<(String, i64)> = topics
            .iter()
            .map(|topic| (topic.clone(), lock_duration))
            .collect();
        self.fetch_and_lock_service_tasks_with_durations(worker_id, max_tasks, &topics)
            .await
    }

    /// Like [`Self::fetch_and_lock_service_tasks`], but with a lock duration
    /// (in seconds) per topic.
    ///
    /// Topics not present in the slice are ignored; duplicate names keep the
    /// last duration. Durations are clamped to 1..=3600 s.
    ///
    /// Topics with a registered in-process handler are reserved for the
    /// dispatcher and are not locked here, including when `worker_id` is
    /// [`IN_PROCESS_WORKER_ID`](super::IN_PROCESS_WORKER_ID).
    pub async fn fetch_and_lock_service_tasks_with_durations(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[(String, i64)],
    ) -> Vec<PendingServiceTask> {
        self.fetch_and_lock_inner(
            worker_id,
            max_tasks,
            topics,
            ServiceTaskFetchAudience::ExternalWorker,
        )
        .await
    }

    pub(crate) async fn fetch_and_lock_in_process(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[String],
        lock_duration: i64,
    ) -> Vec<PendingServiceTask> {
        let topics: Vec<(String, i64)> = topics
            .iter()
            .map(|topic| (topic.clone(), lock_duration))
            .collect();
        self.fetch_and_lock_inner(
            worker_id,
            max_tasks,
            &topics,
            ServiceTaskFetchAudience::InProcessDispatcher,
        )
        .await
    }

    async fn fetch_and_lock_inner(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[(String, i64)],
        audience: ServiceTaskFetchAudience,
    ) -> Vec<PendingServiceTask> {
        let now = Utc::now();
        let mut result = Vec::new();
        let mut to_persist = Vec::new();
        let handlers = Arc::clone(&self.task_handlers);
        let inflight = Arc::clone(&self.in_process_inflight);

        let durations: HashMap<&str, i64> = topics
            .iter()
            .map(|(name, seconds)| (name.as_str(), *seconds))
            .collect();
        let topic_names: Vec<String> = topics.iter().map(|(name, _)| name.clone()).collect();

        let candidates = topic_index::candidate_ids(&self.service_task_topic_index, &topic_names);

        for (topic, task_id) in candidates {
            if result.len() >= max_tasks {
                break;
            }

            let Some(mut task) = self.pending_service_tasks.get_mut(&task_id) else {
                topic_index::remove(&self.service_task_topic_index, &topic, task_id);
                continue;
            };

            // Incidents stay in the map until retry/resolve
            if task.retries <= 0 {
                continue;
            }

            // A running in-process invocation keeps the task even after its lock
            // duration elapses, so the dispatcher cannot execute it twice.
            if inflight.contains(&task_id) {
                continue;
            }

            let reserved = handlers.contains_key(task.topic.as_str());
            let claimable = match audience {
                ServiceTaskFetchAudience::ExternalWorker => !reserved,
                ServiceTaskFetchAudience::InProcessDispatcher => reserved,
            };
            if !claimable {
                continue;
            }

            // A future expiration hides the task. That is either an active worker
            // lock or a retry delay stored without a worker id.
            if let Some(expiration) = task.lock_expiration {
                if expiration > now {
                    continue;
                }
                if task.worker_id.is_some() {
                    tracing::info!("Service task {}: lock expired, releasing", task.id);
                }
            }

            let lock = lock_time_delta(
                durations
                    .get(topic.as_str())
                    .copied()
                    .unwrap_or(FALLBACK_LOCK_SECONDS),
            );

            // Lock the task
            task.worker_id = Some(worker_id.to_string());
            task.lock_expiration = Some(now + lock);

            tracing::info!(
                "Service task {} locked by worker '{}' for {}s",
                task.id,
                worker_id,
                lock.num_seconds()
            );

            result.push(task.clone());
            to_persist.push(task.id);
        }

        for id in to_persist {
            self.persist_service_task(id).await;
        }

        result
    }

    /// Long-polling variant of [`Self::fetch_and_lock_service_tasks_with_durations`].
    ///
    /// Returns as soon as at least one task is locked, or an empty vec once
    /// `timeout` elapses. `timeout` is clamped to [`MAX_SERVICE_TASK_LONG_POLL`].
    /// Waiters are woken when a matching task is inserted or unlocked.
    /// `fallback_tick` covers lock expiries, which are time-based and not signalled.
    /// A zero timeout performs one fetch and returns (backwards compatible).
    ///
    /// The notification is enabled before each fetch so a task that arrives
    /// between the fetch and the sleep is not missed.
    pub async fn fetch_and_lock_service_tasks_long_poll(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[(String, i64)],
        timeout: std::time::Duration,
        fallback_tick: std::time::Duration,
    ) -> Vec<PendingServiceTask> {
        let timeout = timeout.min(MAX_SERVICE_TASK_LONG_POLL);
        let deadline = tokio::time::Instant::now() + timeout;
        // A zero tick would busy-loop until the deadline. Lock expiry is
        // coarse (whole seconds), so a short positive tick is enough.
        let fallback = if fallback_tick.is_zero() {
            std::time::Duration::from_millis(500)
        } else {
            fallback_tick
        };

        loop {
            let notified = self.service_tasks_changed();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let tasks = self
                .fetch_and_lock_service_tasks_with_durations(worker_id, max_tasks, topics)
                .await;
            let now = tokio::time::Instant::now();
            if !tasks.is_empty() || now >= deadline {
                return tasks;
            }

            let wake_at = now + (deadline - now).min(fallback);
            tokio::select! {
                _ = &mut notified => {}
                _ = tokio::time::sleep_until(wake_at) => {}
            }
        }
    }

    /// ID des nächsten noch offenen Tasks derselben Multi-Instance-Gruppe.
    fn next_open_multi_instance_task_id(
        &self,
        instance_id: Uuid,
        node_id: &str,
        token_id: Uuid,
        exclude: Uuid,
    ) -> Option<Uuid> {
        self.pending_service_tasks
            .iter()
            .find(|entry| {
                entry.instance_id == instance_id
                    && entry.node_id == node_id
                    && entry.token_id == token_id
                    && entry.id != exclude
            })
            .map(|entry| entry.id)
    }

    /// Completes an service task, advancing the process instance.
    ///
    /// The task must be locked by `worker_id`. Optional variables are merged.
    pub async fn complete_service_task(
        &self,
        task_id: Uuid,
        worker_id: &str,
        variables: HashMap<String, Value>,
    ) -> EngineResult<()> {
        // Atomically verify lock ownership and remove the task in one shard-lock
        // window to prevent TOCTOU race conditions between concurrent workers.
        let task = self.remove_pending_service_task_if(&task_id, |_, t| {
            t.worker_id.as_deref() == Some(worker_id)
        });

        let task = match task {
            Some((_, task)) => task,
            None => {
                // Distinguish "not locked", "wrong worker", and "not found"
                if let Some(task_ref) = self.pending_service_tasks.get(&task_id) {
                    return match &task_ref.worker_id {
                        None => Err(EngineError::ServiceTaskNotLocked(task_id)),
                        Some(locked_by) => Err(EngineError::ServiceTaskLocked {
                            task_id,
                            worker_id: locked_by.clone(),
                        }),
                    };
                }
                return Err(EngineError::ServiceTaskNotFound(task_id));
            }
        };

        let instance_id = task.instance_id;

        // Reject if instance is suspended — reinsert the task on rejection
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let inst = inst_arc.read().await;
            if matches!(inst.state, crate::runtime::InstanceState::Suspended { .. }) {
                self.insert_pending_service_task(task);
                return Err(EngineError::InstanceSuspended(instance_id));
            }
        }

        let old_state = if let Some(lk) = self.instances.get(&instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };

        // Definition und MI-Fortschritt lesen, bevor der Token entfernt wird.
        let def_key = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let inst = inst_arc.read().await;
            inst.definition_key
        };
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;
        let def = Arc::clone(&def);

        let mi_progress = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let inst = inst_arc.read().await;
            inst.multi_instance_state.get(&task.node_id).cloned()
        };
        let is_multi_instance = mi_progress.is_some();
        let is_last_item = mi_progress
            .as_ref()
            .is_some_and(|p| p.completed.saturating_add(1) >= p.total);
        let mi_definition = match def.get_node(&task.node_id) {
            Some(BpmnElement::ServiceTask {
                multi_instance: Some(mi),
                ..
            }) => Some(mi.clone()),
            _ => None,
        };
        let is_sequential =
            is_multi_instance && mi_definition.as_ref().is_some_and(|mi| mi.is_sequential);
        let mi_element_variable = mi_definition
            .as_ref()
            .and_then(|mi| mi.element_variable.clone());

        // Retrieve token from central store and merge variables. Bei MI bleibt
        // der Token bis zum letzten Item in der Instanz.
        let mut token = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            let mut token = if is_multi_instance && !is_last_item {
                inst.tokens.get(&task.token_id).cloned()
            } else {
                inst.tokens.remove(&task.token_id)
            }
            .ok_or_else(|| {
                EngineError::InvalidDefinition(format!(
                    "Token {} not found in instance",
                    task.token_id
                ))
            })?;

            if is_multi_instance {
                // Item-Variablen des abgeschlossenen Tasks in den zentralen Token mergen.
                if let Some(loop_counter) = task.variables_snapshot.get("loopCounter") {
                    token
                        .variables
                        .insert("loopCounter".to_string(), loop_counter.clone());
                }
                if let Some(element_variable) = &mi_element_variable
                    && let Some(value) = task.variables_snapshot.get(element_variable)
                {
                    token
                        .variables
                        .insert(element_variable.clone(), value.clone());
                }
            }

            token
        };
        let token_for_rollback = token.clone();
        let task_for_rollback = task.clone();
        if is_multi_instance {
            if let Err(error) = crate::engine::data_mappings::publish_multi_instance_outputs(
                &mut token,
                &def,
                &task.node_id,
                &task.variables_snapshot,
                &variables,
            ) {
                self.restore_mapped_service_task(
                    instance_id,
                    token_for_rollback,
                    task_for_rollback,
                    old_state.as_ref(),
                )
                .await;
                return Err(error);
            }
        } else {
            for (k, v) in variables {
                token.variables.insert(k, v);
            }
        }

        // Sequentielles MI: nach jedem Item genau einen Nachfolge-Task
        // anlegen; der Token bleibt bis zum letzten Item in der Instanz.
        if is_sequential && !is_last_item {
            let (completed, total, items) = mi_progress
                .as_ref()
                .map(|p| (p.completed.saturating_add(1), p.total, p.items.clone()))
                .unwrap_or_else(|| (0, 0, Vec::new()));
            let next_item = items.get(completed).cloned().flatten();
            let mut variables_snapshot = token.variables.clone();
            variables_snapshot.insert(
                "loopCounter".to_string(),
                Value::from(completed.saturating_add(1) as u64),
            );
            if let (Some(element_variable), Some(value)) = (&mi_element_variable, next_item) {
                variables_snapshot.insert(element_variable.clone(), value);
            }
            variables_snapshot = match crate::engine::data_mappings::apply_inputs(
                &def,
                &task.node_id,
                variables_snapshot,
            ) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    self.restore_mapped_service_task(
                        instance_id,
                        token_for_rollback,
                        task_for_rollback,
                        old_state.as_ref(),
                    )
                    .await;
                    return Err(error);
                }
            };
            let business_key = old_state.as_ref().and_then(|inst| {
                (!inst.business_key.is_empty()).then(|| inst.business_key.clone())
            });
            let next_task = PendingServiceTask {
                id: Uuid::new_v4(),
                instance_id,
                definition_key: def.key,
                node_id: task.node_id.clone(),
                topic: task.topic.clone(),
                token_id: token.id,
                variables_snapshot,
                created_at: Utc::now(),
                worker_id: None,
                lock_expiration: None,
                retries: 3,
                error_message: None,
                error_details: None,
                business_key,
            };

            {
                let inst_arc = self
                    .instances
                    .get(&instance_id)
                    .await
                    .ok_or(EngineError::NoSuchInstance(instance_id))?;
                let mut inst = inst_arc.write().await;
                inst.variables = token.process_variables();
                inst.tokens.insert(token.id, token);
                if let Some(progress) = inst.multi_instance_state.get_mut(&task.node_id) {
                    progress.completed = progress.completed.saturating_add(1);
                }
                inst.state = InstanceState::WaitingOnServiceTask {
                    task_id: next_task.id,
                };
                inst.push_audit_log(format!(
                    "✅ Sequential multi-instance service task '{}' item {completed}/{total} completed by worker '{}'",
                    task.node_id, worker_id
                ));
            }

            self.insert_pending_service_task(next_task.clone());
            self.persist_service_task(next_task.id).await;

            tracing::info!(
                "Instance {}: completed sequential multi-instance service task '{}' item {completed}/{total}; next task {} (loopCounter {})",
                instance_id,
                task.node_id,
                next_task.id,
                completed.saturating_add(1)
            );

            self.remove_persisted_service_task(task_id).await;
            metrics::counter!("bpmn_tasks_completed_total", "type" => "service").increment(1);

            self.record_history_event(
                instance_id,
                crate::history::HistoryEventType::TaskCompleted,
                &format!(
                    "Sequential multi-instance service task '{}' item {completed}/{total} completed",
                    task.node_id
                ),
                crate::history::ActorType::ServiceWorker,
                Some(worker_id.to_string()),
                old_state.as_ref(),
            )
            .await;

            self.persist_instance(instance_id).await;
            return Ok(());
        }

        // Paralleles MI vor dem letzten Item: nur Fortschritt fortschreiben,
        // Token behalten.
        if is_multi_instance && !is_last_item {
            let (completed, total) = mi_progress
                .as_ref()
                .map(|p| (p.completed.saturating_add(1), p.total))
                .unwrap_or((0, 0));
            let next_task_id = self
                .next_open_multi_instance_task_id(
                    instance_id,
                    &task.node_id,
                    task.token_id,
                    task_id,
                )
                .unwrap_or(task_id);

            {
                let inst_arc = self
                    .instances
                    .get(&instance_id)
                    .await
                    .ok_or(EngineError::NoSuchInstance(instance_id))?;
                let mut inst = inst_arc.write().await;
                inst.variables = token.process_variables();
                inst.tokens.insert(token.id, token);
                if let Some(progress) = inst.multi_instance_state.get_mut(&task.node_id) {
                    progress.completed = progress.completed.saturating_add(1);
                }
                inst.state = InstanceState::WaitingOnServiceTask {
                    task_id: next_task_id,
                };
                inst.push_audit_log(format!(
                    "✅ Multi-instance service task '{}' item {completed}/{total} completed by worker '{}'",
                    task.node_id, worker_id
                ));
            }

            tracing::info!(
                "Instance {}: completed multi-instance service task '{}' item {completed}/{total} (task_id: {task_id})",
                instance_id,
                task.node_id
            );

            self.remove_persisted_service_task(task_id).await;
            metrics::counter!("bpmn_tasks_completed_total", "type" => "service").increment(1);

            self.record_history_event(
                instance_id,
                crate::history::HistoryEventType::TaskCompleted,
                &format!(
                    "Multi-instance service task '{}' item {completed}/{total} completed",
                    task.node_id
                ),
                crate::history::ActorType::ServiceWorker,
                Some(worker_id.to_string()),
                old_state.as_ref(),
            )
            .await;

            self.persist_instance(instance_id).await;
            return Ok(());
        }

        self.cancel_boundary_timers(instance_id, &task.node_id)
            .await;
        self.cancel_boundary_waits(instance_id, &task.node_id).await;

        tracing::info!(
            "Instance {}: completed service task '{}' (task_id: {task_id})",
            instance_id,
            task.node_id
        );

        {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            if is_multi_instance {
                inst.multi_instance_state.remove(&task.node_id);
            }
            inst.push_audit_log(format!(
                "✅ Service task '{}' completed by worker '{}'",
                task.node_id, worker_id
            ));
            if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                inst.state = InstanceState::Running;
            }
            inst.variables = token.process_variables();
        }

        // Run end scripts. Mapping- und Scriptfehler lassen den Task stehen.
        {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            let crate::ProcessInstance {
                audit_log,
                variables,
                ..
            } = &mut *inst;
            if let Err(error) = crate::scripting::run_end_scripts(
                &self.script_config,
                instance_id,
                &mut token,
                &def,
                &task.node_id,
                audit_log,
                variables,
            )
            .await
            {
                drop(inst);
                self.restore_mapped_service_task(
                    instance_id,
                    token_for_rollback,
                    task_for_rollback,
                    old_state.as_ref(),
                )
                .await;
                return Err(error);
            }
        }

        let next =
            crate::engine::executor::resolve_next_target(&def, &task.node_id, &token.variables)?;

        token.current_node = next.clone();
        // Update instance current_node so UI highlights correctly
        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        {
            let mut inst = inst_arc.write().await;
            inst.current_node = next;
        }

        self.remove_persisted_service_task(task_id).await;

        metrics::counter!("bpmn_tasks_completed_total", "type" => "service").increment(1);

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::TaskCompleted,
            &format!("Service task '{}' completed", task.node_id),
            crate::history::ActorType::ServiceWorker,
            Some(worker_id.to_string()),
            old_state.as_ref(),
        )
        .await;

        self.run_instance_batch(instance_id, token).await
    }

    /// Legt Task und Token nach einem fehlgeschlagenen Abschluss wieder zurück.
    ///
    /// Die Aktivierung (`data_scope`) bleibt die des Stands vor diesem Versuch,
    /// damit Inputs nicht erneut angewendet werden.
    async fn restore_mapped_service_task(
        &self,
        instance_id: Uuid,
        token: crate::domain::Token,
        task: PendingServiceTask,
        previous: Option<&crate::runtime::ProcessInstance>,
    ) {
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let mut inst = inst_arc.write().await;
            if let Some(previous) = previous {
                inst.variables = previous.variables.clone();
                inst.state = previous.state.clone();
            }
            inst.tokens.insert(token.id, token);
        }
        self.insert_pending_service_task(task);
    }

    /// Reports a failure for an service task.
    ///
    /// Decrements retries. When retries reach 0, the task becomes an incident.
    /// The task is unlocked and can be fetched again immediately.
    pub async fn fail_service_task(
        &self,
        task_id: Uuid,
        worker_id: &str,
        retries: Option<i32>,
        error_message: Option<String>,
        error_details: Option<String>,
    ) -> EngineResult<()> {
        self.fail_service_task_with_retry_timeout(
            task_id,
            worker_id,
            retries,
            error_message,
            error_details,
            Duration::ZERO,
        )
        .await
    }

    /// Like [`Self::fail_service_task`], but hides the task from fetch until
    /// `retry_after` has elapsed when retries remain.
    ///
    /// The deadline is stored in `lock_expiration` without a worker id. A zero
    /// delay matches [`Self::fail_service_task`]. Values above 24h are capped.
    /// An incident (`retries <= 0`) is never left locked.
    pub async fn fail_service_task_with_retry_timeout(
        &self,
        task_id: Uuid,
        worker_id: &str,
        retries: Option<i32>,
        error_message: Option<String>,
        error_details: Option<String>,
        retry_after: Duration,
    ) -> EngineResult<()> {
        let (instance_id, node_id, new_retries, error_message, error_details) = {
            let mut task = self
                .pending_service_tasks
                .get_mut(&task_id)
                .ok_or(EngineError::ServiceTaskNotFound(task_id))?;

            verify_lock_ownership(task_id, &task.worker_id, worker_id)?;

            let new_retries = retries.unwrap_or(task.retries - 1);
            task.retries = new_retries;
            task.error_message = error_message.clone();
            task.error_details = error_details.clone();
            task.worker_id = None;
            task.lock_expiration = if new_retries <= 0 {
                None
            } else {
                retry_lock_expiration(retry_after)
            };

            (
                task.instance_id,
                task.node_id.clone(),
                new_retries,
                error_message,
                error_details,
            )
        };

        if new_retries > 0 {
            self.notify_service_tasks_available();
        }

        if new_retries <= 0 {
            metrics::counter!("bpmn_errors_total", "type" => "incident").increment(1);
            if let Some(inst_arc) = self.instances.get(&instance_id).await {
                let mut inst = inst_arc.write().await;
                let msg = error_message
                    .clone()
                    .unwrap_or_else(|| "Unknown error".into());
                inst.push_audit_log(format!(
                    "🚨 INCIDENT: Service task '{}' failed with 0 retries — {}",
                    node_id, msg
                ));
                inst.push_or_touch_incident(
                    IncidentKind::ServiceTask,
                    Some(node_id.clone()),
                    Some(task_id),
                    msg,
                    error_details.clone(),
                );
                metrics::counter!("bpmn_incidents_total", "kind" => "service_task").increment(1);
            }
            tracing::warn!("Service task {task_id}: incident created (retries exhausted)");
        } else {
            tracing::info!(
                "Service task {task_id}: failed, {} retries remaining",
                new_retries
            );
        }

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::Error,
            &format!("Service task '{}' failed", task_id),
            crate::history::ActorType::ServiceWorker,
            Some(worker_id.to_string()),
            None, // State variables didn't fundamentally change, but it's an error record
        )
        .await;

        self.persist_service_task(task_id).await;
        self.persist_instance(instance_id).await;

        Ok(())
    }

    /// Retries an incident (a service task with retries <= 0).
    ///
    /// Resets the retry counter and clears the error fields so that a worker
    /// can pick up the task again via fetch-and-lock.
    pub async fn retry_incident(
        &self,
        task_id: Uuid,
        new_retries: Option<i32>,
    ) -> EngineResult<()> {
        let instance_id = {
            let mut task = self
                .pending_service_tasks
                .get_mut(&task_id)
                .ok_or(EngineError::ServiceTaskNotFound(task_id))?;

            if task.retries > 0 {
                return Err(EngineError::InvalidDefinition(
                    "Task is not an incident (retries > 0)".into(),
                ));
            }

            let retries = new_retries.unwrap_or(3);
            task.retries = retries;
            task.error_message = None;
            task.error_details = None;
            task.worker_id = None;
            task.lock_expiration = None;

            let instance_id = task.instance_id;
            let node_id = task.node_id.clone();

            tracing::info!("Service task {task_id}: incident retried with {retries} retries");

            // Update audit log on the instance
            if let Some(inst_arc) = self.instances.get(&instance_id).await {
                let mut inst = inst_arc.write().await;
                inst.push_audit_log(format!(
                    "🔄 Incident on service task '{}' retried ({} retries)",
                    node_id, retries
                ));
            }

            instance_id
        };

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::TaskCompleted,
            &format!("Incident retried (task {})", task_id),
            crate::history::ActorType::User,
            None,
            None,
        )
        .await;

        self.persist_service_task(task_id).await;
        self.persist_instance(instance_id).await;
        self.notify_service_tasks_available();

        Ok(())
    }

    /// Resolves an incident by manually completing the service task.
    ///
    /// Unlike `complete_service_task`, this does not require a worker lock.
    /// The token is advanced to the next node as if a worker had completed it.
    pub async fn resolve_incident(
        &self,
        task_id: Uuid,
        variables: HashMap<String, Value>,
    ) -> EngineResult<()> {
        // Atomically verify incident state (retries <= 0) and remove to prevent TOCTOU
        let task = self.remove_pending_service_task_if(&task_id, |_, t| t.retries <= 0);

        let task = match task {
            Some((_, task)) => task,
            None => {
                if self.pending_service_tasks.contains_key(&task_id) {
                    return Err(EngineError::InvalidDefinition(
                        "Task is not an incident (retries > 0)".into(),
                    ));
                }
                return Err(EngineError::ServiceTaskNotFound(task_id));
            }
        };
        let instance_id = task.instance_id;

        let old_state = if let Some(lk) = self.instances.get(&instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };

        // Retrieve token from central store and merge variables
        let mut token = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.tokens.remove(&task.token_id).ok_or_else(|| {
                EngineError::InvalidDefinition(format!(
                    "Token {} not found in instance",
                    task.token_id
                ))
            })?
        };
        for (k, v) in variables {
            token.variables.insert(k, v);
        }

        self.cancel_boundary_timers(instance_id, &task.node_id)
            .await;
        self.cancel_boundary_waits(instance_id, &task.node_id).await;

        tracing::info!(
            "Instance {}: incident on service task '{}' resolved manually (task_id: {task_id})",
            instance_id,
            task.node_id
        );

        let def_key = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.push_audit_log(format!(
                "✅ Incident on service task '{}' resolved manually",
                task.node_id
            ));

            if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                inst.state = InstanceState::Running;
            }
            inst.current_node = task.node_id.clone();
            inst.definition_key
        };

        // Advance token to the next node
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;

        self.run_end_scripts(instance_id, &mut token, &def, &task.node_id)
            .await?;

        // Register compensation handler if this activity has a BoundaryCompensationEvent
        self.register_compensation_handler(instance_id, &task.node_id, &def)
            .await;

        let next =
            crate::engine::executor::resolve_next_target(&def, &task.node_id, &token.variables)?;

        token.current_node = next.clone();
        {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.current_node = next;
        }

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::TaskCompleted,
            &format!(
                "Incident on service task '{}' resolved manually",
                task.node_id
            ),
            crate::history::ActorType::User,
            None,
            old_state.as_ref(),
        )
        .await;

        self.remove_persisted_service_task(task_id).await;

        // Continue running
        self.run_instance_batch(instance_id, token).await
    }

    /// Extends the lock on an service task.
    pub async fn extend_lock(
        &self,
        task_id: Uuid,
        worker_id: &str,
        additional_duration: i64,
    ) -> EngineResult<()> {
        {
            let mut task = self
                .pending_service_tasks
                .get_mut(&task_id)
                .ok_or(EngineError::ServiceTaskNotFound(task_id))?;

            verify_lock_ownership(task_id, &task.worker_id, worker_id)?;

            let lock = lock_time_delta(additional_duration);
            task.lock_expiration = Some(Utc::now() + lock);

            tracing::info!(
                "Service task {task_id}: lock extended by {}s",
                lock.num_seconds()
            );
        }

        self.persist_service_task(task_id).await;

        Ok(())
    }

    /// Handles a BPMN error for a service task.
    ///
    /// A matching error boundary removes the task and continues the token.
    /// Without a boundary the task is re-inserted as an incident (`retries = 0`).
    pub async fn handle_bpmn_error(
        &self,
        task_id: Uuid,
        worker_id: &str,
        error_code: &str,
    ) -> EngineResult<()> {
        self.handle_bpmn_error_with_variables(task_id, worker_id, error_code, HashMap::new(), None)
            .await
    }

    /// Like [`Self::handle_bpmn_error`], merging `variables` into the token first.
    ///
    /// `error_message` is written to the audit log and, when no boundary catches
    /// the error, stored as the incident details.
    pub async fn handle_bpmn_error_with_variables(
        &self,
        task_id: Uuid,
        worker_id: &str,
        error_code: &str,
        variables: HashMap<String, Value>,
        error_message: Option<String>,
    ) -> EngineResult<()> {
        // Atomically verify lock ownership and remove to prevent TOCTOU
        let task = self.remove_pending_service_task_if(&task_id, |_, t| {
            t.worker_id.as_deref() == Some(worker_id)
        });

        let task = match task {
            Some((_, task)) => task,
            None => {
                if let Some(task_ref) = self.pending_service_tasks.get(&task_id) {
                    return match &task_ref.worker_id {
                        None => Err(EngineError::ServiceTaskNotLocked(task_id)),
                        Some(locked_by) => Err(EngineError::ServiceTaskLocked {
                            task_id,
                            worker_id: locked_by.clone(),
                        }),
                    };
                }
                return Err(EngineError::ServiceTaskNotFound(task_id));
            }
        };
        let instance_id = task.instance_id;

        if (!variables.is_empty() || error_message.is_some())
            && let Some(inst_arc) = self.instances.get(&instance_id).await
        {
            let mut inst = inst_arc.write().await;
            if let Some(message) = &error_message {
                inst.push_audit_log(format!("BPMN error detail: {message}"));
            }
            if let Some(token) = inst.tokens.get_mut(&task.token_id) {
                for (key, value) in &variables {
                    token.variables.insert(key.clone(), value.clone());
                }
            }
            for (key, value) in &variables {
                inst.variables.insert(key.clone(), value.clone());
            }
        }

        let def_key = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let inst = inst_arc.read().await;
            inst.definition_key
        };

        self.cancel_boundary_timers(instance_id, &task.node_id)
            .await;
        self.cancel_boundary_waits(instance_id, &task.node_id).await;

        let target_boundary = if let Some(def) = self.definitions.get(&def_key) {
            crate::engine::executor::find_boundary_error_event(&def, &task.node_id, error_code)
        } else {
            None
        };

        if let Some(boundary_id) = target_boundary {
            let old_state = if let Some(lk) = self.instances.get(&instance_id).await {
                Some(lk.read().await.clone())
            } else {
                None
            };
            {
                let inst_arc = self
                    .instances
                    .get(&instance_id)
                    .await
                    .ok_or(EngineError::NoSuchInstance(instance_id))?;
                let mut inst = inst_arc.write().await;
                inst.push_audit_log(format!(
                    "💥 BPMN Error '{error_code}' caught by boundary event '{boundary_id}'"
                ));
                inst.state = InstanceState::Running;
            }

            self.record_history_event(
                instance_id,
                crate::history::HistoryEventType::TokenAdvanced,
                &format!("Error '{error_code}' caught"),
                crate::history::ActorType::Engine,
                None,
                old_state.as_ref(),
            )
            .await;

            // Retrieve token from central store
            let mut token = {
                let inst_arc = self
                    .instances
                    .get(&instance_id)
                    .await
                    .ok_or(EngineError::NoSuchInstance(instance_id))?;
                let mut inst = inst_arc.write().await;
                inst.tokens.remove(&task.token_id).ok_or_else(|| {
                    EngineError::InvalidDefinition(format!(
                        "Token {} not found in instance",
                        task.token_id
                    ))
                })?
            };
            token.discard_data_scope();
            for (key, value) in &variables {
                token.variables.insert(key.clone(), value.clone());
            }
            let def = self
                .definitions
                .get(&def_key)
                .ok_or(EngineError::NoSuchDefinition(def_key))?;
            let next =
                crate::engine::executor::resolve_next_target(&def, &boundary_id, &token.variables)?;

            token.current_node = next.clone();
            {
                let inst_arc = self
                    .instances
                    .get(&instance_id)
                    .await
                    .ok_or(EngineError::NoSuchInstance(instance_id))?;
                let mut inst = inst_arc.write().await;
                inst.current_node = next;
                inst.variables = token.process_variables();
            }

            self.remove_persisted_service_task(task_id).await;
            self.persist_instance(instance_id).await;
            self.run_instance_batch(instance_id, token).await?;
            return Ok(());
        }

        // Unhandled BPMN error: re-insert as incident so the token is not a zombie.
        let mut incident = task;
        incident.retries = 0;
        incident.worker_id = None;
        incident.lock_expiration = None;
        incident.error_message = Some(format!("Unhandled BPMN error '{error_code}'"));
        if let Some(message) = &error_message {
            incident.error_details = Some(message.clone());
        }

        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let mut inst = inst_arc.write().await;
            inst.push_audit_log(format!(
                "🚨 BPMN error '{}' thrown by worker '{}' at service task '{}' (No boundary event caught it)",
                error_code, worker_id, incident.node_id
            ));
            inst.push_or_touch_incident(
                IncidentKind::ServiceTask,
                Some(incident.node_id.clone()),
                Some(task_id),
                format!("Unhandled BPMN error '{error_code}'"),
                error_message.clone(),
            );
            metrics::counter!("bpmn_incidents_total", "kind" => "service_task").increment(1);
        }

        tracing::warn!(
            "Service task {task_id}: unhandled BPMN error '{error_code}' from worker '{worker_id}'"
        );

        self.insert_pending_service_task(incident);
        self.persist_service_task(task_id).await;
        self.persist_instance(instance_id).await;

        Ok(())
    }
}
