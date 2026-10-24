use uuid::Uuid;

use crate::condition::evaluate_condition;
use crate::domain::{BpmnElement, EngineError, EngineResult, Token};
use crate::engine::WorkflowEngine;
use crate::engine::executor::resolve_next_target;
use crate::history::{ActorType, HistoryEventType};
use crate::runtime::{ActiveToken, InstanceState, PendingConditionalCatch};

impl WorkflowEngine {
    /// Wertet alle Conditional-Catches einer Instanz gegen die aktuellen
    /// Instanzvariablen aus und liefert die freigegebenen Tokens.
    ///
    /// Gefeuert wird nur beim Übergang `false -> true`. Intermediate Catches
    /// werden dabei entfernt; nicht-unterbrechende Boundary-Catches bleiben
    /// registriert und feuern erst nach einem erneuten `false -> true`.
    pub(crate) async fn fire_conditional_catches(
        &self,
        instance_id: Uuid,
    ) -> EngineResult<Vec<Token>> {
        let catches: Vec<PendingConditionalCatch> = self
            .pending_conditional_catches
            .iter()
            .filter(|catch| catch.instance_id == instance_id)
            .map(|catch| catch.value().clone())
            .collect();
        if catches.is_empty() {
            return Ok(Vec::new());
        }

        let (def_key, variables) = {
            let Some(inst_arc) = self.instances.get(&instance_id).await else {
                return Ok(Vec::new());
            };
            let inst = inst_arc.read().await;
            match inst.state {
                InstanceState::Suspended { .. }
                | InstanceState::Completed
                | InstanceState::CompletedWithError { .. } => return Ok(Vec::new()),
                _ => {}
            }
            (inst.definition_key, inst.variables.clone())
        };

        let Some(def) = self.definitions.get(&def_key) else {
            return Ok(Vec::new());
        };

        let mut fired = Vec::new();
        for catch in catches {
            let is_true = evaluate_condition(&catch.condition, &variables);
            if !is_true {
                if catch.last_result {
                    if let Some(mut entry) = self.pending_conditional_catches.get_mut(&catch.id) {
                        entry.last_result = false;
                    }
                    // Der Übergang true -> false muss auch einen Neustart überleben.
                    self.persist_conditional_catch(catch.id).await;
                }
                continue;
            }
            if catch.last_result {
                continue;
            }

            let Some((_, catch)) = self.pending_conditional_catches.remove(&catch.id) else {
                continue;
            };

            match def.nodes.get(&catch.node_id) {
                Some(BpmnElement::ConditionalCatchEvent { .. }) => {
                    let next =
                        resolve_next_target(&def, &catch.node_id, &variables).map_err(|error| {
                            EngineError::InvalidDefinition(format!(
                                "Conditional catch '{}' has no valid successor: {error}",
                                catch.node_id
                            ))
                        })?;
                    let token = {
                        let inst_arc = self
                            .instances
                            .get(&instance_id)
                            .await
                            .ok_or(EngineError::NoSuchInstance(instance_id))?;
                        let mut inst = inst_arc.write().await;
                        let mut token = inst.tokens.remove(&catch.token_id).ok_or_else(|| {
                            EngineError::InvalidDefinition(format!(
                                "Token {} not found in instance",
                                catch.token_id
                            ))
                        })?;
                        token.variables = variables.clone();
                        token.current_node = next.clone();
                        inst.current_node = next;
                        if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                            inst.state = InstanceState::Running;
                        }
                        inst.push_audit_log(format!(
                            "⏳ Conditional catch '{}' fired ({}); continuing",
                            catch.node_id, catch.condition
                        ));
                        token
                    };

                    self.remove_persisted_conditional_catch(catch.id).await;
                    self.record_conditional_caught(&catch).await;
                    fired.push(token);
                }
                Some(BpmnElement::BoundaryConditionalEvent {
                    cancel_activity: true,
                    ..
                }) => {
                    self.clear_wait_states_for_token(instance_id, &catch.token_id)
                        .await;
                    self.cancel_pending_activity(instance_id, &catch.token_id)
                        .await;

                    let next =
                        resolve_next_target(&def, &catch.node_id, &variables).map_err(|error| {
                            EngineError::InvalidDefinition(format!(
                                "Boundary conditional event '{}' has no valid successor: {error}",
                                catch.node_id
                            ))
                        })?;
                    let token = {
                        let inst_arc = self
                            .instances
                            .get(&instance_id)
                            .await
                            .ok_or(EngineError::NoSuchInstance(instance_id))?;
                        let mut inst = inst_arc.write().await;
                        let mut token = inst.tokens.remove(&catch.token_id).ok_or_else(|| {
                            EngineError::InvalidDefinition(format!(
                                "Token {} not found in instance",
                                catch.token_id
                            ))
                        })?;
                        token.variables = variables.clone();
                        token.current_node = next.clone();
                        inst.current_node = next;
                        if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                            inst.state = InstanceState::Running;
                        }
                        inst.push_audit_log(format!(
                            "⏳ Interrupting boundary conditional '{}' fired ({}); activity cancelled",
                            catch.node_id, catch.condition
                        ));
                        token
                    };

                    self.remove_persisted_conditional_catch(catch.id).await;
                    self.record_conditional_caught(&catch).await;
                    fired.push(token);
                }
                Some(BpmnElement::BoundaryConditionalEvent {
                    cancel_activity: false,
                    ..
                }) => {
                    let next =
                        resolve_next_target(&def, &catch.node_id, &variables).map_err(|error| {
                            EngineError::InvalidDefinition(format!(
                                "Boundary conditional event '{}' has no valid successor: {error}",
                                catch.node_id
                            ))
                        })?;
                    let spawned = {
                        let inst_arc = self
                            .instances
                            .get(&instance_id)
                            .await
                            .ok_or(EngineError::NoSuchInstance(instance_id))?;
                        let mut inst = inst_arc.write().await;
                        let mut spawned =
                            inst.tokens.get(&catch.token_id).cloned().ok_or_else(|| {
                                EngineError::InvalidDefinition(format!(
                                    "Token {} not found in instance",
                                    catch.token_id
                                ))
                            })?;
                        spawned.id = Uuid::new_v4();
                        spawned.variables = variables.clone();
                        spawned.current_node = next.clone();
                        inst.tokens.insert(spawned.id, spawned.clone());
                        let branch_index = inst.active_tokens.len();
                        inst.active_tokens.push(ActiveToken {
                            token: spawned.clone(),
                            completed: false,
                            fork_id: Some(catch.node_id.clone()),
                            branch_index,
                        });
                        if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                            inst.state = InstanceState::ParallelExecution {
                                active_token_count: inst.tokens.len(),
                            };
                        }
                        inst.push_audit_log(format!(
                            "⏳ Non-interrupting boundary conditional '{}' fired ({}); activity continues",
                            catch.node_id, catch.condition
                        ));
                        spawned
                    };

                    // Registriert lassen, aber erst nach einem erneuten false -> true feuern.
                    let mut still_pending = catch.clone();
                    still_pending.last_result = true;
                    self.pending_conditional_catches
                        .insert(still_pending.id, still_pending.clone());
                    self.persist_conditional_catch(still_pending.id).await;

                    self.record_conditional_caught(&still_pending).await;
                    fired.push(spawned);
                }
                _ => {
                    tracing::warn!(
                        "Instance {instance_id}: conditional catch {} points to non-conditional node '{}' — dropping",
                        catch.id,
                        catch.node_id
                    );
                    self.remove_persisted_conditional_catch(catch.id).await;
                }
            }
        }

        Ok(fired)
    }

    async fn record_conditional_caught(&self, catch: &PendingConditionalCatch) {
        self.record_history_event(
            catch.instance_id,
            HistoryEventType::ConditionalCaught,
            &format!(
                "Conditional event '{}' caught ({})",
                catch.node_id, catch.condition
            ),
            ActorType::Engine,
            None,
            None,
        )
        .await;
    }
}
