use crate::domain::{EngineError, EngineResult};
use crate::domain::{MultiInstanceDef, ProcessDefinition, Token};
use crate::engine::WorkflowEngine;
use crate::engine::boundary::setup_boundary_events;
use crate::engine::executor::resolve_next_target;
use crate::runtime::{MultiInstanceProgress, NextAction, PendingServiceTask, PendingUserTask};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Liefert die Items für ein paralleles Multi-Instance-Service-Task.
///
/// `collection` wird als Array aus den Token-Variablen gelesen. Bei
/// `loop_cardinality` bleibt der Item-Wert leer (`None`); die Anzahl wird
/// stattdessen über `loopCounter` abgebildet.
fn resolve_multi_instance_items(
    def: &MultiInstanceDef,
    variables: &HashMap<String, Value>,
    element_id: &str,
) -> EngineResult<Vec<Option<Value>>> {
    if let Some(collection) = &def.collection {
        return match variables.get(collection) {
            None => Err(EngineError::InvalidDefinition(format!(
                "Multi-instance collection variable '{collection}' not found for service task '{element_id}'"
            ))),
            Some(Value::Array(items)) => Ok(items.iter().cloned().map(Some).collect()),
            Some(other) => Err(EngineError::InvalidDefinition(format!(
                "Multi-instance collection variable '{collection}' of service task '{element_id}' must be an array, got {}",
                json_type_name(other)
            ))),
        };
    }

    let Some(cardinality) = &def.loop_cardinality else {
        return Err(EngineError::InvalidDefinition(format!(
            "Multi-instance service task '{element_id}' has neither collection nor loopCardinality"
        )));
    };
    let total: usize = cardinality.trim().parse().map_err(|_| {
        EngineError::InvalidDefinition(format!(
            "Multi-instance loopCardinality of service task '{element_id}' must be a positive integer, got '{cardinality}'"
        ))
    })?;
    if total == 0 {
        return Err(EngineError::InvalidDefinition(format!(
            "Multi-instance loopCardinality of service task '{element_id}' must be a positive integer, got '{cardinality}'"
        )));
    }
    Ok(vec![None; total])
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

impl WorkflowEngine {
    pub(crate) async fn handle_user_task(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def_clone: &Arc<ProcessDefinition>,
        current_id: &str,
        assignee: &String,
    ) -> EngineResult<NextAction> {
        let (pending_timers, pending_msgs) =
            setup_boundary_events(def_clone, current_id, instance_id, token);
        for t in pending_timers {
            let id = t.id;
            self.insert_pending_timer(t);
            self.persist_timer(id).await;
        }
        for m in pending_msgs {
            let id = m.id;
            self.pending_message_catches.insert(id, m);
            self.persist_message_catch(id).await;
        }

        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;

        let pending = PendingUserTask {
            task_id: Uuid::new_v4(),
            instance_id,
            node_id: current_id.to_string(),
            assignee: assignee.clone(),
            token_id: token.id,
            created_at: Utc::now(),
            business_key: if inst.business_key.is_empty() {
                None
            } else {
                Some(inst.business_key.clone())
            },
        };

        inst.current_node = current_id.to_string();
        inst.tokens.insert(token.id, token.clone());
        inst.push_audit_log(format!(
            "👤 User task '{current_id}' assigned to '{:?}' — waiting (task_id: {})",
            assignee, pending.task_id
        ));
        tracing::info!(
            "Instance {instance_id}: user task '{current_id}' pending for '{:?}'",
            assignee
        );

        Ok(NextAction::WaitForUser(pending))
    }

    pub(crate) async fn handle_script_task(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def_clone: &Arc<ProcessDefinition>,
        current_id: &str,
        script: &str,
    ) -> EngineResult<NextAction> {
        let result =
            crate::scripting::execute_script_safe(&self.script_config, script, &token.variables)
                .await?;
        token.variables = result;

        self.run_end_scripts(instance_id, token, def_clone, current_id)
            .await?;

        // Register compensation handler if this activity has a BoundaryCompensationEvent
        self.register_compensation_handler(instance_id, current_id, def_clone)
            .await;

        let next = resolve_next_target(def_clone, current_id, &token.variables)?;
        token.current_node = next.clone();

        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;
        inst.current_node = next;
        inst.variables = token.variables.clone();
        inst.push_audit_log(format!("📜 Script task '{current_id}' executed"));
        Ok(NextAction::Continue(token.clone()))
    }

    pub(crate) async fn handle_send_task(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def_clone: &Arc<ProcessDefinition>,
        current_id: &str,
        message_name: &str,
    ) -> EngineResult<NextAction> {
        tracing::info!(
            "Instance {instance_id}: send task '{current_id}' publishing message '{message_name}'"
        );
        self.run_end_scripts(instance_id, token, def_clone, current_id)
            .await?;

        let next = resolve_next_target(def_clone, current_id, &token.variables)?;
        token.current_node = next.clone();

        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;
        inst.current_node = next;
        inst.push_audit_log(format!(
            "📤 Send task '{current_id}' published message '{message_name}'"
        ));
        Ok(NextAction::Continue(token.clone()))
    }

    pub(crate) async fn handle_service_task(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def_clone: &Arc<ProcessDefinition>,
        current_id: &str,
        topic: &str,
        multi_instance: Option<&MultiInstanceDef>,
    ) -> EngineResult<NextAction> {
        let mi_items = match multi_instance {
            Some(mi) => Some(resolve_multi_instance_items(
                mi,
                &token.variables,
                current_id,
            )?),
            None => None,
        };

        // Leere Item-Liste: Aktivität überspringen, bevor Boundary-Events
        // eingerichtet werden.
        if let Some(items) = &mi_items
            && items.is_empty()
        {
            let next = resolve_next_target(def_clone, current_id, &token.variables)?;
            token.current_node = next.clone();
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.current_node = next;
            inst.push_audit_log(format!(
                "⏭ Multi-instance service task '{current_id}' skipped (empty collection)"
            ));
            tracing::info!(
                "Instance {instance_id}: multi-instance service task '{current_id}' skipped (empty collection)"
            );
            return Ok(NextAction::Continue(token.clone()));
        }

        let (pending_timers, pending_msgs) =
            setup_boundary_events(def_clone, current_id, instance_id, token);
        for t in pending_timers {
            let id = t.id;
            self.insert_pending_timer(t);
            self.persist_timer(id).await;
        }
        for m in pending_msgs {
            let id = m.id;
            self.pending_message_catches.insert(id, m);
            self.persist_message_catch(id).await;
        }

        if let (Some(mi), Some(items)) = (multi_instance, mi_items) {
            let total = items.len();
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;

            // Sequentiell: immer genau ein offener Task. Der Token bleibt in
            // der Instanz und wird nach jedem Abschluss für das nächste Item
            // wiederverwendet.
            if mi.is_sequential {
                let first = {
                    let mut inst = inst_arc.write().await;
                    let mut variables_snapshot = token.variables.clone();
                    variables_snapshot.insert("loopCounter".to_string(), Value::from(1_u64));
                    if let (Some(element_variable), Some(value)) = (
                        mi.element_variable.as_ref(),
                        items.first().cloned().flatten(),
                    ) {
                        variables_snapshot.insert(element_variable.clone(), value);
                    }
                    let first = PendingServiceTask {
                        id: Uuid::new_v4(),
                        instance_id,
                        definition_key: def_clone.key,
                        node_id: current_id.to_string(),
                        topic: topic.to_string(),
                        token_id: token.id,
                        variables_snapshot,
                        created_at: Utc::now(),
                        worker_id: None,
                        lock_expiration: None,
                        retries: 3,
                        error_message: None,
                        error_details: None,
                        business_key: if inst.business_key.is_empty() {
                            None
                        } else {
                            Some(inst.business_key.clone())
                        },
                    };

                    inst.current_node = current_id.to_string();
                    inst.tokens.insert(token.id, token.clone());
                    inst.multi_instance_state.insert(
                        current_id.to_string(),
                        MultiInstanceProgress {
                            node_id: current_id.to_string(),
                            total,
                            completed: 0,
                            sequential_token: Some(token.clone()),
                            items,
                        },
                    );
                    inst.push_audit_log(format!(
                        "🔗 Sequential multi-instance service task '{current_id}' started item 1/{total} for topic '{topic}'"
                    ));
                    first
                };

                tracing::info!(
                    "Instance {instance_id}: sequential multi-instance service task '{current_id}' pending with item 1/{total} for topic '{topic}'"
                );
                return Ok(NextAction::WaitForServiceTask(first));
            }

            let pending_tasks = {
                let mut inst = inst_arc.write().await;
                let mut tasks = Vec::with_capacity(total);
                for (idx, item) in items.into_iter().enumerate() {
                    let mut variables_snapshot = token.variables.clone();
                    variables_snapshot
                        .insert("loopCounter".to_string(), Value::from((idx + 1) as u64));
                    if let (Some(element_variable), Some(value)) =
                        (mi.element_variable.as_ref(), item)
                    {
                        variables_snapshot.insert(element_variable.clone(), value);
                    }
                    tasks.push(PendingServiceTask {
                        id: Uuid::new_v4(),
                        instance_id,
                        definition_key: def_clone.key,
                        node_id: current_id.to_string(),
                        topic: topic.to_string(),
                        token_id: token.id,
                        variables_snapshot,
                        created_at: Utc::now(),
                        worker_id: None,
                        lock_expiration: None,
                        retries: 3,
                        error_message: None,
                        error_details: None,
                        business_key: if inst.business_key.is_empty() {
                            None
                        } else {
                            Some(inst.business_key.clone())
                        },
                    });
                }

                inst.current_node = current_id.to_string();
                inst.tokens.insert(token.id, token.clone());
                inst.multi_instance_state.insert(
                    current_id.to_string(),
                    MultiInstanceProgress {
                        node_id: current_id.to_string(),
                        total,
                        completed: 0,
                        sequential_token: None,
                        items: Vec::new(),
                    },
                );
                inst.push_audit_log(format!(
                    "🔗 Multi-instance service task '{current_id}' created {total} tasks for topic '{topic}'"
                ));
                tasks
            };

            tracing::info!(
                "Instance {instance_id}: multi-instance service task '{current_id}' pending with {total} tasks for topic '{topic}'"
            );

            // Der erste Task geht über den Executor in die Pending-Map, die
            // übrigen werden hier direkt eingefügt und persistiert.
            let Some((first, rest)) = pending_tasks.split_first() else {
                return Ok(NextAction::Continue(token.clone()));
            };
            let first = first.clone();
            for task in rest {
                let id = task.id;
                self.insert_pending_service_task(task.clone());
                self.persist_service_task(id).await;
            }
            return Ok(NextAction::WaitForServiceTask(first));
        }

        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;

        let svc_task = PendingServiceTask {
            id: Uuid::new_v4(),
            instance_id,
            definition_key: def_clone.key,
            node_id: current_id.to_string(),
            topic: topic.to_string(),
            token_id: token.id,
            variables_snapshot: token.variables.clone(),
            created_at: Utc::now(),
            worker_id: None,
            lock_expiration: None,
            retries: 3,
            error_message: None,
            error_details: None,
            business_key: if inst.business_key.is_empty() {
                None
            } else {
                Some(inst.business_key.clone())
            },
        };

        inst.current_node = current_id.to_string();
        inst.tokens.insert(token.id, token.clone());
        inst.push_audit_log(format!(
            "🔗 Service task '{current_id}' created for topic '{topic}' (task_id: {})",
            svc_task.id
        ));
        tracing::info!(
            "Instance {instance_id}: service task '{current_id}' pending for topic '{topic}'"
        );
        Ok(NextAction::WaitForServiceTask(svc_task))
    }
}
