use super::WorkflowEngine;
use crate::domain::{BpmnElement, EngineError, EngineResult, ProcessDefinition, Token};
use crate::runtime::{ConditionalWait, EventWaitGroup, NextAction, ProcessInstance};
use std::collections::HashMap;
use uuid::Uuid;

impl WorkflowEngine {
    pub(crate) fn evaluate_conditional_catches(
        instance: &mut ProcessInstance,
        definition: &ProcessDefinition,
    ) {
        let Some(state) = &instance.event_state else {
            return;
        };
        let mut waits: Vec<_> = state.conditional_waits.values().cloned().collect();
        waits.sort_by_key(|wait| (wait.order, wait.id));
        for wait in waits {
            let Some(BpmnElement::ConditionalCatchEvent { condition }) =
                definition.nodes.get(&wait.node_id)
            else {
                continue;
            };
            if wait.claimed || !definition.evaluate_event_condition(condition, &instance.variables)
            {
                continue;
            }
            let state = instance.event_state.as_mut().unwrap();
            if wait.generation != state.generation || !instance.tokens.contains_key(&wait.token_id)
            {
                continue;
            }
            if let Some(group_id) = wait.group {
                let Some(group) = state.gateway_waits.get_mut(&wait.token_id) else {
                    continue;
                };
                if group.id != group_id || group.winner.is_some() {
                    continue;
                }
                group.winner = Some(wait.node_id.clone());
                state
                    .conditional_waits
                    .retain(|id, other| other.group != Some(group_id) || *id == wait.id);
            }
            if let Some(token) = instance.tokens.get_mut(&wait.token_id) {
                token.current_node = wait.node_id.clone();
                token.conditional_wait = Some(wait.id);
            }
            let revision = state.variable_revision;
            let claimed = state.conditional_waits.get_mut(&wait.id).unwrap();
            claimed.claimed = true;
            claimed.claimed_revision = Some(revision);
            state.ready_conditional_tokens.push(wait.token_id);
        }
    }

    pub(crate) fn register_conditional_wait(
        instance: &mut ProcessInstance,
        token: &Token,
        node: &str,
        group: Option<Uuid>,
        order: usize,
    ) {
        let state = instance.event_state.as_mut().unwrap();
        if state
            .conditional_waits
            .values()
            .any(|wait| wait.token_id == token.id && wait.node_id == node)
        {
            return;
        }
        let id = Uuid::new_v4();
        state.conditional_waits.insert(
            id,
            ConditionalWait {
                registered_revision: state.variable_revision,
                claimed_revision: None,
                id,
                token_id: token.id,
                node_id: node.into(),
                generation: state.generation,
                claimed: false,
                group,
                order,
            },
        );
        state.lifecycle_outbox.push(
            crate::history::HistoryEntry::new(
                instance.id,
                crate::history::HistoryEventType::TokenAdvanced,
                "Conditional catch registered",
                crate::history::ActorType::Engine,
                None,
            )
            .with_node(node)
            .with_context("occurrence_id", serde_json::json!(id))
            .with_context("token_id", serde_json::json!(token.id))
            .with_context("generation", serde_json::json!(state.generation))
            .with_context("definition_key", serde_json::json!(instance.definition_key)),
        );
        instance.tokens.insert(token.id, token.clone());
    }

    pub(crate) async fn handle_conditional_catch(
        &self,
        id: Uuid,
        token: &mut Token,
        definition: &ProcessDefinition,
        node: &str,
    ) -> EngineResult<NextAction> {
        let instance = self
            .instances
            .get(&id)
            .await
            .ok_or(EngineError::NoSuchInstance(id))?;
        let mut instance = instance.write().await;
        if !super::event_subprocess::token_valid(&instance, token) {
            return Err(EngineError::AlreadyCompleted);
        }
        if matches!(
            instance.state,
            crate::runtime::InstanceState::Suspended { .. }
        ) {
            if token.conditional_wait.is_none() {
                Self::register_conditional_wait(&mut instance, token, node, None, 0);
            }
            return Ok(NextAction::WaitForConditional {
                node_id: node.into(),
            });
        }
        let claimed = instance
            .event_state
            .as_ref()
            .and_then(|state| {
                state
                    .conditional_waits
                    .values()
                    .find(|wait| wait.token_id == token.id && wait.node_id == node && wait.claimed)
            })
            .cloned();
        if let Some(wait) = claimed {
            if token.conditional_wait != Some(wait.id) {
                return Err(EngineError::AlreadyCompleted);
            }
            let mut candidate = token.clone();
            candidate.variables = instance.variables.clone();
            if let Some(context) = &mut candidate.event_context {
                context.baseline = instance.variables.clone();
            }
            let mut audits = Vec::new();
            let mut variables = instance.variables.clone();
            crate::scripting::run_end_scripts(
                &self.scripts,
                id,
                &mut candidate,
                definition,
                node,
                &mut audits,
                &mut variables,
            )
            .await?;
            let next =
                super::executor::resolve_next_target(definition, node, &candidate.variables)?;
            candidate.current_node = next;
            candidate.conditional_wait = None;
            instance.variables = variables;
            instance.audit_log.extend(audits);
            let definition_key = instance.definition_key;
            let state = instance.event_state.as_mut().unwrap();
            state.conditional_waits.retain(|_, other| {
                other.token_id != token.id || (wait.group.is_none() && other.id != wait.id)
            });
            let entry = crate::history::HistoryEntry::new(
                id,
                crate::history::HistoryEventType::TokenAdvanced,
                if matches!(
                    definition.nodes.get(node),
                    Some(BpmnElement::ConditionalCatchEvent { .. })
                ) {
                    "Conditional catch consumed"
                } else {
                    "Event-gateway catch consumed"
                },
                crate::history::ActorType::Engine,
                None,
            )
            .with_node(node)
            .with_context("occurrence_id", serde_json::json!(wait.id))
            .with_context("token_id", serde_json::json!(token.id))
            .with_context("generation", serde_json::json!(wait.generation))
            .with_context("group_id", serde_json::json!(wait.group))
            .with_context(
                "registered_revision",
                serde_json::json!(wait.registered_revision),
            )
            .with_context("claimed_revision", serde_json::json!(wait.claimed_revision))
            .with_context(
                "variable_revision",
                serde_json::json!(state.variable_revision),
            )
            .with_context("definition_key", serde_json::json!(definition_key));
            state.lifecycle_outbox.push(entry);
            instance.tokens.insert(candidate.id, candidate.clone());
            Self::evaluate_conditionals(&mut instance, definition)?;
            *token = candidate;
            drop(instance);
            self.persist_instance(id).await;
            return Ok(NextAction::Continue(token.clone()));
        }
        if token.conditional_wait.is_some() {
            return Err(EngineError::AlreadyCompleted);
        }
        Self::register_conditional_wait(&mut instance, token, node, None, 0);
        Self::evaluate_conditionals(&mut instance, definition)?;
        Ok(NextAction::WaitForConditional {
            node_id: node.into(),
        })
    }

    /// All event-gateway alternatives publish through the same durable winner transaction.
    pub(crate) async fn claim_gateway_delivery(
        &self,
        id: Uuid,
        token_id: Uuid,
        node: &str,
        job: Uuid,
        payload: &HashMap<String, serde_json::Value>,
    ) -> EngineResult<Option<bool>> {
        let Some(instance) = self.instances.get(&id).await else {
            return Ok(Some(false));
        };
        let mut instance = instance.write().await;
        let definition = self
            .definitions
            .get(&instance.definition_key)
            .ok_or(EngineError::NoSuchDefinition(instance.definition_key))?;
        let gateway = definition.flows.iter().find_map(|(source, flows)| {
            (matches!(
                definition.nodes.get(source),
                Some(BpmnElement::EventBasedGateway)
            ) && flows.iter().any(|flow| flow.target == node))
            .then(|| source.clone())
        });
        let Some(gateway) = gateway else {
            return Ok(None);
        };
        let Some(mut token) = instance.tokens.get(&token_id).cloned() else {
            return Ok(Some(false));
        };
        if token.current_node != gateway || !super::event_subprocess::token_valid(&instance, &token)
        {
            return Ok(Some(false));
        }
        if instance.event_state.is_none() {
            instance.event_state = Some(Default::default());
        }
        let mut jobs = HashMap::from([(job, node.into())]);
        for pending in self
            .pending_timers
            .iter()
            .filter(|pending| pending.instance_id == id && pending.token_id == token_id)
        {
            jobs.insert(pending.id, pending.node_id.clone());
        }
        for pending in self
            .pending_message_catches
            .iter()
            .filter(|pending| pending.instance_id == id && pending.token_id == token_id)
        {
            jobs.insert(pending.id, pending.node_id.clone());
        }
        let state = instance.event_state.as_mut().unwrap();
        let group = state
            .gateway_waits
            .entry(token_id)
            .or_insert_with(|| EventWaitGroup {
                id: Uuid::new_v4(),
                timers: Vec::new(),
                messages: Vec::new(),
                gateway,
                jobs,
                winner: None,
                cleaned: false,
            });
        if group.winner.is_some() || group.jobs.get(&job).is_none_or(|target| target != node) {
            return Ok(Some(false));
        }
        group.winner = Some(node.into());
        let group_id = group.id;
        let generation = state.generation;
        state
            .conditional_waits
            .retain(|_, wait| wait.token_id != token_id);
        let occurrence = Uuid::new_v4();
        state.conditional_waits.insert(
            occurrence,
            ConditionalWait {
                registered_revision: state.variable_revision,
                claimed_revision: Some(state.variable_revision),
                id: occurrence,
                token_id,
                node_id: node.into(),
                generation,
                claimed: true,
                group: Some(group_id),
                order: 0,
            },
        );
        state.ready_conditional_tokens.push(token_id);
        instance.variables.extend(payload.clone());
        token.variables = instance.variables.clone();
        if let Some(context) = &mut token.event_context {
            context.baseline = instance.variables.clone();
            context.started_node = None;
        }
        if token.event_context.is_none() {
            token.event_context = Some(Box::new(crate::domain::token::EventContext {
                generation,
                activation: None,
                started_node: None,
                baseline: instance.variables.clone(),
            }));
        }
        token.current_node = node.into();
        token.conditional_wait = Some(occurrence);
        instance.tokens.insert(token_id, token);
        Self::evaluate_conditionals(&mut instance, &definition)?;
        drop(instance);
        self.persist_instance(id).await;
        Ok(Some(true))
    }

    pub(crate) async fn upgrade_gateway_groups(&self, id: Uuid) -> EngineResult<()> {
        let Some(instance) = self.instances.get(&id).await else {
            return Ok(());
        };
        let mut instance = instance.write().await;
        let definition = self
            .definitions
            .get(&instance.definition_key)
            .ok_or(EngineError::NoSuchDefinition(instance.definition_key))?;
        let tokens: Vec<_> = instance
            .tokens
            .values()
            .filter(|token| {
                matches!(
                    definition.nodes.get(&token.current_node),
                    Some(BpmnElement::EventBasedGateway)
                ) && instance
                    .event_state
                    .as_ref()
                    .is_none_or(|state| !state.gateway_waits.contains_key(&token.id))
            })
            .cloned()
            .collect();
        for mut token in tokens {
            let timers: Vec<_> = self
                .pending_timers
                .iter()
                .filter(|job| job.instance_id == id && job.token_id == token.id)
                .map(|job| job.clone())
                .collect();
            let messages: Vec<_> = self
                .pending_message_catches
                .iter()
                .filter(|job| job.instance_id == id && job.token_id == token.id)
                .map(|job| job.clone())
                .collect();
            let jobs: HashMap<_, _> = timers
                .iter()
                .map(|job| (job.id, job.node_id.clone()))
                .chain(messages.iter().map(|job| (job.id, job.node_id.clone())))
                .collect();
            let targets: std::collections::HashSet<_> = jobs.values().collect();
            if jobs.is_empty()
                || jobs.len() != definition.next_nodes(&token.current_node).len()
                || targets.len() != jobs.len()
                || jobs.values().any(|target| {
                    !definition
                        .next_nodes(&token.current_node)
                        .iter()
                        .any(|flow| &flow.target == target)
                })
            {
                return Err(EngineError::InvalidDefinition(
                    "Cannot reconstruct an unambiguous legacy event-gateway wait".into(),
                ));
            }
            let state = instance.event_state.get_or_insert_with(Default::default);
            let generation = state.generation;
            state.gateway_waits.insert(
                token.id,
                EventWaitGroup {
                    id: Uuid::new_v4(),
                    gateway: token.current_node.clone(),
                    jobs,
                    timers,
                    messages,
                    winner: None,
                    cleaned: false,
                },
            );
            if token.event_context.is_none() {
                token.event_context = Some(Box::new(crate::domain::token::EventContext {
                    generation,
                    activation: None,
                    started_node: None,
                    baseline: instance.variables.clone(),
                }));
            }
            instance.tokens.insert(token.id, token);
        }
        Ok(())
    }

    pub(crate) async fn resume_claimed_catches(&self, id: Uuid) -> EngineResult<()> {
        let Some(instance) = self.instances.get(&id).await else {
            return Ok(());
        };
        let tokens: Vec<_> = {
            let instance = instance.read().await;
            let Some(definition) = self.definitions.get(&instance.definition_key) else {
                return Ok(());
            };
            if !definition.has_conditionals() {
                return Ok(());
            }
            let mut tokens: Vec<_> = instance
                .tokens
                .values()
                .filter(|token| {
                    let waiting = instance.event_state.as_ref().is_some_and(|state| {
                        state
                            .conditional_waits
                            .values()
                            .any(|wait| wait.token_id == token.id && !wait.claimed)
                            || state.gateway_waits.get(&token.id).is_some_and(|group| {
                                group.winner.is_none() && group.gateway == token.current_node
                            })
                    });
                    !waiting
                        && !self
                            .pending_user_tasks
                            .iter()
                            .any(|job| job.instance_id == id && job.token_id == token.id)
                        && !self
                            .pending_service_tasks
                            .iter()
                            .any(|job| job.instance_id == id && job.token_id == token.id)
                        && !self
                            .pending_timers
                            .iter()
                            .any(|job| job.instance_id == id && job.token_id == token.id)
                        && !self
                            .pending_message_catches
                            .iter()
                            .any(|job| job.instance_id == id && job.token_id == token.id)
                        && !instance
                            .outstanding_calls
                            .values()
                            .any(|call| call.token.id == token.id)
                        && !instance.join_barriers.values().any(|barrier| {
                            barrier
                                .arrived_tokens
                                .iter()
                                .any(|arrived| arrived.id == token.id)
                        })
                })
                .cloned()
                .collect();
            tokens.sort_by_key(|token| (token.conditional_wait.is_none(), token.id));
            tokens
        };
        for token in tokens {
            Box::pin(self.run_instance_batch(id, token)).await?;
        }
        Ok(())
    }

    pub async fn get_pending_conditional_catches(&self) -> Vec<serde_json::Value> {
        let mut result = Vec::new();
        for (id, instance) in self.instances.all().await {
            let instance = instance.read().await;
            if let Some(state) = &instance.event_state {
                let Some(definition) = self.definitions.get(&instance.definition_key) else {
                    continue;
                };
                for wait in state.conditional_waits.values().filter(|wait| {
                    matches!(
                        definition.nodes.get(&wait.node_id),
                        Some(BpmnElement::ConditionalCatchEvent { .. })
                    )
                }) {
                    result.push(serde_json::json!({ "id":wait.id, "instance_id":id, "node_id":wait.node_id, "token_id":wait.token_id, "claimed":wait.claimed, "generation":wait.generation }));
                }
            }
        }
        result
    }

    pub(crate) async fn restore_gateway_jobs(&self, id: Uuid) {
        let Some(instance) = self.instances.get(&id).await else {
            return;
        };
        let groups: Vec<_> = {
            let instance = instance.read().await;
            if matches!(
                instance.state,
                crate::runtime::InstanceState::Suspended { .. }
                    | crate::runtime::InstanceState::Completed
                    | crate::runtime::InstanceState::CompletedWithError { .. }
            ) {
                return;
            }
            instance
                .event_state
                .as_ref()
                .map(|state| {
                    state
                        .gateway_waits
                        .iter()
                        .filter(|(token, group)| {
                            group.winner.is_none()
                                && (group
                                    .timers
                                    .iter()
                                    .any(|job| !self.pending_timers.contains_key(&job.id))
                                    || group.messages.iter().any(|job| {
                                        !self.pending_message_catches.contains_key(&job.id)
                                    }))
                                && instance
                                    .tokens
                                    .get(token)
                                    .is_some_and(|token| token.current_node == group.gateway)
                        })
                        .map(|(_, group)| group.clone())
                        .collect()
                })
                .unwrap_or_default()
        };
        if groups.is_empty() {
            return;
        }
        self.persist_instance(id).await;
        for group in groups {
            for job in group.timers.clone() {
                let guard = instance.write().await;
                if guard
                    .event_state
                    .as_ref()
                    .and_then(|state| {
                        state
                            .gateway_waits
                            .values()
                            .find(|current| current.id == group.id)
                    })
                    .is_none_or(|current| current.winner.is_some())
                {
                    continue;
                }

                if !self.pending_timers.contains_key(&job.id) {
                    let job_id = job.id;
                    self.insert_pending_timer(job);
                    drop(guard);
                    self.persist_timer(job_id).await;
                }
            }
            for job in group.messages.clone() {
                let guard = instance.write().await;
                if guard
                    .event_state
                    .as_ref()
                    .and_then(|state| {
                        state
                            .gateway_waits
                            .values()
                            .find(|current| current.id == group.id)
                    })
                    .is_none_or(|current| current.winner.is_some())
                {
                    continue;
                }

                if !self.pending_message_catches.contains_key(&job.id) {
                    let job_id = job.id;
                    self.pending_message_catches.insert(job_id, job);
                    drop(guard);
                    self.persist_message_catch(job_id).await;
                }
            }
        }
    }

    pub(crate) async fn cleanup_gateway_losers(&self, id: Uuid) {
        let Some(instance) = self.instances.get(&id).await else {
            return;
        };
        let groups: Vec<_> = instance
            .read()
            .await
            .event_state
            .as_ref()
            .map(|state| {
                state
                    .gateway_waits
                    .iter()
                    .filter(|(_, group)| {
                        group.winner.is_some()
                            && (!group.cleaned
                                || group.jobs.keys().any(|job| {
                                    self.pending_timers.contains_key(job)
                                        || self.pending_message_catches.contains_key(job)
                                }))
                    })
                    .map(|(token, group)| (*token, group.clone()))
                    .collect()
            })
            .unwrap_or_default();
        for (token, group) in groups {
            if !group.cleaned {
                instance.write().await.push_audit_log(format!(
                    "Event gateway '{}' alternatives cancelled (group {})",
                    group.gateway, group.id
                ));
            }
            for job in group.jobs.keys() {
                if self.remove_pending_timer(job).is_some() {
                    self.remove_persisted_timer(*job).await;
                }
                self.pending_message_catches.remove(job);
                self.remove_persisted_message_catch(*job).await;
            }
            if let Some(current) = instance
                .write()
                .await
                .event_state
                .as_mut()
                .and_then(|state| state.gateway_waits.get_mut(&token))
                && current.id == group.id
            {
                current.cleaned = true;
            }
        }
    }
}
