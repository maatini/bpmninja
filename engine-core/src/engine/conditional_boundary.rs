use super::WorkflowEngine;
use crate::domain::{BpmnElement, EngineError, EngineResult, ProcessDefinition, Token};
use crate::runtime::{BoundaryActivation, ConditionalWait, ProcessInstance};
use std::collections::HashMap;
use uuid::Uuid;

impl WorkflowEngine {
    pub(crate) fn register_conditional_boundaries(
        instance: &mut ProcessInstance,
        definition: &ProcessDefinition,
        token: &mut Token,
    ) {
        if !super::event_subprocess::token_valid(instance, token) {
            return;
        }
        if !definition.nodes.values().any(|node| matches!(node, BpmnElement::BoundaryConditionalEvent { attached_to, .. } if attached_to == &token.current_node)) { return; }
        let Some(state) = &mut instance.event_state else {
            return;
        };
        if let Some(existing) = state.boundary_activations.get(&token.id)
            && existing.node_id == token.current_node
            && token.boundary_activation == Some(existing.id)
        {
            return;
        }
        let id = Uuid::new_v4();
        token.boundary_activation = Some(id);
        let subscriptions: HashMap<_, _> = definition.nodes.iter().filter(|&(_node, element)| matches!(element, BpmnElement::BoundaryConditionalEvent { attached_to, .. } if attached_to == &token.current_node)).map(|(node, _element)| (node.clone(), Uuid::new_v4())).collect();
        for (node, subscription) in &subscriptions {
            state.lifecycle_outbox.push(
                crate::history::HistoryEntry::new(
                    instance.id,
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Conditional boundary registered",
                    crate::history::ActorType::Engine,
                    None,
                )
                .with_node(node)
                .with_context("subscription_id", serde_json::json!(subscription))
                .with_context("activity_activation_id", serde_json::json!(id))
                .with_context(
                    "variable_revision",
                    serde_json::json!(state.variable_revision),
                ),
            );
        }
        state.boundary_activations.insert(
            token.id,
            BoundaryActivation {
                tasks: Vec::new(),
                jobs: Default::default(),
                timers: Vec::new(),
                messages: Vec::new(),
                id,
                subscriptions,
                registered_revision: state.variable_revision,
                node_id: token.current_node.clone(),
                token: token.clone(),
                latches: HashMap::new(),
            },
        );
        instance.tokens.insert(token.id, token.clone());
    }

    pub(crate) fn track_boundary_task(
        instance: &mut ProcessInstance,
        token: &Token,
        task: crate::runtime::BoundaryTask,
    ) {
        if let Some(activation) = instance
            .event_state
            .as_mut()
            .and_then(|state| state.boundary_activations.get_mut(&token.id))
            .filter(|activation| Some(activation.id) == token.boundary_activation)
        {
            activation.tasks.push(task);
        }
    }

    pub(crate) async fn track_boundary_job(
        &self,
        id: Uuid,
        token: &Token,
        job: Uuid,
        timer: Option<crate::runtime::PendingTimer>,
        message: Option<crate::runtime::PendingMessageCatch>,
    ) -> EngineResult<()> {
        let Some(marker) = token.boundary_activation else {
            return Ok(());
        };
        let instance = self
            .instances
            .get(&id)
            .await
            .ok_or(EngineError::NoSuchInstance(id))?;
        let mut instance = instance.write().await;
        let activation = instance
            .event_state
            .as_mut()
            .and_then(|state| state.boundary_activations.get_mut(&token.id))
            .filter(|activation| activation.id == marker)
            .ok_or(EngineError::AlreadyCompleted)?;
        activation.jobs.insert(job);
        if let Some(timer) = timer {
            activation.timers.push(timer);
        }
        if let Some(message) = message {
            activation.messages.push(message);
        }
        drop(instance);
        self.persist_instance(id).await;
        Ok(())
    }

    pub(crate) async fn restore_conditional_boundary_jobs(&self, id: Uuid) {
        let Some(instance) = self.instances.get(&id).await else {
            return;
        };
        let activations = {
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
                        .boundary_activations
                        .values()
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        for activation in activations {
            for timer in activation.timers {
                let live = {
                    let instance = instance.write().await;
                    if super::event_subprocess::token_valid(&instance, &activation.token)
                        && !self.pending_timers.contains_key(&timer.id)
                    {
                        self.insert_pending_timer(timer.clone());
                        true
                    } else {
                        false
                    }
                };
                if live {
                    self.persist_timer(timer.id).await;
                }
            }
            for message in activation.messages {
                let live = {
                    let instance = instance.write().await;
                    if super::event_subprocess::token_valid(&instance, &activation.token)
                        && !self.pending_message_catches.contains_key(&message.id)
                    {
                        self.pending_message_catches
                            .insert(message.id, message.clone());
                        true
                    } else {
                        false
                    }
                };
                if live {
                    self.persist_message_catch(message.id).await;
                }
            }
        }
    }

    pub(crate) async fn cancel_boundary_job_ids(&self, jobs: &[Uuid]) {
        for job in jobs {
            if self.remove_pending_timer(job).is_some() {
                self.remove_persisted_timer(*job).await;
            }
            if self.pending_message_catches.remove(job).is_some() {
                self.remove_persisted_message_catch(*job).await;
            }
        }
    }

    /// Close only this visit. Clearing the caller marker allows its continuation
    /// while stale copies remain fenced by the removed activation identity.
    pub(crate) fn close_conditional_boundaries(instance: &mut ProcessInstance, token: &mut Token) {
        if let Some(id) = token.boundary_activation.take()
            && let Some(state) = &mut instance.event_state
            && state
                .boundary_activations
                .get(&token.id)
                .is_some_and(|activation| activation.id == id)
        {
            let activation = state.boundary_activations.remove(&token.id).unwrap();
            state.boundary_task_cleanup.extend(activation.tasks.clone());
            if let Some(context) = &mut token.event_context {
                context.started_node = None;
            }
            state.lifecycle_outbox.push(
                crate::history::HistoryEntry::new(
                    instance.id,
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Conditional boundary subscriptions closed",
                    crate::history::ActorType::Engine,
                    None,
                )
                .with_node(&activation.node_id)
                .with_context("activity_activation_id", serde_json::json!(activation.id)),
            );
        }
    }

    pub(crate) fn evaluate_conditional_boundaries(
        instance: &mut ProcessInstance,
        definition: &ProcessDefinition,
    ) {
        let Some(state) = &instance.event_state else {
            return;
        };
        let mut activations: Vec<_> = state.boundary_activations.values().cloned().collect();
        activations.sort_by_key(|activation| activation.id);
        for activation in activations {
            if !super::event_subprocess::token_valid(instance, &activation.token) {
                continue;
            }
            let mut boundaries: Vec<_> = definition
                .nodes
                .iter()
                .filter_map(|(node, element)| {
                    if let BpmnElement::BoundaryConditionalEvent {
                        attached_to,
                        condition,
                        cancel_activity,
                        order,
                    } = element
                        && attached_to == &activation.node_id
                    {
                        return Some((*order, node.clone(), condition.clone(), *cancel_activity));
                    }
                    None
                })
                .collect();
            boundaries.sort_by_key(|(order, node, _, _)| (*order, node.clone()));
            for (order, node, condition, interrupting) in boundaries {
                let current = definition.evaluate_event_condition(&condition, &instance.variables);
                let state = instance.event_state.as_mut().unwrap();
                let Some(live) = state.boundary_activations.get_mut(&activation.token.id) else {
                    break;
                };
                if live.id != activation.id {
                    break;
                }
                let previous = live.latches.insert(node.clone(), current).unwrap_or(false);
                if !current || previous {
                    continue;
                }
                let mut branch = activation.token.clone();
                branch.id = Uuid::new_v4();
                branch.current_node = node.clone();
                branch.boundary_activation = None;
                branch.discard_data_scope();
                branch.variables = instance.variables.clone();
                if let Some(context) = &mut branch.event_context {
                    context.baseline = instance.variables.clone();
                    context.started_node = None;
                }
                let occurrence = Uuid::new_v4();
                branch.conditional_wait = Some(occurrence);
                state.conditional_waits.insert(
                    occurrence,
                    ConditionalWait {
                        boundary_activation: Some(activation.id),
                        id: occurrence,
                        token_id: branch.id,
                        node_id: node.clone(),
                        generation: state.generation,
                        claimed: true,
                        group: None,
                        order,
                        registered_revision: activation.registered_revision,
                        claimed_revision: Some(state.variable_revision),
                    },
                );
                state.ready_conditional_tokens.push(branch.id);
                state.lifecycle_outbox.push(
                    crate::history::HistoryEntry::new(
                        instance.id,
                        crate::history::HistoryEventType::TokenAdvanced,
                        "Conditional boundary claimed",
                        crate::history::ActorType::Engine,
                        None,
                    )
                    .with_node(&node)
                    .with_context("activity_activation_id", serde_json::json!(activation.id))
                    .with_context("occurrence_id", serde_json::json!(occurrence))
                    .with_context("attached_to", serde_json::json!(activation.node_id))
                    .with_context("interrupting", serde_json::json!(interrupting))
                    .with_context("token_id", serde_json::json!(branch.id))
                    .with_context("generation", serde_json::json!(state.generation))
                    .with_context(
                        "variable_revision",
                        serde_json::json!(state.variable_revision),
                    )
                    .with_context("definition_key", serde_json::json!(instance.definition_key)),
                );
                instance.tokens.insert(branch.id, branch);
                if !interrupting && instance.tokens.len() > 1 {
                    instance.state = crate::runtime::InstanceState::ParallelExecution {
                        active_token_count: instance.tokens.len(),
                    };
                }
                if interrupting {
                    state.boundary_task_cleanup.extend(activation.tasks.clone());
                    state.boundary_activations.remove(&activation.token.id);
                    state.boundary_cleanup.push(activation.token.id);
                    instance.tokens.remove(&activation.token.id);
                    for active in &mut instance.active_tokens {
                        if active.token.id == activation.token.id {
                            active.completed = true;
                        }
                    }
                    break;
                }
            }
        }
    }

    pub(crate) async fn cleanup_stale_boundary_tasks(&self, id: Uuid) {
        let Some(instance) = self.instances.get(&id).await else {
            return;
        };
        let (users, services) = {
            let instance = instance.read().await;
            let Some(definition) = self.definitions.get(&instance.definition_key) else {
                return;
            };
            if !definition
                .nodes
                .values()
                .any(|node| matches!(node, BpmnElement::BoundaryConditionalEvent { .. }))
            {
                return;
            }
            let stale = |token: Uuid, node: &str| {
                instance.tokens.get(&token).is_some_and(|token| {
                    token.boundary_activation.is_none() && token.current_node != node
                })
            };
            (
                self.pending_user_tasks
                    .iter()
                    .filter(|job| job.instance_id == id && stale(job.token_id, &job.node_id))
                    .map(|job| job.task_id)
                    .collect::<Vec<_>>(),
                self.pending_service_tasks
                    .iter()
                    .filter(|job| job.instance_id == id && stale(job.token_id, &job.node_id))
                    .map(|job| job.id)
                    .collect::<Vec<_>>(),
            )
        };
        for job in users {
            self.pending_user_tasks.remove(&job);
            self.remove_persisted_user_task(job).await;
        }
        for job in services {
            self.remove_pending_service_task(&job);
            self.remove_persisted_service_task(job).await;
        }
    }

    pub(crate) async fn upgrade_conditional_boundaries(&self, id: Uuid) -> EngineResult<()> {
        let Some(instance) = self.instances.get(&id).await else {
            return Ok(());
        };
        let mut instance = instance.write().await;
        let Some(definition) = self.definitions.get(&instance.definition_key) else {
            return Ok(());
        };
        if !definition.has_conditionals() || instance.event_state.is_none() {
            return Ok(());
        }
        let tokens: Vec<_> = instance
            .tokens
            .values()
            .filter(|token| token.boundary_activation.is_none())
            .cloned()
            .collect();
        for mut token in tokens {
            let users = self
                .pending_user_tasks
                .iter()
                .filter(|job| {
                    job.instance_id == id
                        && job.token_id == token.id
                        && job.node_id == token.current_node
                })
                .count();
            let services = self
                .pending_service_tasks
                .iter()
                .filter(|job| {
                    job.instance_id == id
                        && job.token_id == token.id
                        && job.node_id == token.current_node
                })
                .count();
            let messages = self
                .pending_message_catches
                .iter()
                .filter(|job| {
                    job.instance_id == id
                        && job.token_id == token.id
                        && job.node_id == token.current_node
                })
                .count();
            if users + services + messages == 1 {
                Self::register_conditional_boundaries(&mut instance, &definition, &mut token);
                if let Some(activation) = instance
                    .event_state
                    .as_mut()
                    .and_then(|state| state.boundary_activations.get_mut(&token.id))
                {
                    activation.tasks.extend(
                        self.pending_user_tasks
                            .iter()
                            .filter(|job| {
                                job.instance_id == id
                                    && job.token_id == token.id
                                    && job.node_id == token.current_node
                            })
                            .map(|job| crate::runtime::BoundaryTask::User(job.task_id)),
                    );
                    activation.tasks.extend(
                        self.pending_service_tasks
                            .iter()
                            .filter(|job| {
                                job.instance_id == id
                                    && job.token_id == token.id
                                    && job.node_id == token.current_node
                            })
                            .map(|job| crate::runtime::BoundaryTask::Service(job.id)),
                    );
                    activation.tasks.extend(
                        self.pending_message_catches
                            .iter()
                            .filter(|job| {
                                job.instance_id == id
                                    && job.token_id == token.id
                                    && job.node_id == token.current_node
                            })
                            .map(|job| crate::runtime::BoundaryTask::Message(job.id)),
                    );
                    activation.timers.extend(
                        self.pending_timers
                            .iter()
                            .filter(|job| job.instance_id == id && job.token_id == token.id)
                            .map(|job| job.clone()),
                    );
                    activation.messages.extend(
                        self.pending_message_catches
                            .iter()
                            .filter(|job| {
                                job.instance_id == id
                                    && job.token_id == token.id
                                    && job.node_id != token.current_node
                            })
                            .map(|job| job.clone()),
                    );
                    activation.jobs.extend(
                        self.pending_timers
                            .iter()
                            .filter(|job| job.instance_id == id && job.token_id == token.id)
                            .map(|job| job.id),
                    );
                    activation.jobs.extend(
                        self.pending_message_catches
                            .iter()
                            .filter(|job| {
                                job.instance_id == id
                                    && job.token_id == token.id
                                    && job.node_id != token.current_node
                            })
                            .map(|job| job.id),
                    );
                }
            }
        }
        Ok(())
    }

    /// Old token IDs cannot belong to a later visit: every boundary interruption
    /// replaces the token. Retain cleanup intent until all external records go.
    pub(crate) async fn cleanup_conditional_boundaries(&self, id: Uuid) {
        let Some(instance) = self.instances.get(&id).await else {
            return;
        };
        let (ids, tasks) = instance
            .read()
            .await
            .event_state
            .as_ref()
            .map(|state| {
                (
                    state.boundary_cleanup.clone(),
                    state.boundary_task_cleanup.clone(),
                )
            })
            .unwrap_or_default();
        if ids.is_empty() && tasks.is_empty() {
            return;
        }
        self.persist_instance(id).await;
        for task in &tasks {
            match task {
                crate::runtime::BoundaryTask::User(job) => {
                    self.pending_user_tasks.remove(job);
                    self.remove_persisted_user_task(*job).await;
                }
                crate::runtime::BoundaryTask::Service(job) => {
                    self.remove_pending_service_task(job);
                    self.remove_persisted_service_task(*job).await;
                }
                crate::runtime::BoundaryTask::Message(job) => {
                    self.pending_message_catches.remove(job);
                    self.remove_persisted_message_catch(*job).await;
                }
            }
        }
        for token in &ids {
            let users: Vec<_> = self
                .pending_user_tasks
                .iter()
                .filter(|job| job.instance_id == id && job.token_id == *token)
                .map(|job| job.task_id)
                .collect();
            let services: Vec<_> = self
                .pending_service_tasks
                .iter()
                .filter(|job| job.instance_id == id && job.token_id == *token)
                .map(|job| job.id)
                .collect();
            let timers: Vec<_> = self
                .pending_timers
                .iter()
                .filter(|job| job.instance_id == id && job.token_id == *token)
                .map(|job| job.id)
                .collect();
            let catches: Vec<_> = self
                .pending_message_catches
                .iter()
                .filter(|job| job.instance_id == id && job.token_id == *token)
                .map(|job| job.id)
                .collect();
            for job in users {
                self.pending_user_tasks.remove(&job);
                self.remove_persisted_user_task(job).await;
            }
            for job in services {
                self.remove_pending_service_task(&job);
                self.remove_persisted_service_task(job).await;
            }
            for job in timers {
                self.remove_pending_timer(&job);
                self.remove_persisted_timer(job).await;
            }
            for job in catches {
                self.pending_message_catches.remove(&job);
                self.remove_persisted_message_catch(job).await;
            }
        }
        if let Some(state) = &mut instance.write().await.event_state {
            state.boundary_cleanup.retain(|token| !ids.contains(token));
            state
                .boundary_task_cleanup
                .retain(|task| !tasks.contains(task));
        }
        self.persist_instance(id).await;
    }
}

pub(crate) fn validate(definition: &ProcessDefinition) -> EngineResult<()> {
    for (node, element) in &definition.nodes {
        if let BpmnElement::BoundaryConditionalEvent {
            attached_to,
            condition,
            ..
        } = element
        {
            crate::condition::validate_event_condition(condition)?;
            if !matches!(
                definition.nodes.get(attached_to),
                Some(
                    BpmnElement::UserTask(_)
                        | BpmnElement::ServiceTask { .. }
                        | BpmnElement::ScriptTask { .. }
                        | BpmnElement::SendTask { .. }
                        | BpmnElement::MessageCatchEvent { .. }
                )
            ) || matches!(
                definition.nodes.get(attached_to),
                Some(
                    BpmnElement::ServiceTask {
                        multi_instance: Some(_),
                        ..
                    } | BpmnElement::ScriptTask {
                        multi_instance: Some(_),
                        ..
                    } | BpmnElement::SendTask {
                        multi_instance: Some(_),
                        ..
                    }
                )
            ) || definition.loops.contains_key(attached_to)
                || definition
                    .flows
                    .values()
                    .flatten()
                    .any(|flow| &flow.target == node)
            {
                return Err(EngineError::InvalidDefinition(format!(
                    "Conditional boundary '{node}' requires a non-loop task attachment and no incoming flows"
                )));
            }
        }
    }
    Ok(())
}
