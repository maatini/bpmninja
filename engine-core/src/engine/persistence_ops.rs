use uuid::Uuid;
use crate::ProcessInstance;
use super::WorkflowEngine;

impl WorkflowEngine {
    /// Logs and counts a persistence error (fire-and-forget pattern).
    pub(crate) fn log_persistence_error(&self, context: &str, err: impl std::fmt::Display) {
        self.persistence_error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        log::error!("PERSISTENCE FAILURE [{}]: {}", context, err);
    }

    /// Helper to record a history entry for an instance, calculating the diff automatically.
    pub(crate) async fn record_history_event(
        &self,
        instance_id: Uuid,
        event_type: crate::history::HistoryEventType,
        description: &str,
        actor_type: crate::history::ActorType,
        actor_id: Option<String>,
        old_state: Option<&ProcessInstance>,
    ) {
        if let Some(p) = &self.persistence {
            let new_state = if let Some(lk) = self.instances.get(&instance_id).await { Some(lk.read().await.clone()) } else { None };
            let diff = match (old_state, new_state.as_ref()) {
                (Some(o), Some(n)) => crate::history::calculate_diff(o, n),
                _ => crate::history::HistoryDiff { 
                    variables: None, status: None, current_node: None, human_readable: None 
                },
            };
            
            // Do not record if nothing changed for generic token move
            if diff.is_empty() && matches!(event_type, crate::history::HistoryEventType::TokenAdvanced) {
                return;
            }

            let mut entry = crate::history::HistoryEntry::new(
                instance_id, event_type, description, actor_type, actor_id);
            if !diff.is_empty() {
                entry = entry.with_diff(diff);
            }
            if let Some(curr) = new_state.as_ref().or(old_state) {
                if let Some(def) = self.definitions.get(&curr.definition_key).await {
                    entry.definition_version = Some(def.version);
                }
            }
            
            if let Some(curr) = new_state {
                entry = entry.with_node(curr.current_node.clone());

                // Snapshot heuristic: store a full snapshot every 8 audit log entries
                if !curr.audit_log.is_empty() && curr.audit_log.len() % 8 == 0 {
                    if let Ok(json_state) = serde_json::to_value(curr) {
                        entry = entry.with_snapshot(json_state);
                    }
                }
            }

            if let Err(e) = p.append_history_entry(&entry).await {
                self.log_persistence_error(&format!("record_history_event({})", instance_id), e);
            }
        }
    }

    /// Persists the current state of a process instance (if a persistence
    /// layer is configured). Logs and swallows errors.
    pub(crate) async fn persist_instance(&self, instance_id: Uuid) {
        if let (Some(p), Some(inst_arc)) = (&self.persistence, self.instances.get(&instance_id).await) {
            let mut inst = inst_arc.write().await;
            // Trim audit log to prevent NATS KV 1MB value overflow
            if inst.audit_log.len() > crate::engine::types::MAX_AUDIT_LOG_ENTRIES {
                let overflow = inst.audit_log.len() - crate::engine::types::MAX_AUDIT_LOG_ENTRIES;
                inst.audit_log = inst.audit_log.split_off(overflow);
                inst.audit_log.insert(0, format!("... ({} older entries trimmed, see History API)", overflow));
            }
            if let Err(e) = p.save_instance(&inst).await {
                self.log_persistence_error(&format!("save_instance({})", instance_id), e);
            }
        }
    }

    /// Persists a process definition to the KV store.
    pub(crate) async fn persist_definition(&self, key: Uuid) {
        if let (Some(p), Some(def)) = (&self.persistence, self.definitions.get(&key).await) {
            if let Err(e) = p.save_definition(&def).await {
                self.log_persistence_error(&format!("save_definition({})", key), e);
            }
        }
    }

    /// Persists a pending user task to the KV store.
    pub(crate) async fn persist_user_task(&self, task_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Some(task) = self.pending_user_tasks.get(&task_id) {
                if let Err(e) = p.save_user_task(task).await {
                    self.log_persistence_error(&format!("save_user_task({})", task_id), e);
                }
            }
        }
    }

    /// Deletes a completed pending user task from the KV store.
    pub(crate) async fn remove_persisted_user_task(&self, task_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Err(e) = p.delete_user_task(task_id).await {
                self.log_persistence_error(&format!("delete_user_task({})", task_id), e);
            }
        }
    }

    /// Persists a pending service task to the KV store.
    pub(crate) async fn persist_service_task(&self, task_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Some(task) = self.pending_service_tasks.get(&task_id) {
                if let Err(e) = p.save_service_task(task).await {
                    self.log_persistence_error(&format!("save_service_task({})", task_id), e);
                }
            }
        }
    }

    /// Deletes a completed pending service task from the KV store.
    pub(crate) async fn remove_persisted_service_task(&self, task_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Err(e) = p.delete_service_task(task_id).await {
                self.log_persistence_error(&format!("delete_service_task({})", task_id), e);
            }
        }
    }

    /// Persists a pending timer to the KV store.
    pub(crate) async fn persist_timer(&self, timer_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Some(timer) = self.pending_timers.get(&timer_id) {
                if let Err(e) = p.save_timer(timer).await {
                    self.log_persistence_error(&format!("save_timer({})", timer_id), e);
                }
            }
        }
    }

    /// Deletes a completed pending timer from the KV store.
    pub(crate) async fn remove_persisted_timer(&self, timer_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Err(e) = p.delete_timer(timer_id).await {
                self.log_persistence_error(&format!("delete_timer({})", timer_id), e);
            }
        }
    }

    /// Persists a pending message catch to the KV store.
    pub(crate) async fn persist_message_catch(&self, catch_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Some(catch) = self.pending_message_catches.get(&catch_id) {
                if let Err(e) = p.save_message_catch(catch).await {
                    self.log_persistence_error(&format!("save_message_catch({})", catch_id), e);
                }
            }
        }
    }

    /// Deletes a completed pending message catch from the KV store.
    pub(crate) async fn remove_persisted_message_catch(&self, catch_id: Uuid) {
        if let Some(p) = &self.persistence {
            if let Err(e) = p.delete_message_catch(catch_id).await {
                self.log_persistence_error(&format!("delete_message_catch({})", catch_id), e);
            }
        }
    }
}
