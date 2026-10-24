//! Service task operations (Camunda-style fetch-and-lock pattern).
//!
//! These are `impl WorkflowEngine` methods extracted into a separate file
//! for maintainability. The public API is unchanged.

use std::collections::HashMap;
use std::sync::Arc;

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
    pub async fn fetch_and_lock_service_tasks_with_durations(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[(String, i64)],
    ) -> Vec<PendingServiceTask> {
        let now = Utc::now();
        let mut result = Vec::new();
        let mut to_persist = Vec::new();

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

            // Skip tasks that are already locked and not expired
            if let Some(expiration) = task.lock_expiration {
                if expiration > now {
                    continue;
                }
                // Lock expired — release it
                tracing::info!("Service task {}: lock expired, releasing", task.id);
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
        let mi_element_variable = match def.get_node(&task.node_id) {
            Some(BpmnElement::ServiceTask {
                multi_instance: Some(mi),
                ..
            }) => mi.element_variable.clone(),
            _ => None,
        };

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
        for (k, v) in variables {
            token.variables.insert(k, v);
        }

        // Vor dem letzten Item: nur Fortschritt fortschreiben, Token behalten.
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
                inst.variables = token.variables.clone();
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
        self.cancel_boundary_message_catches(instance_id, &task.node_id)
            .await;

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
            inst.variables = token.variables.clone();
        }

        // Run end scripts
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
            crate::scripting::run_end_scripts(
                &self.script_config,
                instance_id,
                &mut token,
                &def,
                &task.node_id,
                audit_log,
                variables,
            )
            .await?;
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

    /// Reports a failure for an service task.
    ///
    /// Decrements retries. When retries reach 0, the task becomes an incident.
    pub async fn fail_service_task(
        &self,
        task_id: Uuid,
        worker_id: &str,
        retries: Option<i32>,
        error_message: Option<String>,
        error_details: Option<String>,
    ) -> EngineResult<()> {
        let instance_id = {
            let mut task = self
                .pending_service_tasks
                .get_mut(&task_id)
                .ok_or(EngineError::ServiceTaskNotFound(task_id))?;

            // Verify lock ownership
            verify_lock_ownership(task_id, &task.worker_id, worker_id)?;

            // Update retries
            let new_retries = retries.unwrap_or(task.retries - 1);
            task.retries = new_retries;
            task.error_message = error_message.clone();
            task.error_details = error_details.clone();

            // Release the lock so it can be retried (or becomes incident)
            task.worker_id = None;
            task.lock_expiration = None;

            let instance_id = task.instance_id;
            let node_id = task.node_id.clone();

            if new_retries <= 0 {
                metrics::counter!("bpmn_errors_total", "type" => "incident").increment(1);
                // Incident: log and record on the instance
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
                    metrics::counter!("bpmn_incidents_total", "kind" => "service_task")
                        .increment(1);
                }
                tracing::warn!("Service task {task_id}: incident created (retries exhausted)");
            } else {
                tracing::info!(
                    "Service task {task_id}: failed, {} retries remaining",
                    new_retries
                );
            }
            instance_id
        };

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
        self.cancel_boundary_message_catches(instance_id, &task.node_id)
            .await;

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
        self.cancel_boundary_message_catches(instance_id, &task.node_id)
            .await;

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
                None,
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
