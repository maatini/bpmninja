use super::WorkflowEngine;
use crate::InstanceState;
use crate::domain::BpmnElement;
use crate::domain::{EngineError, EngineResult};
use crate::history::{ActorType, HistoryEventType};
use crate::runtime::PendingSignalCatch;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

impl WorkflowEngine {
    /// Korreliert ein Broadcast-Signal mit allen wartenden Catches und startet
    /// Instanzen für passende Signal-Start-Events.
    pub async fn correlate_signal(
        &self,
        signal_name: String,
        variables: HashMap<String, Value>,
    ) -> EngineResult<Vec<Uuid>> {
        let mut affected = self
            .broadcast_signal(&signal_name, &variables, true)
            .await?;
        affected.sort_unstable();
        affected.dedup();
        Ok(affected)
    }

    /// Zustellung eines Signals an alle lokalen Catches.
    ///
    /// `start_instances` ist nur für `correlate_signal` gesetzt: interne Throws
    /// dürfen keine neuen Instanzen starten (Selbst-Trigger-Schutz).
    pub(crate) async fn broadcast_signal(
        &self,
        signal_name: &str,
        variables: &HashMap<String, Value>,
        start_instances: bool,
    ) -> EngineResult<Vec<Uuid>> {
        let catches: Vec<PendingSignalCatch> = self
            .pending_signal_catches
            .iter()
            .filter(|c| c.signal_name == signal_name)
            .map(|c| c.value().clone())
            .collect();

        let mut affected = Vec::new();
        for catch in catches {
            // Kann durch einen vorherigen Catch desselben Broadcasts schon weg sein.
            let Some((_, catch)) = self.pending_signal_catches.remove(&catch.id) else {
                continue;
            };
            self.resume_signal_catch(&catch, signal_name, variables)
                .await?;
            affected.push(catch.instance_id);
        }

        if start_instances {
            let mut defs_to_start = Vec::new();
            let all_defs = self.definitions.all();
            for (def_key, def) in &all_defs {
                if let Some((
                    _,
                    BpmnElement::SignalStartEvent {
                        signal_name: ref_name,
                    },
                )) = def.start_event()
                    && ref_name == signal_name
                {
                    defs_to_start.push(*def_key);
                }
            }

            for def_key in defs_to_start {
                let new_id = self.start_instance_from_signal(def_key, variables).await?;
                affected.push(new_id);
            }
        }

        Ok(affected)
    }

    /// Weckt einen einzelnen Signal-Catch und setzt den Token hinter dem Event fort.
    async fn resume_signal_catch(
        &self,
        catch: &PendingSignalCatch,
        signal_name: &str,
        variables: &HashMap<String, Value>,
    ) -> EngineResult<()> {
        let def_key = {
            let inst_arc = self
                .instances
                .get(&catch.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(catch.instance_id))?;
            inst_arc.read().await.definition_key
        };

        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;

        let is_non_interrupting = matches!(
            def.nodes.get(&catch.node_id),
            Some(BpmnElement::BoundarySignalEvent {
                cancel_activity: false,
                ..
            })
        );

        if !is_non_interrupting {
            // Interrupting: Geschwister-Waits (Timer, Messages, Signale) am selben
            // Token sowie den laufenden Task entfernen.
            self.clear_wait_states_for_token(catch.instance_id, &catch.token_id)
                .await;
            self.cancel_pending_activity(catch.instance_id, &catch.token_id)
                .await;
        }

        let mut token = {
            let inst_arc = self
                .instances
                .get(&catch.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(catch.instance_id))?;
            let mut inst = inst_arc.write().await;

            if is_non_interrupting {
                let mut spawned = inst
                    .tokens
                    .get(&catch.token_id)
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition(format!(
                            "Token {} not found in instance",
                            catch.token_id
                        ))
                    })?
                    .clone();

                spawned.id = Uuid::new_v4();
                inst.tokens.insert(spawned.id, spawned.clone());

                let active = crate::runtime::ActiveToken {
                    token: spawned.clone(),
                    completed: false,
                    fork_id: Some(spawned.current_node.clone()),
                    branch_index: inst.active_tokens.len(),
                };
                inst.active_tokens.push(active);

                if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                    inst.state = InstanceState::ParallelExecution {
                        active_token_count: inst.tokens.len(),
                    };
                }

                spawned
            } else {
                inst.tokens.remove(&catch.token_id).ok_or_else(|| {
                    EngineError::InvalidDefinition(format!(
                        "Token {} not found in instance",
                        catch.token_id
                    ))
                })?
            }
        };

        token.variables.extend(variables.clone());

        let old_state = if let Some(lk) = self.instances.get(&catch.instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };

        {
            let inst_arc = self
                .instances
                .get(&catch.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(catch.instance_id))?;
            let mut inst = inst_arc.write().await;
            if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                inst.state = InstanceState::Running;
            }
            inst.push_audit_log(format!(
                "📡 Signal '{}' caught at '{}' — resuming catch {}",
                signal_name, catch.node_id, catch.id
            ));
        }

        self.record_history_event(
            catch.instance_id,
            HistoryEventType::SignalCaught,
            &format!("Signal '{}' caught at '{}'", signal_name, catch.node_id),
            ActorType::Engine,
            None,
            old_state.as_ref(),
        )
        .await;

        let next =
            crate::engine::executor::resolve_next_target(&def, &catch.node_id, &token.variables)?;
        token.current_node = next.clone();

        {
            let inst_arc = self
                .instances
                .get(&catch.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(catch.instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.current_node = next;
        }

        self.remove_persisted_signal_catch(catch.id).await;

        // Boxed, weil ein Throw im selben Batch diesen Pfad rekursiv erreicht.
        Box::pin(self.run_instance_batch(catch.instance_id, token)).await?;

        Ok(())
    }

    /// Entfernt offene User-/Service-Tasks eines durch ein Boundary-Signal
    /// abgebrochenen Tokens.
    async fn cancel_pending_activity(&self, instance_id: Uuid, token_id: &Uuid) {
        let user_task_ids: Vec<Uuid> = self
            .pending_user_tasks
            .iter()
            .filter(|r| r.instance_id == instance_id && &r.token_id == token_id)
            .map(|r| r.task_id)
            .collect();
        for id in user_task_ids {
            self.pending_user_tasks.remove(&id);
            self.remove_persisted_user_task(id).await;
        }

        let service_task_ids: Vec<Uuid> = self
            .pending_service_tasks
            .iter()
            .filter(|r| r.instance_id == instance_id && &r.token_id == token_id)
            .map(|r| r.id)
            .collect();
        for id in service_task_ids {
            self.remove_pending_service_task(&id);
            self.remove_persisted_service_task(id).await;
        }
    }
}
