use super::WorkflowEngine;
use crate::InstanceState;
use crate::domain::BpmnElement;
use crate::domain::{EngineError, EngineResult};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

impl WorkflowEngine {
    pub async fn correlate_message(
        &self,
        message_name: String,
        business_key: Option<String>,
        variables: HashMap<String, Value>,
    ) -> EngineResult<Vec<Uuid>> {
        let mut affected_instances = Vec::new();
        let mut to_resume = Vec::new();

        for catch in self.pending_message_catches.iter() {
            if catch.kind == crate::runtime::CatchKind::Message
                && catch.message_name == message_name
                && let Some(inst_arc) = self.instances.get(&catch.instance_id).await
            {
                let inst = inst_arc.read().await;
                if matches!(
                    inst.state,
                    InstanceState::Suspended { .. }
                        | InstanceState::Completed
                        | InstanceState::CompletedWithError { .. }
                ) {
                    continue;
                }
                if let Some(ref bk) = business_key
                    && &inst.business_key != bk
                {
                    continue;
                }
                to_resume.push(catch.id);
                affected_instances.push(catch.instance_id);
            }
        }

        for catch_id in to_resume {
            // An interrupting sibling (multi-instance boundary, event gateway) may
            // already have removed this catch.
            if !self.pending_message_catches.contains_key(&catch_id) {
                continue;
            }
            self.resume_catch(catch_id, &variables, &format!("Message '{message_name}'"))
                .await?;
        }

        let mut defs_to_start = Vec::new();
        let all_defs = self.definitions.all();
        for (def_key, def) in &all_defs {
            if let Some((
                _,
                BpmnElement::MessageStartEvent {
                    message_name: ref_msg,
                },
            )) = def.start_event()
                && ref_msg == &message_name
            {
                defs_to_start.push(*def_key);
            }
        }

        for def_key in defs_to_start {
            let new_id = self
                .start_instance_with_variables(def_key, variables.clone())
                .await?;
            if let Some(ref bk) = business_key {
                if let Some(inst_arc) = self.instances.get(&new_id).await {
                    let mut inst = inst_arc.write().await;
                    inst.business_key = bk.clone();
                }
                self.persist_instance(new_id).await;
            }
            affected_instances.push(new_id);
        }

        Ok(affected_instances)
    }

    /// Resumes one pending message/signal catch: event sub-process trigger,
    /// interrupting or non-interrupting boundary, or a waiting catch event.
    pub(crate) async fn resume_catch(
        &self,
        catch_id: Uuid,
        variables: &HashMap<String, Value>,
        event_label: &str,
    ) -> EngineResult<()> {
        if let Some(catch) = self
            .pending_message_catches
            .get(&catch_id)
            .map(|c| c.clone())
            && self
                .is_event_subscription(catch.instance_id, catch.token_id)
                .await
        {
            return self
                .fire_event_subscription(catch.instance_id, catch.token_id, variables)
                .await;
        }
        let Some(catch) = self
            .pending_message_catches
            .remove(&catch_id)
            .map(|(_, catch)| catch)
        else {
            return Ok(()); // A concurrent gateway winner already consumed/cancelled this delivery.
        };

        if !self
            .event_work_valid(catch.instance_id, catch.token_id)
            .await
        {
            self.remove_persisted_message_catch(catch.id).await;
            return Ok(());
        }
        if let Some(won) = self
            .claim_gateway_delivery(
                catch.instance_id,
                catch.token_id,
                &catch.node_id,
                catch.id,
                variables,
            )
            .await?
        {
            self.remove_persisted_message_catch(catch.id).await;
            if won {
                self.run_conditional_work(catch.instance_id).await?;
            }
            return Ok(());
        }
        // Event-Based Gateway support: If this message catch triggered, clear any sibling wait states
        self.clear_wait_states_for_token(catch.instance_id, &catch.token_id)
            .await;

        if catch.token_id.is_nil() {
            self.pending_message_catches.insert(catch.id, catch);
            return Err(EngineError::InvalidDefinition(
                "Legacy nil-token scope subscription requires migration".into(),
            ));
        }

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

        let mut is_non_interrupting = false;
        if let Some(
            crate::domain::BpmnElement::BoundaryMessageEvent {
                cancel_activity: false,
                ..
            }
            | crate::domain::BpmnElement::BoundarySignalEvent {
                cancel_activity: false,
                ..
            },
        ) = def.nodes.get(&catch.node_id)
        {
            is_non_interrupting = true;
        }

        // Retrieve token from central store
        let mut token = {
            let inst_arc = self
                .instances
                .get(&catch.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(catch.instance_id))?;
            let mut inst = inst_arc.write().await;

            if is_non_interrupting {
                let mut original = inst
                    .tokens
                    .get(&catch.token_id)
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition(format!(
                            "Token {} not found in instance",
                            catch.token_id
                        ))
                    })?
                    .clone();

                original.id = uuid::Uuid::new_v4();
                inst.tokens.insert(original.id, original.clone());

                let active = crate::runtime::ActiveToken {
                    token: original.clone(),
                    completed: false,
                    fork_id: Some(original.current_node.clone()),
                    branch_index: inst.active_tokens.len(),
                };
                inst.active_tokens.push(active);

                if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                    inst.state = InstanceState::ParallelExecution {
                        active_token_count: inst.tokens.len(),
                    };
                }

                original
            } else {
                inst.tokens.remove(&catch.token_id).ok_or_else(|| {
                    EngineError::InvalidDefinition(format!(
                        "Token {} not found in instance",
                        catch.token_id
                    ))
                })?
            }
        };

        if !is_non_interrupting {
            self.interrupt_loop_activity(catch.instance_id, &mut token, &def, &catch.node_id)
                .await;
            let node_to_cancel = &token.current_node;

            let mut to_remove_ut = Vec::new();
            for r in self.pending_user_tasks.iter() {
                if r.value().instance_id == catch.instance_id
                    && r.value().node_id == *node_to_cancel
                    && r.value().token_id == catch.token_id
                {
                    to_remove_ut.push(*r.key());
                }
            }
            for id in to_remove_ut {
                self.pending_user_tasks.remove(&id);
                self.remove_persisted_user_task(id).await;
            }

            let mut to_remove_st = Vec::new();
            for r in self.pending_service_tasks.iter() {
                if r.value().instance_id == catch.instance_id
                    && r.value().node_id == *node_to_cancel
                    && r.value().token_id == catch.token_id
                {
                    to_remove_st.push(*r.key());
                }
            }
            for id in to_remove_st {
                self.remove_pending_service_task(&id);
                self.remove_persisted_service_task(id).await;
            }
        }

        token.discard_data_scope();
        token.variables.extend(variables.clone());

        let old_state = if let Some(lk) = self.instances.get(&catch.instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };

        let _def_key_audit = {
            let inst_arc = self
                .instances
                .get(&catch.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(catch.instance_id))?;
            let mut inst = inst_arc.write().await;
            if !matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                inst.state = InstanceState::Running;
            }
            inst.push_audit_log(format!("{event_label} correlated, resuming '{catch_id}'"));
            inst.definition_key
        };

        self.record_history_event(
            catch.instance_id,
            crate::history::HistoryEventType::TokenAdvanced,
            &format!("{event_label} correlated"),
            crate::history::ActorType::Engine,
            None,
            old_state.as_ref(),
        )
        .await;

        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;
        if def.has_conditionals() {
            self.run_end_scripts(catch.instance_id, &mut token, &def, &catch.node_id)
                .await?;
            self.run_conditional_work(catch.instance_id).await?;
            if let Some(instance) = self.instances.get(&catch.instance_id).await
                && !crate::engine::event_subprocess::token_valid(&*instance.read().await, &token)
            {
                return Ok(());
            }
        }
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

        self.remove_persisted_message_catch(catch_id).await;
        self.run_instance_batch(catch.instance_id, token).await?;
        Ok(())
    }
}
