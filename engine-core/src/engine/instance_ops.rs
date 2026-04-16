use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::FileReference;
use crate::domain::{EngineError, EngineResult};
use crate::persistence::CompletedInstanceQuery;

use super::WorkflowEngine;
use crate::runtime::{
    EngineStats, InstanceState, PendingMessageCatch, PendingServiceTask, PendingTimer,
    PendingUserTask, ProcessInstance,
};

impl WorkflowEngine {
    /// Returns summary statistics for monitoring dashboards.
    pub async fn get_stats(&self) -> EngineStats {
        let all_insts = self.instances.all().await;
        let mut running = 0;
        let mut comp = 0;
        let mut w_user = 0;
        let mut w_serv = 0;
        for lk in all_insts.values() {
            let st = &lk.read().await.state;
            match st {
                InstanceState::Running => running += 1,
                InstanceState::Completed | InstanceState::CompletedWithError { .. } => comp += 1,
                InstanceState::WaitingOnUserTask { .. } => w_user += 1,
                InstanceState::WaitingOnServiceTask { .. } => w_serv += 1,
                _ => {}
            }
        }
        EngineStats {
            definitions_count: self.definitions.len(),
            instances_total: all_insts.len(),
            instances_running: running,
            instances_completed: comp,
            instances_waiting_user: w_user,
            instances_waiting_service: w_serv,
            pending_user_tasks: self.pending_user_tasks.len(),
            pending_service_tasks: self.pending_service_tasks.len(),
            pending_timers: self.pending_timers.len(),
            pending_message_catches: self.pending_message_catches.len(),
            persistence_errors: self
                .persistence_error_count
                .load(std::sync::atomic::Ordering::Relaxed),
            pending_retry_jobs: 0, // mpsc unbounded channel has no len(); always 0 in stats for now
        }
    }

    /// Returns the state of a process instance (checks archive if not in active map).
    pub async fn get_instance_state(&self, instance_id: Uuid) -> EngineResult<InstanceState> {
        if let Some(i_arc) = self.instances.get(&instance_id).await {
            return Ok(i_arc.read().await.state.clone());
        }
        // Fall back to archived instances
        let inst = self.get_instance_or_archived(instance_id).await?;
        Ok(inst.state)
    }

    /// Returns the audit log of a process instance (checks archive if not in active map).
    pub async fn get_audit_log(&self, instance_id: Uuid) -> EngineResult<Vec<String>> {
        if let Some(i_arc) = self.instances.get(&instance_id).await {
            return Ok(i_arc.read().await.audit_log.clone());
        }
        let inst = self.get_instance_or_archived(instance_id).await?;
        Ok(inst.audit_log)
    }

    /// Returns all currently pending user tasks.
    pub fn get_pending_user_tasks(&self) -> Vec<PendingUserTask> {
        self.pending_user_tasks
            .iter()
            .map(|it| it.value().clone())
            .collect()
    }

    /// Returns all pending service tasks (for debugging / admin).
    pub fn get_pending_service_tasks(&self) -> Vec<PendingServiceTask> {
        self.pending_service_tasks
            .iter()
            .map(|it| it.value().clone())
            .collect()
    }

    /// Like `get_pending_user_tasks` but enriches each task's `business_key`
    /// from the in-memory instance (covers tasks persisted before the field existed).
    pub async fn get_pending_user_tasks_enriched(&self) -> Vec<PendingUserTask> {
        let tasks = self.get_pending_user_tasks();
        let mut result = Vec::with_capacity(tasks.len());
        for mut task in tasks {
            if task.business_key.is_none()
                && let Some(arc) = self.instances.get(&task.instance_id).await
            {
                let inst = arc.read().await;
                if !inst.business_key.is_empty() {
                    task.business_key = Some(inst.business_key.clone());
                }
            }
            result.push(task);
        }
        result
    }

    /// Like `get_pending_service_tasks` but enriches each task's `business_key`
    /// from the in-memory instance.
    pub async fn get_pending_service_tasks_enriched(&self) -> Vec<PendingServiceTask> {
        let tasks = self.get_pending_service_tasks();
        let mut result = Vec::with_capacity(tasks.len());
        for mut task in tasks {
            if task.business_key.is_none()
                && let Some(arc) = self.instances.get(&task.instance_id).await
            {
                let inst = arc.read().await;
                if !inst.business_key.is_empty() {
                    task.business_key = Some(inst.business_key.clone());
                }
            }
            result.push(task);
        }
        result
    }

    /// Returns all currently pending timers.
    pub fn get_pending_timers(&self) -> Vec<PendingTimer> {
        self.pending_timers
            .iter()
            .map(|it| it.value().clone())
            .collect()
    }

    /// Returns all currently pending message catch events.
    pub fn get_pending_message_catches(&self) -> Vec<PendingMessageCatch> {
        self.pending_message_catches
            .iter()
            .map(|it| it.value().clone())
            .collect()
    }

    /// Query archived (completed) instances via persistence.
    pub async fn query_completed_instances(
        &self,
        query: CompletedInstanceQuery,
    ) -> EngineResult<Vec<ProcessInstance>> {
        if let Some(p) = &self.persistence {
            p.query_completed_instances(query).await
        } else {
            Ok(vec![])
        }
    }

    /// Load a single instance by ID — checks active instances first, then archive.
    pub async fn get_instance_or_archived(&self, id: Uuid) -> EngineResult<ProcessInstance> {
        // Check active instances first
        if let Some(inst_arc) = self.instances.get(&id).await {
            return Ok(inst_arc.read().await.clone());
        }
        // Fall back to archived instances
        if let Some(p) = &self.persistence
            && let Some(inst) = p.get_completed_instance(&id.to_string()).await?
        {
            return Ok(inst);
        }
        Err(EngineError::NoSuchInstance(id))
    }

    /// Returns a list of all process instances (cloned).
    pub async fn list_instances(&self) -> Vec<ProcessInstance> {
        let all = self.instances.all().await;
        let mut out = Vec::with_capacity(all.len());
        for lk in all.values() {
            out.push(lk.read().await.clone());
        }
        out
    }

    /// Returns full details for a single process instance (checks archive if not in active map).
    pub async fn get_instance_details(&self, id: Uuid) -> EngineResult<ProcessInstance> {
        self.get_instance_or_archived(id).await
    }

    /// Updates variables on a running process instance.
    ///
    /// - Keys with non-null values are created or overwritten.
    /// - Keys with `Value::Null` are removed from the instance variables.
    pub async fn update_instance_variables(
        &self,
        instance_id: Uuid,
        variables: HashMap<String, Value>,
    ) -> EngineResult<()> {
        let old_state = if let Some(lk) = self.instances.get(&instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };

        let updated_vars = {
            let instance_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut instance = instance_arc.write().await;

            let mut added: usize = 0;
            let mut modified: usize = 0;
            let mut deleted: usize = 0;

            for (key, value) in variables {
                if value.is_null() {
                    // Delete
                    if instance.variables.remove(&key).is_some() {
                        deleted += 1;
                    }
                } else {
                    match instance.variables.entry(key) {
                        std::collections::hash_map::Entry::Occupied(mut e) => {
                            // Update existing
                            e.insert(value);
                            modified += 1;
                        }
                        std::collections::hash_map::Entry::Vacant(e) => {
                            // Create new
                            e.insert(value);
                            added += 1;
                        }
                    }
                }
            }

            instance.push_audit_log(format!(
                "Variables updated: +{added} ~{modified} -{deleted}"
            ));

            tracing::info!(
                "Instance {}: variables updated (+{added} ~{modified} -{deleted})",
                instance_id
            );

            instance.variables.clone()
        };

        // With centralized tokens, we also update instance.tokens so that
        // when a pending task is completed, it picks up the latest variables.
        {
            let instance_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut instance = instance_arc.write().await;
            for token in instance.tokens.values_mut() {
                for (key, value) in &updated_vars {
                    if value.is_null() {
                        token.variables.remove(key);
                    } else {
                        token.variables.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::VariableUpdated,
            "Variables updated directly",
            crate::history::ActorType::User, // API call
            None,
            old_state.as_ref(),
        )
        .await;

        self.persist_instance(instance_id).await;

        Ok(())
    }

    /// Suspends a running process instance.
    ///
    /// While suspended, timers won't fire and task completions are rejected.
    /// The previous state is stored inside the `Suspended` variant so that
    /// `resume_instance` can restore it.
    pub async fn suspend_instance(&self, instance_id: Uuid) -> EngineResult<()> {
        let old_state = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let inst = inst_arc.read().await;
            Some(inst.clone())
        };

        {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;

            // Cannot suspend an already-completed or already-suspended instance
            match &inst.state {
                InstanceState::Completed | InstanceState::CompletedWithError { .. } => {
                    return Err(EngineError::AlreadyCompleted);
                }
                InstanceState::Suspended { .. } => {
                    return Err(EngineError::InstanceSuspended(instance_id));
                }
                _ => {}
            }

            let previous = inst.state.clone();
            inst.state = InstanceState::Suspended {
                previous_state: Box::new(previous),
            };
            inst.push_audit_log("⏸ Instance suspended".to_string());
        }

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::InstanceSuspended,
            "Instance suspended",
            crate::history::ActorType::User,
            None,
            old_state.as_ref(),
        )
        .await;

        self.persist_instance(instance_id).await;

        tracing::info!("Instance {instance_id}: suspended");
        Ok(())
    }

    /// Resumes a previously suspended instance, restoring its prior state.
    pub async fn resume_instance(&self, instance_id: Uuid) -> EngineResult<()> {
        let old_state = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let inst = inst_arc.read().await;
            Some(inst.clone())
        };

        {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;

            match inst.state.clone() {
                InstanceState::Suspended { previous_state } => {
                    inst.state = *previous_state;
                }
                _ => {
                    return Err(EngineError::InvalidDefinition(
                        "Instance is not suspended".into(),
                    ));
                }
            }

            inst.push_audit_log("▶ Instance resumed".to_string());
        }

        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::InstanceResumed,
            "Instance resumed",
            crate::history::ActorType::User,
            None,
            old_state.as_ref(),
        )
        .await;

        self.persist_instance(instance_id).await;

        tracing::info!("Instance {instance_id}: resumed");
        Ok(())
    }

    /// Moves the active token to a different node in the process definition.
    ///
    /// This is equivalent to Camunda's "Modify Process Instance" — one of the
    /// most powerful admin/ops tools. It:
    /// 1. Validates that the target node exists in the definition.
    /// 2. Cancels all pending wait states (user tasks, service tasks, timers,
    ///    message catches) for this instance.
    /// 3. Creates a fresh token at the target node.
    /// 4. Optionally merges additional variables.
    /// 5. Starts execution from the target node via `run_instance_batch`.
    pub async fn move_token(
        &self,
        instance_id: Uuid,
        target_node_id: &str,
        variables: HashMap<String, Value>,
        cancel_current: bool,
    ) -> EngineResult<()> {
        // --- 1. Validate instance exists and is not completed ---
        let (def_key, old_current_node) = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let inst = inst_arc.read().await;

            match &inst.state {
                InstanceState::Completed | InstanceState::CompletedWithError { .. } => {
                    return Err(EngineError::AlreadyCompleted);
                }
                InstanceState::Suspended { .. } => {
                    return Err(EngineError::InstanceSuspended(instance_id));
                }
                _ => {}
            }

            (inst.definition_key, inst.current_node.clone())
        };

        // --- 2. Validate target node exists in definition ---
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;

        if !def.nodes.contains_key(target_node_id) {
            return Err(EngineError::NoSuchNode(target_node_id.to_string()));
        }

        let old_state = if let Some(lk) = self.instances.get(&instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };

        // --- 3. Cancel all pending wait states if requested ---
        if cancel_current {
            // Remove pending user tasks
            let user_task_ids: Vec<Uuid> = self
                .pending_user_tasks
                .iter()
                .filter(|t| t.instance_id == instance_id)
                .map(|t| t.task_id)
                .collect();
            for tid in &user_task_ids {
                self.pending_user_tasks.remove(tid);
                if let Some(p) = &self.persistence {
                    let _ = p.delete_user_task(*tid).await;
                }
            }

            // Remove pending service tasks
            let service_task_ids: Vec<Uuid> = self
                .pending_service_tasks
                .iter()
                .filter(|t| t.instance_id == instance_id)
                .map(|t| t.id)
                .collect();
            for tid in &service_task_ids {
                self.pending_service_tasks.remove(tid);
                if let Some(p) = &self.persistence {
                    let _ = p.delete_service_task(*tid).await;
                }
            }

            // Remove pending timers
            let timer_ids: Vec<Uuid> = self
                .pending_timers
                .iter()
                .filter(|t| t.instance_id == instance_id)
                .map(|t| t.id)
                .collect();
            for tid in &timer_ids {
                self.pending_timers.remove(tid);
                if let Some(p) = &self.persistence {
                    let _ = p.delete_timer(*tid).await;
                }
            }

            // Remove pending message catches
            let msg_ids: Vec<Uuid> = self
                .pending_message_catches
                .iter()
                .filter(|t| t.instance_id == instance_id)
                .map(|t| t.id)
                .collect();
            for tid in &msg_ids {
                self.pending_message_catches.remove(tid);
                if let Some(p) = &self.persistence {
                    let _ = p.delete_message_catch(*tid).await;
                }
            }
        }

        // --- 4. Create a fresh token at the target node ---
        let mut token_vars = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let inst = inst_arc.read().await;
            inst.variables.clone()
        };
        // Merge provided variables
        for (k, v) in variables {
            if v.is_null() {
                token_vars.remove(&k);
            } else {
                token_vars.insert(k, v);
            }
        }

        let token = crate::domain::Token {
            id: Uuid::new_v4(),
            current_node: target_node_id.to_string(),
            variables: token_vars.clone(),
            is_merged: false,
        };

        // --- 5. Reset instance state ---
        {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;

            // Clear old tokens and active_tokens
            inst.tokens.clear();
            inst.active_tokens.clear();
            inst.join_barriers.clear();
            inst.multi_instance_state.clear();

            // Insert the new token
            inst.tokens.insert(token.id, token.clone());

            // Update instance state
            inst.state = InstanceState::Running;
            inst.current_node = target_node_id.to_string();

            // Sync variables to instance level
            inst.variables = token_vars;

            inst.push_audit_log(format!(
                "🎯 Token moved: '{}' → '{}'",
                old_current_node, target_node_id
            ));
        }

        // --- 6. Record history ---
        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::TokenMoved,
            &format!(
                "Token manually moved from '{}' to '{}'",
                old_current_node, target_node_id
            ),
            crate::history::ActorType::User,
            None,
            old_state.as_ref(),
        )
        .await;

        self.persist_instance(instance_id).await;

        tracing::info!(
            "Instance {}: token moved from '{}' to '{}'",
            instance_id,
            old_current_node,
            target_node_id
        );

        // --- 7. Start execution from target node ---
        self.run_instance_batch(instance_id, token).await
    }

    /// Migrates an active process instance to a different process definition.
    ///
    /// Tokens are remapped via `node_mapping` (old node ID → new node ID).
    /// If a token sits on a node that is absent in the target definition and
    /// no mapping entry covers it, the call fails with `EngineError::OrphanedToken`
    /// — nothing is written.
    ///
    /// Only instances in `Running`, `WaitingOn*`, or `ParallelExecution` states
    /// may be migrated; `Completed`, `CompletedWithError`, and `Suspended` are rejected.
    pub async fn migrate_instance(
        &self,
        instance_id: Uuid,
        target_definition_key: Uuid,
        node_mapping: HashMap<String, String>,
    ) -> EngineResult<()> {
        // --- 1. Validate target definition exists ---
        let (target_id, target_version) = {
            let def = self
                .definitions
                .get(&target_definition_key)
                .ok_or(EngineError::NoSuchDefinition(target_definition_key))?;
            (def.id.clone(), def.version)
        };

        // --- 2. Capture old state for history diff ---
        let old_state = {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            inst_arc.read().await.clone()
        };

        // --- 3. Guard: reject terminal / suspended states ---
        match &old_state.state {
            crate::runtime::InstanceState::Completed
            | crate::runtime::InstanceState::CompletedWithError { .. } => {
                return Err(EngineError::AlreadyCompleted);
            }
            crate::runtime::InstanceState::Suspended { .. } => {
                return Err(EngineError::InstanceSuspended(instance_id));
            }
            _ => {}
        }

        // --- 4. Pre-validate: every token node must exist in target definition
        //        (either directly or via mapping) — validate BEFORE writing ---
        {
            let def = self
                .definitions
                .get(&target_definition_key)
                .ok_or(EngineError::NoSuchDefinition(target_definition_key))?;

            // Validate instance.current_node
            let resolved = node_mapping
                .get(&old_state.current_node)
                .map(String::as_str)
                .unwrap_or(old_state.current_node.as_str());
            if !def.nodes.contains_key(resolved) {
                return Err(EngineError::OrphanedToken(resolved.to_string()));
            }

            // Validate every token
            for token in old_state.tokens.values() {
                let resolved = node_mapping
                    .get(&token.current_node)
                    .map(String::as_str)
                    .unwrap_or(token.current_node.as_str());
                if !def.nodes.contains_key(resolved) {
                    return Err(EngineError::OrphanedToken(resolved.to_string()));
                }
            }
            // Validate active_tokens (they may differ from tokens in edge cases)
            for at in &old_state.active_tokens {
                let resolved = node_mapping
                    .get(&at.token.current_node)
                    .map(String::as_str)
                    .unwrap_or(at.token.current_node.as_str());
                if !def.nodes.contains_key(resolved) {
                    return Err(EngineError::OrphanedToken(resolved.to_string()));
                }
            }
        }

        // --- 5. Apply migration (write) ---
        {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;

            // Remap definition key
            inst.definition_key = target_definition_key;

            // Remap instance.current_node
            if let Some(new_node) = node_mapping.get(&inst.current_node) {
                inst.current_node = new_node.clone();
            }

            // Remap all tokens
            for token in inst.tokens.values_mut() {
                if let Some(new_node) = node_mapping.get(&token.current_node) {
                    token.current_node = new_node.clone();
                }
            }

            // Remap active_tokens
            for at in &mut inst.active_tokens {
                if let Some(new_node) = node_mapping.get(&at.token.current_node) {
                    at.token.current_node = new_node.clone();
                }
            }

            // Remap join_barriers keys (gateway_node_id → new)
            let remapped_barriers: HashMap<String, crate::runtime::JoinBarrier> = inst
                .join_barriers
                .drain()
                .map(|(k, v)| (node_mapping.get(&k).cloned().unwrap_or(k), v))
                .collect();
            inst.join_barriers = remapped_barriers;

            // Remap multi_instance_state keys
            let remapped_mi: HashMap<String, crate::runtime::MultiInstanceProgress> = inst
                .multi_instance_state
                .drain()
                .map(|(k, v)| (node_mapping.get(&k).cloned().unwrap_or(k), v))
                .collect();
            inst.multi_instance_state = remapped_mi;

            inst.push_audit_log(format!(
                "🔧 Migriert zu Definition {target_id} (v{target_version})"
            ));
        }

        // --- 6. Remap pending tasks that belong to this instance ---
        for mut entry in self.pending_user_tasks.iter_mut() {
            if entry.instance_id == instance_id
                && let Some(new_node) = node_mapping.get(&entry.node_id)
            {
                entry.node_id = new_node.clone();
            }
        }
        for mut entry in self.pending_service_tasks.iter_mut() {
            if entry.instance_id == instance_id {
                if let Some(new_node) = node_mapping.get(&entry.node_id) {
                    entry.node_id = new_node.clone();
                }
                entry.definition_key = target_definition_key;
            }
        }
        for mut entry in self.pending_timers.iter_mut() {
            if entry.instance_id == instance_id
                && let Some(new_node) = node_mapping.get(&entry.node_id)
            {
                entry.node_id = new_node.clone();
            }
        }
        for mut entry in self.pending_message_catches.iter_mut() {
            if entry.instance_id == instance_id
                && let Some(new_node) = node_mapping.get(&entry.node_id)
            {
                entry.node_id = new_node.clone();
            }
        }

        // --- 7. Record history ---
        self.record_history_event(
            instance_id,
            crate::history::HistoryEventType::InstanceMigrated,
            &format!(
                "Instanz zu Definition '{target_id}' (v{target_version}) migriert"
            ),
            crate::history::ActorType::User,
            None,
            Some(&old_state),
        )
        .await;

        // --- 8. Persist updated instance ---
        self.persist_instance(instance_id).await;

        tracing::info!(
            "Instance {instance_id}: migriert zu Definition '{target_id}' (v{target_version})"
        );

        Ok(())
    }

    /// Deletes a process instance and cleans up associated pending tasks.
    pub async fn delete_instance(&self, instance_id: Uuid) -> EngineResult<()> {
        let removed_inst_arc = self
            .instances
            .remove(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let removed_inst = removed_inst_arc.read().await.clone();

        if let Some(ref persistence) = self.persistence {
            // Delete associated files
            for value in removed_inst.variables.values() {
                if let Some(file_ref) = FileReference::from_variable_value(value) {
                    let _ = persistence.delete_file(&file_ref.object_key).await;
                }
            }

            // Delete associated user tasks from persistence
            for task in self
                .pending_user_tasks
                .iter()
                .filter(|t| t.instance_id == instance_id)
            {
                let _ = persistence.delete_user_task(task.task_id).await;
            }
            // Delete associated service tasks from persistence
            for task in self
                .pending_service_tasks
                .iter()
                .filter(|t| t.instance_id == instance_id)
            {
                let _ = persistence.delete_service_task(task.id).await;
            }
            // Delete associated timers from persistence
            for timer in self
                .pending_timers
                .iter()
                .filter(|t| t.instance_id == instance_id)
            {
                let _ = persistence.delete_timer(timer.id).await;
            }
            // Delete associated message catches from persistence
            for catch in self
                .pending_message_catches
                .iter()
                .filter(|t| t.instance_id == instance_id)
            {
                let _ = persistence.delete_message_catch(catch.id).await;
            }
            // Delete instance from persistence
            persistence
                .delete_instance(&instance_id.to_string())
                .await?;
        }

        // Clean up pending user tasks in memory
        self.pending_user_tasks
            .retain(|_, t| t.instance_id != instance_id);

        // Clean up pending service tasks in memory
        self.pending_service_tasks
            .retain(|_, t| t.instance_id != instance_id);

        // Clean up pending timers in memory
        self.pending_timers
            .retain(|_, t| t.instance_id != instance_id);

        // Clean up pending message catches in memory
        self.pending_message_catches
            .retain(|_, t| t.instance_id != instance_id);

        Ok(())
    }
}
