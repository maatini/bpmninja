//! Service task operations (Camunda-style fetch-and-lock pattern).
//!
//! These are `impl WorkflowEngine` methods extracted into a separate file
//! for maintainability. The public API is unchanged.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{TimeDelta, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{EngineError, EngineResult};

use super::WorkflowEngine;
use super::topic_index;
use crate::runtime::{InstanceState, PendingServiceTask};

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
    /// Long-polling variant of [`Self::fetch_and_lock_service_tasks`].
    ///
    /// Returns as soon as at least one task is locked, or empty once `timeout`
    /// elapses. Waiters are woken immediately when tasks become fetchable;
    /// `fallback_tick` only covers lock expiries, which are time-based and
    /// not signalled.
    pub async fn fetch_and_lock_service_tasks_long_poll(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[String],
        lock_duration: i64,
        timeout: std::time::Duration,
        fallback_tick: std::time::Duration,
    ) -> Vec<PendingServiceTask> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            // Register interest before fetching so no notification is lost.
            let notified = self.service_tasks_changed();
            tokio::pin!(notified);
            notified.as_mut().enable();

            let tasks = self
                .fetch_and_lock_service_tasks(worker_id, max_tasks, topics, lock_duration)
                .await;
            let now = tokio::time::Instant::now();
            if !tasks.is_empty() || now >= deadline {
                return tasks;
            }

            let wait = (deadline - now).min(fallback_tick);
            tokio::select! {
                _ = &mut notified => {}
                _ = tokio::time::sleep(wait) => {}
            }
        }
    }

    pub async fn fetch_and_lock_service_tasks(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[String],
        lock_duration: i64,
    ) -> Vec<PendingServiceTask> {
        self.fetch_and_lock_service_tasks_for(
            worker_id,
            max_tasks,
            topics,
            TimeDelta::seconds(lock_duration),
        )
        .await
    }

    /// Millisecond variant for transports whose contract requires exact lock durations.
    pub async fn fetch_and_lock_service_tasks_ms(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[String],
        lock_duration_ms: i64,
    ) -> Vec<PendingServiceTask> {
        self.fetch_and_lock_service_tasks_for(
            worker_id,
            max_tasks,
            topics,
            TimeDelta::milliseconds(lock_duration_ms),
        )
        .await
    }

    async fn fetch_and_lock_service_tasks_for(
        &self,
        worker_id: &str,
        max_tasks: usize,
        topics: &[String],
        lock_duration: TimeDelta,
    ) -> Vec<PendingServiceTask> {
        let now = Utc::now();
        let Some(expires_at) = now.checked_add_signed(lock_duration) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        let mut to_persist = Vec::new();

        let candidates = topic_index::candidate_ids(&self.service_task_topic_index, topics);

        for (topic, task_id) in candidates {
            if result.len() >= max_tasks {
                break;
            }

            let instance_id = self
                .pending_service_tasks
                .get(&task_id)
                .map(|task| task.instance_id);
            if let Some(instance_id) = instance_id
                && let Some(instance) = self.instances.get(&instance_id).await
                && matches!(instance.read().await.state, InstanceState::Suspended { .. })
            {
                continue;
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

            // Lock the task
            task.worker_id = Some(worker_id.to_string());
            task.lock_expiration = Some(expires_at);

            tracing::info!(
                "Service task {} locked by worker '{}' for {}s",
                task.id,
                worker_id,
                lock_duration.num_seconds()
            );

            result.push(task.clone());
            to_persist.push(task.id);
        }

        for id in to_persist {
            self.persist_service_task(id).await;
        }

        result
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
        self.complete_service_task_with_types(task_id, worker_id, variables, HashMap::new())
            .await
    }

    pub async fn complete_service_task_with_types(
        &self,
        task_id: Uuid,
        worker_id: &str,
        variables: HashMap<String, Value>,
        variable_types: HashMap<String, String>,
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
        let updated_names: Vec<String> = variables.keys().cloned().collect();

        // Retrieve token from central store and merge variables
        let mut token = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            let retrieved = inst.tokens.remove(&task.token_id);
            drop(inst);
            if let Some(token) = retrieved {
                token
            } else {
                self.remove_persisted_service_task(task_id).await;
                return Err(EngineError::AlreadyCompleted);
            }
        };
        let original_token = token.clone();
        for (k, v) in variables {
            token.variables.insert(k, v);
        }

        tracing::info!(
            "Instance {}: completed service task '{}' (task_id: {task_id})",
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
            if !crate::engine::event_subprocess::token_valid(&inst, &token) {
                drop(inst);
                self.remove_persisted_service_task(task_id).await;
                return Err(EngineError::AlreadyCompleted);
            }
            inst.push_audit_log(format!(
                "✅ Service task '{}' completed by worker '{}'",
                task.node_id, worker_id
            ));
            if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                inst.state = InstanceState::Running;
            }
            if !crate::engine::event_subprocess::token_valid(&inst, &token) {
                drop(inst);
                self.remove_persisted_service_task(task_id).await;
                return Err(EngineError::AlreadyCompleted);
            }
            if token.event_context.is_none() {
                inst.variables = token.process_variables();
            }
            for name in updated_names {
                inst.variable_types.remove(&name);
            }
            inst.variable_types.extend(variable_types);
            inst.definition_key
        };

        // Advance token to the next node
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;
        let def = Arc::clone(&def);

        if let Err(error) = self
            .run_end_scripts(instance_id, &mut token, &def, &task.node_id)
            .await
        {
            if let Some(inst_arc) = self.instances.get(&instance_id).await {
                let mut inst = inst_arc.write().await;
                if !crate::engine::event_subprocess::token_valid(&inst, &original_token) {
                    drop(inst);
                    self.remove_persisted_service_task(task_id).await;
                    return Err(EngineError::AlreadyCompleted);
                }
                if let Some(old) = &old_state {
                    inst.state = old.state.clone();
                    // Event-context completions commit only after all scripts succeed.
                    // Their failure must not roll back unrelated concurrent globals.
                    if original_token.event_context.is_none() {
                        inst.variables = old.variables.clone();
                    }
                    inst.variable_types = old.variable_types.clone();
                }
                inst.tokens.insert(original_token.id, original_token);
            }
            self.insert_pending_service_task(task);
            self.persist_instance(instance_id).await;
            return Err(error);
        }

        self.cancel_boundary_timers(instance_id, &task.node_id, Some(task.token_id))
            .await;
        self.cancel_boundary_message_catches(instance_id, &task.node_id, Some(task.token_id))
            .await;

        self.run_conditional_work(instance_id).await?;
        if let Some(instance) = self.instances.get(&instance_id).await
            && !crate::engine::event_subprocess::token_valid(&*instance.read().await, &token)
        {
            return Ok(());
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
        self.emit_task_changed(instance_id, task_id).await;

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
        self.fail_service_task_with_retry_timeout(
            task_id,
            worker_id,
            retries,
            error_message,
            error_details,
            0,
        )
        .await
    }

    /// Stores the retry deadline in the existing persisted lock-expiration field.
    /// No timer or in-memory-only scheduling state is needed after a restart.
    pub async fn fail_service_task_with_retry_timeout(
        &self,
        task_id: Uuid,
        worker_id: &str,
        retries: Option<i32>,
        error_message: Option<String>,
        error_details: Option<String>,
        retry_timeout_ms: i64,
    ) -> EngineResult<()> {
        if retry_timeout_ms < 0 {
            return Err(EngineError::InvalidDefinition(
                "retry timeout must not be negative".into(),
            ));
        }
        let retry_at = if retry_timeout_ms == 0 {
            None
        } else {
            Some(
                Utc::now()
                    .checked_add_signed(TimeDelta::milliseconds(retry_timeout_ms))
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition("retry timeout exceeds date range".into())
                    })?,
            )
        };
        let (instance_id, node_id, new_retries) = {
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
            task.lock_expiration = retry_at;

            (task.instance_id, task.node_id.clone(), new_retries)
        }; // DashMap guard released before any `.await`.
        // Lock released: the task is fetchable again (unless it became an incident).
        self.notify_service_tasks_available();

        if new_retries <= 0 {
            metrics::counter!("bpmn_errors_total", "type" => "incident").increment(1);
            // Incident: log and record on the instance
            if let Some(inst_arc) = self.instances.get(&instance_id).await {
                let mut inst = inst_arc.write().await;
                let msg = error_message.unwrap_or_else(|| "Unknown error".into());
                inst.push_audit_log(format!(
                    "🚨 INCIDENT: Service task '{}' failed with 0 retries — {}",
                    node_id, msg
                ));
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
        self.run_conditional_work(instance_id).await?;

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
        let (instance_id, node_id, retries) = {
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

            (task.instance_id, task.node_id.clone(), retries)
        }; // DashMap guard released before any `.await`.
        // Incident reset: the task is fetchable again.
        self.notify_service_tasks_available();

        tracing::info!("Service task {task_id}: incident retried with {retries} retries");

        // Update audit log on the instance
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let mut inst = inst_arc.write().await;
            inst.push_audit_log(format!(
                "🔄 Incident on service task '{}' retried ({} retries)",
                node_id, retries
            ));
        }

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
        self.run_conditional_work(instance_id).await?;

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
        let updated_names: Vec<_> = variables.keys().cloned().collect();

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
            let retrieved = inst.tokens.remove(&task.token_id);
            drop(inst);
            if let Some(token) = retrieved {
                token
            } else {
                self.remove_persisted_service_task(task_id).await;
                return Err(EngineError::AlreadyCompleted);
            }
        };
        let original_token = token.clone();
        for (k, v) in variables {
            token.variables.insert(k, v);
        }

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
            for name in updated_names {
                inst.variable_types.remove(&name);
            }
            inst.definition_key
        };

        // Advance token to the next node
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;

        if let Err(error) = self
            .run_end_scripts(instance_id, &mut token, &def, &task.node_id)
            .await
        {
            if let Some(inst_arc) = self.instances.get(&instance_id).await {
                let mut inst = inst_arc.write().await;
                if !crate::engine::event_subprocess::token_valid(&inst, &original_token) {
                    drop(inst);
                    self.remove_persisted_service_task(task_id).await;
                    return Err(EngineError::AlreadyCompleted);
                }
                if let Some(old) = &old_state {
                    inst.state = old.state.clone();
                    // Event-context completions commit only after all scripts succeed.
                    // Their failure must not roll back unrelated concurrent globals.
                    if original_token.event_context.is_none() {
                        inst.variables = old.variables.clone();
                    }
                }
                inst.tokens.insert(original_token.id, original_token);
            }
            self.insert_pending_service_task(task);
            self.persist_instance(instance_id).await;
            return Err(error);
        }

        // Register compensation handler if this activity has a BoundaryCompensationEvent
        self.register_compensation_handler(instance_id, &task.node_id, &def)
            .await;

        self.cancel_boundary_timers(instance_id, &task.node_id, Some(task.token_id))
            .await;
        self.cancel_boundary_message_catches(instance_id, &task.node_id, Some(task.token_id))
            .await;

        self.run_conditional_work(instance_id).await?;
        if let Some(instance) = self.instances.get(&instance_id).await
            && !crate::engine::event_subprocess::token_valid(&*instance.read().await, &token)
        {
            return Ok(());
        }
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
        self.emit_task_changed(instance_id, task_id).await;

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
        self.extend_lock_for(task_id, worker_id, TimeDelta::seconds(additional_duration))
            .await
    }

    pub async fn extend_lock_ms(
        &self,
        task_id: Uuid,
        worker_id: &str,
        duration_ms: i64,
    ) -> EngineResult<()> {
        self.extend_lock_for(task_id, worker_id, TimeDelta::milliseconds(duration_ms))
            .await
    }

    async fn extend_lock_for(
        &self,
        task_id: Uuid,
        worker_id: &str,
        additional_duration: TimeDelta,
    ) -> EngineResult<()> {
        let expires_at = Utc::now()
            .checked_add_signed(additional_duration)
            .ok_or_else(|| {
                EngineError::InvalidDefinition("lock duration exceeds date range".into())
            })?;
        {
            let mut task = self
                .pending_service_tasks
                .get_mut(&task_id)
                .ok_or(EngineError::ServiceTaskNotFound(task_id))?;

            verify_lock_ownership(task_id, &task.worker_id, worker_id)?;

            task.lock_expiration = Some(expires_at);

            tracing::info!(
                "Service task {task_id}: lock extended by {}ms",
                additional_duration.num_milliseconds()
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
        self.handle_bpmn_error_with_variables(
            task_id,
            worker_id,
            error_code,
            HashMap::new(),
            HashMap::new(),
            None,
        )
        .await
    }

    /// Applies worker variables to the token before the boundary path executes.
    pub async fn handle_bpmn_error_with_variables(
        &self,
        task_id: Uuid,
        worker_id: &str,
        error_code: &str,
        variables: HashMap<String, Value>,
        variable_types: HashMap<String, String>,
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

        if let Some(instance) = self.instances.get(&instance_id).await {
            let mut instance = instance.write().await;
            for name in variables.keys() {
                instance.variable_types.remove(name);
            }
            instance.variable_types.extend(variable_types);
            if let Some(message) = &error_message {
                instance.push_audit_log(format!("BPMN error detail: {message}"));
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

        self.cancel_boundary_timers(instance_id, &task.node_id, Some(task.token_id))
            .await;
        self.cancel_boundary_message_catches(instance_id, &task.node_id, Some(task.token_id))
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
                inst.tokens
                    .remove(&task.token_id)
                    .map(|mut token| {
                        Self::close_conditional_boundaries(&mut inst, &mut token);
                        token
                    })
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition(format!(
                            "Token {} not found in instance",
                            task.token_id
                        ))
                    })?
            };
            token.discard_data_scope();
            token.variables.extend(variables);
            let def = self
                .definitions
                .get(&def_key)
                .ok_or(EngineError::NoSuchDefinition(def_key))?;
            self.interrupt_loop_activity(instance_id, &mut token, &def, &boundary_id)
                .await;
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
                if !crate::engine::event_subprocess::token_valid(&inst, &token) {
                    drop(inst);
                    self.remove_persisted_service_task(task_id).await;
                    return Err(EngineError::AlreadyCompleted);
                }
                if token.event_context.is_none() {
                    inst.variables = token.process_variables();
                }
            }

            self.remove_persisted_service_task(task_id).await;
            self.emit_task_changed(instance_id, task_id).await;
            self.persist_instance(instance_id).await;
            self.run_instance_batch(instance_id, token).await?;
            return Ok(());
        }

        if let Some(instance) = self.instances.get(&instance_id).await {
            let token = instance.read().await.tokens.get(&task.token_id).cloned();
            if let Some(mut token) = token {
                token.variables.extend(variables.clone());
                if let Some(handler) = self
                    .prepare_event_code(instance_id, error_code, false, &token)
                    .await?
                {
                    self.remove_persisted_service_task(task_id).await;
                    self.run_instance_batch(instance_id, handler).await?;
                    return Ok(());
                }
            }
        }
        // Unhandled BPMN error: re-insert as incident so the token is not a zombie.
        let mut incident = task;
        incident.retries = 0;
        incident.worker_id = None;
        incident.lock_expiration = None;
        incident.error_message = Some(format!("Unhandled BPMN error '{error_code}'"));

        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let mut inst = inst_arc.write().await;
            if let Some(token) = inst.tokens.get_mut(&incident.token_id) {
                token.variables.extend(variables.clone());
            }
            inst.variables.extend(variables);
            let definition = self
                .definitions
                .get(&inst.definition_key)
                .ok_or(EngineError::NoSuchDefinition(inst.definition_key))?;
            Self::evaluate_conditionals(&mut inst, &definition)?;
            inst.push_audit_log(format!(
                "🚨 BPMN error '{}' thrown by worker '{}' at service task '{}' (No boundary event caught it)",
                error_code, worker_id, incident.node_id
            ));
        }

        tracing::warn!(
            "Service task {task_id}: unhandled BPMN error '{error_code}' from worker '{worker_id}'"
        );

        self.insert_pending_service_task(incident);
        self.persist_service_task(task_id).await;
        self.persist_instance(instance_id).await;
        self.run_conditional_work(instance_id).await?;

        Ok(())
    }
}
