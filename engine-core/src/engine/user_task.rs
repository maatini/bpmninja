use super::WorkflowEngine;
use crate::InstanceState;
use crate::domain::{EngineError, EngineResult};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

impl WorkflowEngine {
    /// Completes a pending user task by its task_id, optionally merging variables.
    ///
    /// Resumes the process instance after the user task.
    pub async fn complete_user_task(
        &self,
        task_id: Uuid,
        additional_vars: HashMap<String, Value>,
    ) -> EngineResult<()> {
        // Find and remove the pending task
        let pending = self
            .pending_user_tasks
            .remove(&task_id)
            .map(|(_, v)| v)
            .ok_or_else(|| EngineError::TaskNotPending {
                task_id,
                actual_state: "not found in pending tasks".into(),
            })?;

        let instance_id = pending.instance_id;
        let updated_names: Vec<String> = additional_vars.keys().cloned().collect();

        // Reject if instance is suspended
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let inst = inst_arc.read().await;
            if matches!(inst.state, InstanceState::Suspended { .. }) {
                // Re-insert the pending task so it isn't lost
                self.pending_user_tasks.insert(task_id, pending);
                return Err(EngineError::InstanceSuspended(instance_id));
            }
        }

        // Retrieve token from central store and merge additional variables
        let mut token = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            let retrieved = inst.tokens.remove(&pending.token_id);
            drop(inst);
            if let Some(token) = retrieved {
                token
            } else {
                self.remove_persisted_user_task(task_id).await;
                return Err(EngineError::AlreadyCompleted);
            }
        };
        let original_token = token.clone();
        for (k, v) in additional_vars {
            token.variables.insert(k, v);
        }

        let old_state = if let Some(lk) = self.instances.get(&instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };

        tracing::info!(
            "Instance {instance_id}: completed user task '{}' (task_id: {task_id})",
            pending.node_id
        );

        let def_key = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.audit_log
                .push(format!("✅ User task '{}' completed", pending.node_id));

            if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                inst.state = InstanceState::Running;
            }
            inst.current_node = pending.node_id.clone();
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
        // Current node's end scripts — on failure, rollback task to pending so it isn't lost
        if let Err(e) = self
            .run_end_scripts(instance_id, &mut token, &def, &pending.node_id)
            .await
        {
            tracing::error!(
                "Instance {instance_id}: end script failed for user task '{}': {e} — re-inserting task",
                pending.node_id
            );
            // Restore token into instance so the task is completable again
            if let Some(inst_arc) = self.instances.get(&instance_id).await {
                let mut inst = inst_arc.write().await;
                if !crate::engine::event_subprocess::token_valid(&inst, &original_token) {
                    drop(inst);
                    self.remove_persisted_user_task(task_id).await;
                    return Err(EngineError::AlreadyCompleted);
                }
                if let Some(old) = &old_state {
                    inst.state = old.state.clone();
                    inst.current_node = old.current_node.clone();
                    inst.variable_types = old.variable_types.clone();
                }
                inst.tokens.insert(original_token.id, original_token);
            }
            self.pending_user_tasks.insert(task_id, pending);
            self.persist_instance(instance_id).await;
            return Err(e);
        }

        self.remove_persisted_user_task(task_id).await;
        self.cancel_boundary_timers(instance_id, &pending.node_id, Some(pending.token_id))
            .await;
        self.cancel_boundary_message_catches(instance_id, &pending.node_id, Some(pending.token_id))
            .await;

        // Register compensation handler if this activity has a BoundaryCompensationEvent
        self.register_compensation_handler(instance_id, &pending.node_id, &def)
            .await;

        self.run_conditional_work(instance_id).await?;
        if let Some(instance) = self.instances.get(&instance_id).await
            && !crate::engine::event_subprocess::token_valid(&*instance.read().await, &token)
        {
            return Ok(());
        }
        let next =
            crate::engine::executor::resolve_next_target(&def, &pending.node_id, &token.variables)?;

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

        metrics::counter!("bpmn_tasks_completed_total", "type" => "user").increment(1);

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::TaskCompleted,
            &format!("User task '{}' completed", pending.node_id),
            crate::history::ActorType::User,
            Some(pending.assignee.clone()),
            old_state.as_ref(),
        )
        .await;

        // Continue running
        self.emit_task_changed(instance_id, task_id).await;
        self.run_instance_batch(instance_id, token).await
    }
}
