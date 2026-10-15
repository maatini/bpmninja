//! Process-level event handlers. Subscriptions and activation fences live in instance snapshots.
use super::WorkflowEngine;
use crate::domain::token::EventContext;
use crate::domain::{
    EngineError, EngineResult, EventSubProcessTrigger as Trigger, ListenerEvent, ProcessDefinition,
    Token,
};
use crate::history::{ActorType, HistoryEntry, HistoryEventType};
use crate::runtime::{
    ActiveToken, CatchKind, EventSubProcessState, EventSubscription, InstanceState,
    PendingMessageCatch, PendingTimer, ProcessInstance,
};
use chrono::Utc;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn queue_lifecycle(
    instance: &mut ProcessInstance,
    activation: Uuid,
    handler: &str,
    event_type: HistoryEventType,
    reason: &str,
) {
    let actor = if event_type == HistoryEventType::ListenerExecuted {
        ActorType::Listener
    } else {
        ActorType::Engine
    };
    let mut entry = HistoryEntry::new(
        instance.id,
        event_type,
        format!("Event subprocess '{handler}' ({activation}): {reason}"),
        actor,
        None,
    )
    .with_node(handler)
    .with_context("activation_id", Value::String(activation.to_string()))
    .with_context(
        "definition_key",
        Value::String(instance.definition_key.to_string()),
    )
    .with_context("reason", Value::String(reason.into()));
    if let Some(state) = &mut instance.event_state {
        entry
            .context
            .insert("generation".into(), Value::from(state.generation));
        state.lifecycle_outbox.push(entry);
    }
}

fn queue_cancelled_activations(instance: &mut ProcessInstance, reason: &str) {
    let cancelled = instance
        .event_state
        .as_ref()
        .map(|state| state.activations.clone())
        .unwrap_or_default();
    for (activation, handler) in cancelled {
        queue_lifecycle(
            instance,
            activation,
            &handler,
            HistoryEventType::EventSubprocessInterrupted,
            reason,
        );
    }
}

pub(crate) fn token_valid(instance: &ProcessInstance, token: &Token) -> bool {
    if token.event_context.is_none() {
        return !matches!(&instance.state, InstanceState::CompletedWithError { error_code } if error_code == "SCOPE_INTERRUPTED");
    }
    token.event_context.as_ref().is_none_or(|context| {
        instance
            .event_state
            .as_ref()
            .is_some_and(|state| state.generation == context.generation)
            && !matches!(
                instance.state,
                InstanceState::Completed | InstanceState::CompletedWithError { .. }
            )
    })
}

/// Programmatic definitions use the same supported subset as deployed XML.
pub(crate) fn validate_definition(
    definition: &crate::domain::ProcessDefinition,
) -> EngineResult<()> {
    let invalid = |message: &str| EngineError::InvalidDefinition(message.into());
    if !definition.event_listeners.is_empty() {
        return Err(invalid(
            "Legacy ScopeEventListener requires migration to EventSubProcess",
        ));
    }
    if definition.event_subprocesses.is_empty() {
        return Ok(());
    }
    let mut ids = HashSet::new();
    let mut owners = HashMap::new();
    for (index, handler) in definition.event_subprocesses.iter().enumerate() {
        if handler.id.is_empty()
            || handler.id == definition.id
            || definition.nodes.contains_key(&handler.id)
            || !ids.insert(&handler.id)
        {
            return Err(invalid("Event subprocess IDs must be nonempty and unique"));
        }
        if matches!(handler.trigger, Trigger::Error(_)) && !handler.is_interrupting {
            return Err(invalid(
                "Error event subprocess starts must be interrupting",
            ));
        }
        if !handler.node_ids.contains(&handler.start_node)
            || !matches!(
                definition.nodes.get(&handler.start_node),
                Some(crate::domain::BpmnElement::StartEvent)
            )
        {
            return Err(invalid(
                "Event subprocess requires its own plain handler start node",
            ));
        }
        if !matches!(handler.trigger, Trigger::Timer(_))
            && definition.event_subprocesses[..index]
                .iter()
                .any(|other| other.trigger == handler.trigger)
        {
            return Err(invalid("Duplicate event subprocess trigger"));
        }
        for node in &handler.node_ids {
            if !definition.nodes.contains_key(node) || owners.insert(node, &handler.id).is_some() {
                return Err(invalid(
                    "Event subprocess nodes must exist and belong to one handler",
                ));
            }
        }
    }
    for (source, flows) in &definition.flows {
        if flows
            .iter()
            .any(|flow| owners.get(source) != owners.get(&flow.target))
        {
            return Err(invalid(
                "Sequence flows cannot cross an event subprocess boundary",
            ));
        }
    }
    Ok(())
}

impl WorkflowEngine {
    /// Container scripts are atomic process-variable updates, independent of task scopes.
    async fn run_event_container_listener(
        &self,
        instance: &mut ProcessInstance,
        token: &mut Token,
        definition: &ProcessDefinition,
        handler: &str,
        event: ListenerEvent,
    ) -> EngineResult<()> {
        if !definition
            .listeners
            .get(handler)
            .is_some_and(|listeners| listeners.iter().any(|listener| listener.event == event))
        {
            return Ok(());
        }
        let mut candidate = token.clone();
        // Refresh shared variables while retaining this activation's explicit payload.
        let baseline = &token.event_context.as_ref().unwrap().baseline;
        candidate.variables = instance.variables.clone();
        for key in baseline.keys() {
            if !token.variables.contains_key(key) {
                candidate.variables.remove(key);
            }
        }
        for (key, value) in &token.variables {
            if baseline.get(key) != Some(value) {
                candidate.variables.insert(key.clone(), value.clone());
            }
        }
        let before = crate::history::DiffSnapshot::from_instance(instance);
        let mut audits = Vec::new();
        crate::scripting::run_node_scripts(
            &self.scripts,
            instance.id,
            &mut candidate,
            definition,
            handler,
            event.clone(),
            &mut audits,
        )
        .await?;
        instance.variables = candidate.variables.clone();
        instance.audit_log.extend(audits);
        let context = candidate.event_context.as_mut().unwrap();
        context.baseline = instance.variables.clone();
        let activation = context.activation.unwrap();
        queue_lifecycle(
            instance,
            activation,
            handler,
            HistoryEventType::ListenerExecuted,
            if event == ListenerEvent::Start {
                "container start listener"
            } else {
                "container end listener"
            },
        );
        let diff = crate::history::calculate_diff_from_snapshot(&before, instance);
        if let Some(entry) = instance
            .event_state
            .as_mut()
            .and_then(|state| state.lifecycle_outbox.last_mut())
        {
            entry.context.insert(
                "listener_event".into(),
                Value::String(
                    if event == ListenerEvent::Start {
                        "start"
                    } else {
                        "end"
                    }
                    .into(),
                ),
            );
            entry.diff = Some(diff);
        }
        *token = candidate;
        Ok(())
    }

    pub(crate) async fn start_event_container(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        definition: &ProcessDefinition,
    ) -> EngineResult<()> {
        let Some(activation) = token
            .event_context
            .as_ref()
            .and_then(|context| context.activation)
        else {
            return Ok(());
        };
        let instance = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        {
            let mut instance = instance.write().await;
            if !token_valid(&instance, token) {
                return Err(EngineError::AlreadyCompleted);
            }
            let state = instance.event_state.as_ref().unwrap();
            let handler_id = state
                .activations
                .get(&activation)
                .ok_or(EngineError::AlreadyCompleted)?
                .clone();
            let handler = definition
                .event_subprocesses
                .iter()
                .find(|handler| handler.id == handler_id)
                .ok_or_else(|| {
                    EngineError::InvalidDefinition("Missing event subprocess container".into())
                })?;
            if token.current_node != handler.start_node
                || state.started_activations.contains(&activation)
                || !definition
                    .listeners
                    .get(&handler_id)
                    .is_some_and(|listeners| {
                        listeners
                            .iter()
                            .any(|listener| listener.event == ListenerEvent::Start)
                    })
            {
                return Ok(());
            }
            self.run_event_container_listener(
                &mut instance,
                token,
                definition,
                &handler_id,
                ListenerEvent::Start,
            )
            .await?;
            instance
                .event_state
                .as_mut()
                .unwrap()
                .started_activations
                .insert(activation);
            instance.tokens.insert(token.id, token.clone());
        }
        self.persist_instance(instance_id).await;
        self.flush_event_lifecycle(instance_id).await;
        Ok(())
    }

    /// Write durable, idempotent history before acknowledging the snapshot outbox.
    /// No instance lock is held during backend I/O or notification delivery.
    pub(crate) async fn flush_event_lifecycle(&self, instance_id: Uuid) {
        let _guard = self.event_lifecycle_locks.lock(instance_id).await;
        let Some(instance) = self.instances.get(&instance_id).await else {
            return;
        };
        let (key, mut entries) = {
            let instance = instance.read().await;
            (
                instance.definition_key,
                instance
                    .event_state
                    .as_ref()
                    .map(|state| state.lifecycle_outbox.clone())
                    .unwrap_or_default(),
            )
        };
        if entries.is_empty() {
            return;
        }
        self.persist_instance(instance_id).await;
        let version = self
            .definitions
            .get(&key)
            .map(|definition| definition.version);
        for entry in &mut entries {
            entry.definition_version = version;
        }
        if !self.flush_history(instance_id, entries.clone()).await {
            return;
        }
        let written: HashSet<_> = entries.iter().map(|entry| entry.id).collect();
        {
            let mut instance = instance.write().await;
            if let Some(state) = &mut instance.event_state {
                state
                    .lifecycle_outbox
                    .retain(|entry| !written.contains(&entry.id));
            }
        }
        self.persist_instance(instance_id).await;
        for entry in entries {
            let phase = match entry.event_type {
                HistoryEventType::EventSubprocessActivated => "activated",
                HistoryEventType::EventSubprocessInterrupted => "interrupted",
                HistoryEventType::EventSubprocessScopeInterrupted => "scope_interrupted",
                HistoryEventType::EventSubprocessCompleted => "completed",
                _ => continue,
            };
            if let Some(activation_id) = entry
                .context
                .get("activation_id")
                .and_then(Value::as_str)
                .and_then(|id| Uuid::parse_str(id).ok())
            {
                self.emit_event(super::EngineEvent::EventSubprocessChanged {
                    event_id: entry.id,
                    instance_id,
                    definition_key: key,
                    subprocess_id: entry.node_id.unwrap_or_default(),
                    activation_id,
                    phase: phase.into(),
                    reason: entry
                        .context
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                });
            }
        }
    }

    pub(crate) async fn event_work_valid(&self, instance_id: Uuid, token_id: Uuid) -> bool {
        let Some(instance) = self.instances.get(&instance_id).await else {
            return false;
        };
        let instance = instance.read().await;
        if instance.event_state.is_none() {
            return true;
        }
        instance
            .tokens
            .get(&token_id)
            .or_else(|| {
                instance
                    .outstanding_calls
                    .values()
                    .find(|call| call.token.id == token_id)
                    .map(|call| &call.token)
            })
            .is_some_and(|token| token_valid(&instance, token))
    }

    pub(crate) async fn initialize_event_subprocesses(
        &self,
        instance_id: Uuid,
        token: &mut Token,
    ) -> EngineResult<()> {
        let instance = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let key = instance.read().await.definition_key;
        let def = self
            .definitions
            .get(&key)
            .ok_or(EngineError::NoSuchDefinition(key))?;
        if !def.event_listeners.is_empty() {
            return Err(EngineError::InvalidDefinition("Legacy ScopeEventListener definitions are unsupported; migrate to EventSubProcess definitions".into()));
        }
        if def.event_subprocesses.is_empty() {
            return Ok(());
        }
        let mut state = EventSubProcessState::default();
        let mut timers = Vec::new();
        let mut catches = Vec::new();
        for handler in &def.event_subprocesses {
            let id = Uuid::new_v4();
            let (next_due, remaining) = match &handler.trigger {
                Trigger::Timer(timer) => (
                    timer.next_expiry(Utc::now()),
                    timer.initial_remaining_repetitions(),
                ),
                _ => (None, None),
            };
            if matches!(handler.trigger, Trigger::Timer(_)) && next_due.is_none() {
                continue;
            }
            state.subscriptions.insert(
                id,
                EventSubscription {
                    subprocess_id: handler.id.clone(),
                    next_due,
                    remaining,
                },
            );
            match &handler.trigger {
                Trigger::Timer(timer) => timers.push(PendingTimer {
                    id,
                    instance_id,
                    node_id: handler.start_node.clone(),
                    expires_at: next_due.unwrap(),
                    token_id: id,
                    timer_def: Some(timer.clone()),
                    remaining_repetitions: remaining,
                }),
                Trigger::Message(name) | Trigger::Signal(name) => {
                    catches.push(PendingMessageCatch {
                        id,
                        instance_id,
                        node_id: handler.start_node.clone(),
                        message_name: name.clone(),
                        token_id: id,
                        kind: if matches!(handler.trigger, Trigger::Signal(_)) {
                            CatchKind::Signal
                        } else {
                            CatchKind::Message
                        },
                    })
                }
                _ => {}
            }
        }
        token.event_context = Some(Box::new(EventContext {
            generation: 0,
            activation: None,
            started_node: None,
            baseline: token.variables.clone(),
        }));
        {
            let mut instance = instance.write().await;
            instance.event_state = Some(Box::new(state));
            instance.tokens.insert(token.id, token.clone());
            instance.active_tokens.push(ActiveToken {
                token: token.clone(),
                fork_id: None,
                branch_index: 0,
                completed: false,
            });
        }
        self.persist_instance(instance_id).await;
        for timer in timers {
            let id = timer.id;
            self.insert_pending_timer(timer);
            self.persist_timer(id).await;
        }
        for catch in catches {
            let id = catch.id;
            self.pending_message_catches.insert(id, catch);
            self.persist_message_catch(id).await;
        }
        Ok(())
    }

    pub(crate) async fn is_event_subscription(&self, instance_id: Uuid, id: Uuid) -> bool {
        if let Some(instance) = self.instances.get(&instance_id).await {
            return instance
                .read()
                .await
                .event_state
                .as_ref()
                .is_some_and(|s| s.subscriptions.contains_key(&id));
        }
        false
    }

    /// Claims and persists the handler before it can expose work; interruption invalidates old tokens.
    pub(crate) async fn prepare_event_activation(
        &self,
        instance_id: Uuid,
        subscription_id: Uuid,
        variables: &HashMap<String, Value>,
    ) -> EngineResult<Option<Token>> {
        let instance = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let key = instance.read().await.definition_key;
        let def = self
            .definitions
            .get(&key)
            .ok_or(EngineError::NoSuchDefinition(key))?;
        let (token, children, interrupted) = {
            let mut instance = instance.write().await;
            if matches!(
                instance.state,
                InstanceState::Completed
                    | InstanceState::CompletedWithError { .. }
                    | InstanceState::Suspended { .. }
            ) {
                return Ok(None);
            }
            let Some(subscription) = instance
                .event_state
                .as_ref()
                .and_then(|s| s.subscriptions.get(&subscription_id))
                .cloned()
            else {
                return Ok(None);
            };
            let handler = def
                .event_subprocesses
                .iter()
                .find(|h| h.id == subscription.subprocess_id)
                .ok_or_else(|| {
                    EngineError::InvalidDefinition(
                        "Event subscription refers to a missing handler".into(),
                    )
                })?;
            if matches!(handler.trigger, Trigger::Timer(_))
                && subscription.next_due.is_none_or(|due| due > Utc::now())
            {
                return Ok(None);
            }
            let mut children = Vec::new();
            if handler.is_interrupting {
                queue_cancelled_activations(
                    &mut instance,
                    "cancelled by interrupting event subprocess",
                );
                children.extend(instance.outstanding_calls.keys().copied());
                instance.tokens.clear();
                instance.active_tokens.clear();
                instance.join_barriers.clear();
                instance.multi_instance_state.clear();
                instance.outstanding_calls.clear();
                let state = instance.event_state.as_mut().unwrap();
                state.generation += 1;
                state.main_completed = true;
                state.subscriptions.clear();
                state.activations.clear();
                state.started_activations.clear();
            } else if let Trigger::Timer(timer) = &handler.trigger {
                let remaining = subscription.remaining.map(|n| n.saturating_sub(1));
                let next_due = if timer.is_recurring() && remaining != Some(0) {
                    timer.next_expiry(Utc::now())
                } else {
                    None
                };
                let state = instance.event_state.as_mut().unwrap();
                if let Some(next_due) = next_due {
                    let subscription = state.subscriptions.get_mut(&subscription_id).unwrap();
                    subscription.next_due = Some(next_due);
                    subscription.remaining = remaining;
                } else {
                    state.subscriptions.remove(&subscription_id);
                }
            }
            let activation = Uuid::new_v4();
            let generation = instance.event_state.as_ref().unwrap().generation;
            let baseline = instance.variables.clone();
            let mut effective = baseline.clone();
            effective.extend(variables.clone());
            let mut token = Token::with_variables(&handler.start_node, effective);
            token.event_context = Some(Box::new(EventContext {
                generation,
                activation: Some(activation),
                started_node: None,
                baseline,
            }));
            instance
                .event_state
                .as_mut()
                .unwrap()
                .activations
                .insert(activation, handler.id.clone());
            instance.tokens.insert(token.id, token.clone());
            instance.active_tokens.push(ActiveToken {
                token: token.clone(),
                fork_id: Some(handler.id.clone()),
                branch_index: 0,
                completed: false,
            });
            instance.state = InstanceState::ParallelExecution {
                active_token_count: instance
                    .active_tokens
                    .iter()
                    .filter(|t| !t.completed)
                    .count(),
            };
            instance.push_audit_log(format!(
                "Event subprocess '{}' activated ({activation}, interrupting={})",
                handler.id, handler.is_interrupting
            ));
            if handler.is_interrupting {
                queue_lifecycle(
                    &mut instance,
                    activation,
                    &handler.id,
                    HistoryEventType::EventSubprocessScopeInterrupted,
                    "containing process scope interrupted",
                );
            }
            queue_lifecycle(
                &mut instance,
                activation,
                &handler.id,
                HistoryEventType::EventSubprocessActivated,
                "activated",
            );
            (token, children, handler.is_interrupting)
        };
        self.persist_instance(instance_id).await;
        self.flush_event_lifecycle(instance_id).await;
        if interrupted {
            self.cancel_event_work(instance_id).await;
            for child in children {
                self.abort_event_child(child).await;
            }
        }
        Ok(Some(token))
    }

    pub(crate) async fn fire_event_subscription(
        &self,
        instance_id: Uuid,
        id: Uuid,
        variables: &HashMap<String, Value>,
    ) -> EngineResult<()> {
        if let Some(token) = self
            .prepare_event_activation(instance_id, id, variables)
            .await?
        {
            Box::pin(self.run_instance_batch(instance_id, token)).await?;
        }
        Ok(())
    }

    /// Run after loading definitions, instances and pending work. Remove cancelled records
    /// and resume an activation persisted before its first waiting task was written.
    pub async fn reconcile_event_subprocesses(&self) -> EngineResult<()> {
        for (id, instance) in self.instances.all().await {
            self.flush_event_lifecycle(id).await;
            let snapshot = instance.read().await.clone();
            let Some(state) = &snapshot.event_state else {
                continue;
            };
            if matches!(
                snapshot.state,
                InstanceState::Completed | InstanceState::CompletedWithError { .. }
            ) {
                self.cleanup_finished_event_scope(id).await;
                continue;
            }
            let known_tokens: HashSet<_> = snapshot
                .tokens
                .keys()
                .copied()
                .chain(snapshot.outstanding_calls.values().map(|c| c.token.id))
                .chain(
                    snapshot
                        .join_barriers
                        .values()
                        .flat_map(|j| j.arrived_tokens.iter().map(|t| t.id)),
                )
                .collect();
            let users: Vec<_> = self
                .pending_user_tasks
                .iter()
                .filter(|t| t.instance_id == id && !known_tokens.contains(&t.token_id))
                .map(|t| t.task_id)
                .collect();
            let services: Vec<_> = self
                .pending_service_tasks
                .iter()
                .filter(|t| t.instance_id == id && !known_tokens.contains(&t.token_id))
                .map(|t| t.id)
                .collect();
            let timers: Vec<_> = self
                .pending_timers
                .iter()
                .filter(|t| {
                    t.instance_id == id
                        && !known_tokens.contains(&t.token_id)
                        && !state.subscriptions.contains_key(&t.token_id)
                })
                .map(|t| t.id)
                .collect();
            let catches: Vec<_> = self
                .pending_message_catches
                .iter()
                .filter(|t| {
                    t.instance_id == id
                        && !known_tokens.contains(&t.token_id)
                        && !state.subscriptions.contains_key(&t.token_id)
                })
                .map(|t| t.id)
                .collect();
            for task in users {
                self.pending_user_tasks.remove(&task);
                self.remove_persisted_user_task(task).await;
            }
            for task in services {
                self.remove_pending_service_task(&task);
                self.remove_persisted_service_task(task).await;
            }
            for task in timers {
                self.remove_pending_timer(&task);
                self.remove_persisted_timer(task).await;
            }
            for task in catches {
                self.pending_message_catches.remove(&task);
                self.remove_persisted_message_catch(task).await;
            }
            // The snapshot is written before exposing jobs. Recreate any jobs
            // missing after a crash between these separate persistence writes.
            let definition = self
                .definitions
                .get(&snapshot.definition_key)
                .ok_or(EngineError::NoSuchDefinition(snapshot.definition_key))?;
            for (subscription_id, subscription) in &state.subscriptions {
                let handler = definition
                    .event_subprocesses
                    .iter()
                    .find(|handler| handler.id == subscription.subprocess_id)
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition("Missing persisted event handler".into())
                    })?;
                match &handler.trigger {
                    Trigger::Timer(timer) if !self.pending_timers.contains_key(subscription_id) => {
                        if let Some(expires_at) = subscription.next_due {
                            self.insert_pending_timer(PendingTimer {
                                id: *subscription_id,
                                instance_id: id,
                                node_id: handler.start_node.clone(),
                                token_id: *subscription_id,
                                expires_at,
                                timer_def: Some(timer.clone()),
                                remaining_repetitions: subscription.remaining,
                            });
                            self.persist_timer(*subscription_id).await;
                        }
                    }
                    Trigger::Message(name) | Trigger::Signal(name)
                        if !self.pending_message_catches.contains_key(subscription_id) =>
                    {
                        self.pending_message_catches.insert(
                            *subscription_id,
                            PendingMessageCatch {
                                id: *subscription_id,
                                instance_id: id,
                                node_id: handler.start_node.clone(),
                                token_id: *subscription_id,
                                message_name: name.clone(),
                                kind: if matches!(handler.trigger, Trigger::Signal(_)) {
                                    CatchKind::Signal
                                } else {
                                    CatchKind::Message
                                },
                            },
                        );
                        self.persist_message_catch(*subscription_id).await;
                    }
                    _ => {}
                }
            }
            if matches!(snapshot.state, InstanceState::Suspended { .. }) {
                continue;
            }
            for token in snapshot.tokens.values() {
                let parked = self
                    .pending_user_tasks
                    .iter()
                    .any(|t| t.instance_id == id && t.token_id == token.id)
                    || self
                        .pending_service_tasks
                        .iter()
                        .any(|t| t.instance_id == id && t.token_id == token.id)
                    || self.pending_timers.iter().any(|t| {
                        t.instance_id == id
                            && t.token_id == token.id
                            && t.node_id == token.current_node
                    })
                    || self
                        .pending_message_catches
                        .iter()
                        .any(|t| t.instance_id == id && t.token_id == token.id)
                    || snapshot
                        .outstanding_calls
                        .values()
                        .any(|c| c.token.id == token.id)
                    || snapshot
                        .join_barriers
                        .values()
                        .any(|j| j.arrived_tokens.iter().any(|t| t.id == token.id));
                if !parked {
                    self.run_instance_batch(id, token.clone()).await?;
                }
            }
            // A crash may leave a finished body awaiting its container-end
            // listener, with no runnable token left to drive completion.
            if !matches!(
                instance.read().await.state,
                InstanceState::Completed | InstanceState::CompletedWithError { .. }
            ) {
                match self.all_tokens_completed(id).await {
                    Ok(true) => {
                        let mut current = instance.write().await;
                        if !matches!(
                            current.state,
                            InstanceState::Suspended { .. }
                                | InstanceState::Completed
                                | InstanceState::CompletedWithError { .. }
                        ) && current.tokens.is_empty()
                            && current.outstanding_calls.is_empty()
                            && current.event_state.as_ref().is_some_and(|state| {
                                state.main_completed && state.activations.is_empty()
                            })
                        {
                            current.state = InstanceState::Completed;
                            current.completed_at = Some(Utc::now());
                            current.push_audit_log("Recovered completed process scope".into());
                            drop(current);
                            self.record_history_event(
                                id,
                                HistoryEventType::InstanceCompleted,
                                "Recovered completed process scope",
                                ActorType::Engine,
                                None,
                                None,
                            )
                            .await;
                        }
                    }
                    Ok(false) => {}
                    Err(error) => {
                        {
                            let mut current = instance.write().await;
                            current.state = InstanceState::CompletedWithError {
                                error_code: "EVENT_SUBPROCESS_EXECUTION_ERROR".into(),
                            };
                            current.completed_at = Some(Utc::now());
                            current.push_audit_log(error.to_string());
                        }
                        self.cleanup_finished_event_scope(id).await;
                        self.emit_instance_changed(id).await;
                        return Err(error);
                    }
                }
                self.finish_instance_batch(id).await?;
            }
        }
        Ok(())
    }

    pub(crate) async fn prepare_event_code(
        &self,
        instance_id: Uuid,
        code: &str,
        escalation: bool,
        source: &Token,
    ) -> EngineResult<Option<Token>> {
        let instance = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let (key, subscriptions, excluded) = {
            let instance = instance.read().await;
            if !token_valid(&instance, source) {
                return Ok(None);
            }
            let Some(state) = &instance.event_state else {
                return Ok(None);
            };
            let excluded = source
                .event_context
                .as_ref()
                .and_then(|c| c.activation)
                .and_then(|id| state.activations.get(&id))
                .cloned();
            (
                instance.definition_key,
                state.subscriptions.clone(),
                excluded,
            )
        };
        let def = self
            .definitions
            .get(&key)
            .ok_or(EngineError::NoSuchDefinition(key))?;
        let mut candidates = Vec::new();
        for (id, subscription) in subscriptions {
            let Some(handler) = def
                .event_subprocesses
                .iter()
                .find(|h| h.id == subscription.subprocess_id && Some(&h.id) != excluded.as_ref())
            else {
                continue;
            };
            let expected = match (&handler.trigger, escalation) {
                (Trigger::Error(code), false) | (Trigger::Escalation(code), true) => code,
                _ => continue,
            };
            if expected.as_deref().is_none_or(|expected| expected == code) {
                candidates.push((expected.is_none(), handler.id.clone(), id));
            }
        }
        candidates.sort();
        let mut payload = source.process_variables();
        if let Some(context) = &source.event_context {
            payload.retain(|key, value| context.baseline.get(key) != Some(value));
        }
        match candidates.first() {
            Some((_, _, id)) => {
                self.prepare_event_activation(instance_id, *id, &payload)
                    .await
            }
            None => Ok(None),
        }
    }

    pub(crate) async fn cleanup_finished_event_scope(&self, instance_id: Uuid) {
        let Some(instance) = self.instances.get(&instance_id).await else {
            return;
        };
        let children = {
            let mut instance = instance.write().await;
            if instance.event_state.is_none()
                || !matches!(
                    instance.state,
                    InstanceState::Completed | InstanceState::CompletedWithError { .. }
                )
            {
                return;
            }
            let children = instance
                .outstanding_calls
                .keys()
                .copied()
                .collect::<Vec<_>>();
            queue_cancelled_activations(&mut instance, "containing process scope finished");
            instance.outstanding_calls.clear();
            instance.tokens.clear();
            instance.active_tokens.clear();
            instance.join_barriers.clear();
            instance.multi_instance_state.clear();
            let state = instance.event_state.as_mut().unwrap();
            state.subscriptions.clear();
            state.activations.clear();
            state.started_activations.clear();
            children
        };
        self.cancel_event_work(instance_id).await;
        for child in children {
            self.abort_event_child(child).await;
        }
        self.persist_instance(instance_id).await;
        self.flush_event_lifecycle(instance_id).await;
    }

    pub(crate) async fn terminate_event_scope(
        &self,
        instance_id: Uuid,
        source: &Token,
    ) -> EngineResult<()> {
        let instance = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        {
            let mut instance = instance.write().await;
            if !token_valid(&instance, source) {
                return Ok(());
            }
            instance.state = InstanceState::Completed;
            instance.completed_at = Some(Utc::now());
            if let Some(state) = &mut instance.event_state {
                state.generation += 1;
            }
            instance.push_audit_log("Process scope terminated".into());
        }
        self.cleanup_finished_event_scope(instance_id).await;
        Ok(())
    }

    pub(crate) async fn cancel_event_work(&self, instance_id: Uuid) {
        let users: Vec<_> = self
            .pending_user_tasks
            .iter()
            .filter(|t| t.instance_id == instance_id)
            .map(|t| t.task_id)
            .collect();
        let services: Vec<_> = self
            .pending_service_tasks
            .iter()
            .filter(|t| t.instance_id == instance_id)
            .map(|t| t.id)
            .collect();
        let timers: Vec<_> = self
            .pending_timers
            .iter()
            .filter(|t| t.instance_id == instance_id)
            .map(|t| t.id)
            .collect();
        let catches: Vec<_> = self
            .pending_message_catches
            .iter()
            .filter(|t| t.instance_id == instance_id)
            .map(|t| t.id)
            .collect();
        for id in users {
            self.pending_user_tasks.remove(&id);
            self.remove_persisted_user_task(id).await;
        }
        for id in services {
            self.remove_pending_service_task(&id);
            self.remove_persisted_service_task(id).await;
        }
        for id in timers {
            self.remove_pending_timer(&id);
            self.remove_persisted_timer(id).await;
        }
        for id in catches {
            self.pending_message_catches.remove(&id);
            self.remove_persisted_message_catch(id).await;
        }
    }

    async fn abort_event_child(&self, id: Uuid) {
        let children = if let Some(instance) = self.instances.get(&id).await {
            let mut instance = instance.write().await;
            let children = instance
                .outstanding_calls
                .keys()
                .copied()
                .collect::<Vec<_>>();
            queue_cancelled_activations(&mut instance, "parent scope interrupted");
            instance.outstanding_calls.clear();
            instance.tokens.clear();
            instance.active_tokens.clear();
            instance.join_barriers.clear();
            instance.multi_instance_state.clear();
            if let Some(state) = &mut instance.event_state {
                state.generation += 1;
                state.subscriptions.clear();
                state.activations.clear();
                state.started_activations.clear();
            }
            instance.state = InstanceState::CompletedWithError {
                error_code: "SCOPE_INTERRUPTED".into(),
            };
            instance.completed_at = Some(Utc::now());
            children
        } else {
            Vec::new()
        };
        self.cancel_event_work(id).await;
        self.persist_instance(id).await;
        self.flush_event_lifecycle(id).await;
        for child in children {
            Box::pin(self.abort_event_child(child)).await;
        }
    }

    /// Completion ignores subscriptions, but includes active handler calls and parked joins.
    pub(crate) async fn refresh_event_completion(
        &self,
        instance: &mut ProcessInstance,
    ) -> EngineResult<()> {
        let mut active = HashSet::new();
        let mut main_active = false;
        for token in instance
            .tokens
            .values()
            .chain(instance.outstanding_calls.values().map(|call| &call.token))
            .chain(
                instance
                    .join_barriers
                    .values()
                    .flat_map(|join| &join.arrived_tokens),
            )
            .chain(
                instance
                    .active_tokens
                    .iter()
                    .filter(|t| !t.completed)
                    .map(|t| &t.token),
            )
        {
            match token
                .event_context
                .as_ref()
                .and_then(|context| context.activation)
            {
                Some(id) => {
                    active.insert(id);
                }
                None => main_active = true,
            }
        }
        let completed = if let Some(state) = &mut instance.event_state {
            state.main_completed = !main_active;
            state
                .activations
                .iter()
                .filter(|(id, _)| !active.contains(id))
                .map(|(id, handler)| (*id, handler.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if completed.is_empty() {
            return Ok(());
        }
        let definition = self
            .definitions
            .get(&instance.definition_key)
            .ok_or(EngineError::NoSuchDefinition(instance.definition_key))?;
        let mut completed = completed;
        completed.sort();
        for (activation, handler) in completed {
            let mut token = Token::with_variables(&handler, instance.variables.clone());
            token.event_context = Some(Box::new(EventContext {
                generation: instance.event_state.as_ref().unwrap().generation,
                activation: Some(activation),
                baseline: instance.variables.clone(),
                started_node: None,
            }));
            self.run_event_container_listener(
                instance,
                &mut token,
                &definition,
                &handler,
                ListenerEvent::End,
            )
            .await?;
            let state = instance.event_state.as_mut().unwrap();
            state.activations.remove(&activation);
            state.started_activations.remove(&activation);
            queue_lifecycle(
                instance,
                activation,
                &handler,
                HistoryEventType::EventSubprocessCompleted,
                "completed",
            );
        }
        Ok(())
    }
}
