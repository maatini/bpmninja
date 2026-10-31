//! Event-Subprozesse auf Prozessebene.
//!
//! Der Prozess ist der umgebende Scope: Beim Start werden Subscriptions angelegt,
//! ein unterbrechender Start bricht die Scope-Tokens ab, ein nicht unterbrechender
//! läuft parallel, und beim Verlassen des Scopes werden Timer und Catches gelöscht.

use super::WorkflowEngine;
use crate::domain::{
    EngineError, EngineResult, EventSubProcess, EventSubProcessTrigger, EventSubscription, Token,
};
use crate::runtime::{
    ActiveToken, InstanceState, PendingMessageCatch, PendingSignalCatch, PendingTimer,
    ProcessInstance,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub(crate) enum EventConsumption {
    Timer(PendingTimer),
    Message(PendingMessageCatch),
    Signal(PendingSignalCatch),
    Code,
}

pub(crate) enum ScopeActivation {
    Interrupting(Token),
    Parallel(Token),
}

impl WorkflowEngine {
    /// Legt die Subscriptions an, sobald die Instanz den Prozess-Scope betritt.
    pub(crate) async fn open_event_scope(&self, instance_id: Uuid) -> EngineResult<()> {
        let def_key = {
            let Some(instance) = self.instances.get(&instance_id).await else {
                return Err(EngineError::NoSuchInstance(instance_id));
            };
            let instance = instance.read().await;
            if instance.event_scope || !instance.event_subscriptions.is_empty() {
                return Ok(());
            }
            instance.definition_key
        };
        let Some(def) = self.definitions.get(&def_key) else {
            return Err(EngineError::NoSuchDefinition(def_key));
        };
        if def.event_subprocesses.is_empty() {
            return Ok(());
        }

        let now = self.now();
        let mut subscriptions = Vec::new();
        let mut timers = Vec::new();
        let mut messages = Vec::new();
        let mut signals = Vec::new();
        for handler in &def.event_subprocesses {
            let id = Uuid::new_v4();
            match &handler.trigger {
                EventSubProcessTrigger::Timer(timer) => {
                    let Some(expires_at) = timer.next_expiry(now) else {
                        tracing::info!(
                            "Event subprocess '{}' has no future timer expiry; not subscribing",
                            handler.id
                        );
                        continue;
                    };
                    let remaining = timer.initial_remaining_repetitions();
                    subscriptions.push(EventSubscription {
                        id,
                        subprocess_id: handler.id.clone(),
                        next_due: Some(expires_at),
                        remaining,
                    });
                    timers.push(PendingTimer {
                        id,
                        instance_id,
                        node_id: handler.start_node.clone(),
                        expires_at,
                        token_id: id,
                        timer_def: Some(timer.clone()),
                        remaining_repetitions: remaining,
                    });
                }
                EventSubProcessTrigger::Message(message_name) => {
                    subscriptions.push(plain_subscription(id, &handler.id));
                    messages.push(PendingMessageCatch {
                        id,
                        instance_id,
                        node_id: handler.start_node.clone(),
                        message_name: message_name.clone(),
                        token_id: id,
                    });
                }
                EventSubProcessTrigger::Signal(signal_name) => {
                    subscriptions.push(plain_subscription(id, &handler.id));
                    signals.push(PendingSignalCatch {
                        id,
                        instance_id,
                        node_id: handler.start_node.clone(),
                        signal_name: signal_name.clone(),
                        token_id: id,
                    });
                }
                EventSubProcessTrigger::Error(_) | EventSubProcessTrigger::Escalation(_) => {
                    subscriptions.push(plain_subscription(id, &handler.id));
                }
            }
        }

        let opened = subscriptions.len();
        {
            let Some(instance) = self.instances.get(&instance_id).await else {
                return Err(EngineError::NoSuchInstance(instance_id));
            };
            let mut instance = instance.write().await;
            instance.event_scope = true;
            instance.event_subscriptions = subscriptions;
            instance.push_audit_log(format!(
                "Event subprocess scope opened ({opened} subscription(s))"
            ));
        }
        for timer in timers {
            let id = timer.id;
            self.insert_pending_timer(timer);
            self.persist_timer(id).await;
        }
        for catch in messages {
            let id = catch.id;
            self.pending_message_catches.insert(id, catch);
            self.persist_message_catch(id).await;
        }
        for catch in signals {
            let id = catch.id;
            self.pending_signal_catches.insert(id, catch);
            self.persist_signal_catch(id).await;
        }
        self.persist_instance(instance_id).await;
        Ok(())
    }

    pub(crate) async fn event_handler_for_token(
        &self,
        instance_id: Uuid,
        token_id: Uuid,
    ) -> Option<String> {
        let instance = self.instances.get(&instance_id).await?;
        instance
            .read()
            .await
            .event_handler_tokens
            .get(&token_id)
            .cloned()
    }

    pub(crate) async fn is_event_subscription(&self, instance_id: Uuid, id: Uuid) -> bool {
        let Some(instance) = self.instances.get(&instance_id).await else {
            return false;
        };
        instance
            .read()
            .await
            .event_subscriptions
            .iter()
            .any(|subscription| subscription.id == id)
    }

    /// Startet den Handler einer Message-, Signal- oder Timer-Subscription.
    ///
    /// Der Aufrufer hat den Wait bereits aus der Map genommen. Liegt er danach
    /// wieder in der Map, bleibt der persistierte Datensatz; sonst wird er gelöscht.
    pub(crate) async fn activate_event_subscription(
        &self,
        instance_id: Uuid,
        consumption: EventConsumption,
        variables: &HashMap<String, Value>,
    ) -> EngineResult<Option<ScopeActivation>> {
        let subscription_id = consumption.id();
        let instance = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let (def_key, subscription, suspended, terminal) = {
            let instance = instance.read().await;
            let suspended = matches!(instance.state, InstanceState::Suspended { .. });
            let terminal = is_terminal(&instance.state);
            let subscription = instance
                .event_subscriptions
                .iter()
                .find(|subscription| subscription.id == subscription_id)
                .cloned();
            (instance.definition_key, subscription, suspended, terminal)
        };
        if suspended {
            self.reinsert_consumption(consumption);
            return Ok(None);
        }
        if terminal {
            return Ok(None);
        }
        let Some(subscription) = subscription else {
            return Ok(None);
        };
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;
        let Some(handler) = def.event_subprocess_by_id(&subscription.subprocess_id) else {
            return Err(EngineError::InvalidDefinition(format!(
                "Event subscription refers to missing subprocess '{}'",
                subscription.subprocess_id
            )));
        };
        let handler = handler.clone();
        self.activate_handler(
            instance_id,
            &handler,
            &subscription,
            variables,
            &consumption,
        )
        .await
    }

    pub(crate) async fn try_activate_event_code(
        &self,
        instance_id: Uuid,
        code: &str,
        escalation: bool,
        variables: &HashMap<String, Value>,
        exclude_subprocess: Option<&str>,
    ) -> EngineResult<Option<ScopeActivation>> {
        let instance = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let (def_key, subscriptions, runnable) = {
            let instance = instance.read().await;
            let runnable = !matches!(
                instance.state,
                InstanceState::Suspended { .. }
                    | InstanceState::Completed
                    | InstanceState::CompletedWithError { .. }
            );
            (
                instance.definition_key,
                instance.event_subscriptions.clone(),
                runnable,
            )
        };
        if !runnable {
            return Ok(None);
        }
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;
        let mut candidates = Vec::new();
        for subscription in subscriptions {
            if exclude_subprocess == Some(subscription.subprocess_id.as_str()) {
                continue;
            }
            let Some(handler) = def.event_subprocess_by_id(&subscription.subprocess_id) else {
                continue;
            };
            let expected = match (&handler.trigger, escalation) {
                (EventSubProcessTrigger::Error(expected), false)
                | (EventSubProcessTrigger::Escalation(expected), true) => expected,
                _ => continue,
            };
            if expected.as_deref().is_none_or(|expected| expected == code) {
                candidates.push((
                    expected.is_none(),
                    handler.id.clone(),
                    subscription,
                    handler.clone(),
                ));
            }
        }
        candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        let Some((_, _, subscription, handler)) = candidates.into_iter().next() else {
            return Ok(None);
        };
        self.activate_handler(
            instance_id,
            &handler,
            &subscription,
            variables,
            &EventConsumption::Code,
        )
        .await
    }

    async fn activate_handler(
        &self,
        instance_id: Uuid,
        handler: &EventSubProcess,
        subscription: &EventSubscription,
        variables: &HashMap<String, Value>,
        consumption: &EventConsumption,
    ) -> EngineResult<Option<ScopeActivation>> {
        let now = self.now();
        let reschedule = reschedule_timer(handler, subscription, now);

        let (token, children, interrupting) = {
            let Some(instance) = self.instances.get(&instance_id).await else {
                return Err(EngineError::NoSuchInstance(instance_id));
            };
            let mut instance = instance.write().await;
            if matches!(
                instance.state,
                InstanceState::Suspended { .. }
                    | InstanceState::Completed
                    | InstanceState::CompletedWithError { .. }
            ) {
                return Ok(None);
            }
            if !instance
                .event_subscriptions
                .iter()
                .any(|current| current.id == subscription.id)
            {
                return Ok(None);
            }

            let children: Vec<Uuid> = if handler.is_interrupting {
                let children = instance.outstanding_calls.keys().copied().collect();
                instance.tokens.clear();
                instance.active_tokens.clear();
                instance.join_barriers.clear();
                instance.multi_instance_state.clear();
                instance.outstanding_calls.clear();
                instance.event_subscriptions.clear();
                instance.event_handler_tokens.clear();
                children
            } else {
                if let Some(next) = &reschedule {
                    if let Some(current) = instance
                        .event_subscriptions
                        .iter_mut()
                        .find(|current| current.id == subscription.id)
                    {
                        current.next_due = Some(next.expires_at);
                        current.remaining = next.remaining;
                    }
                } else if matches!(handler.trigger, EventSubProcessTrigger::Timer(_)) {
                    instance
                        .event_subscriptions
                        .retain(|current| current.id != subscription.id);
                }
                Vec::new()
            };

            let mut merged = instance.variables.clone();
            merged.extend(variables.clone());
            instance.variables = merged.clone();
            let token = Token::with_variables(&handler.start_node, merged);
            instance
                .event_handler_tokens
                .insert(token.id, handler.id.clone());
            instance.tokens.insert(token.id, token.clone());
            let branch_index = instance.active_tokens.len();
            instance.active_tokens.push(ActiveToken {
                token: token.clone(),
                fork_id: Some(handler.id.clone()),
                branch_index,
                completed: false,
            });
            instance.current_node = handler.start_node.clone();
            if handler.is_interrupting {
                instance.state = InstanceState::Running;
            } else if !matches!(instance.state, InstanceState::ParallelExecution { .. })
                || instance.tokens.len() > 1
            {
                instance.state = InstanceState::ParallelExecution {
                    active_token_count: instance.tokens.len(),
                };
            }
            instance.push_audit_log(format!(
                "Event subprocess '{}' activated (interrupting={})",
                handler.id, handler.is_interrupting
            ));
            (token, children, handler.is_interrupting)
        };

        if interrupting {
            self.cancel_scope_waits(instance_id).await;
            for child in children {
                Box::pin(self.abort_interrupted_instance(child)).await;
            }
        } else {
            self.retain_consumption(consumption, subscription, reschedule.as_ref());
        }
        self.persist_instance(instance_id).await;
        if interrupting {
            Ok(Some(ScopeActivation::Interrupting(token)))
        } else {
            Ok(Some(ScopeActivation::Parallel(token)))
        }
    }

    fn reinsert_consumption(&self, consumption: EventConsumption) {
        match consumption {
            EventConsumption::Timer(timer) => self.insert_pending_timer(timer),
            EventConsumption::Message(catch) => {
                self.pending_message_catches.insert(catch.id, catch);
            }
            EventConsumption::Signal(catch) => {
                self.pending_signal_catches.insert(catch.id, catch);
            }
            EventConsumption::Code => {}
        }
    }

    fn retain_consumption(
        &self,
        consumption: &EventConsumption,
        subscription: &EventSubscription,
        reschedule: Option<&TimerReschedule>,
    ) {
        match consumption {
            EventConsumption::Message(catch) => {
                self.pending_message_catches.insert(catch.id, catch.clone());
            }
            EventConsumption::Signal(catch) => {
                self.pending_signal_catches.insert(catch.id, catch.clone());
            }
            EventConsumption::Timer(timer) => {
                if let Some(next) = reschedule {
                    self.insert_pending_timer(PendingTimer {
                        id: timer.id,
                        instance_id: timer.instance_id,
                        node_id: timer.node_id.clone(),
                        expires_at: next.expires_at,
                        token_id: subscription.id,
                        timer_def: Some(next.timer.clone()),
                        remaining_repetitions: next.remaining,
                    });
                }
            }
            EventConsumption::Code => {}
        }
    }

    /// Entfernt Subscriptions und ihre Waits, wenn der Prozess-Scope endet.
    pub(crate) async fn close_event_scope(&self, instance_id: Uuid) {
        let ids = {
            let Some(instance) = self.instances.get(&instance_id).await else {
                return;
            };
            let mut instance = instance.write().await;
            if instance.event_subscriptions.is_empty() {
                return;
            }
            let ids: Vec<Uuid> = instance
                .event_subscriptions
                .iter()
                .map(|subscription| subscription.id)
                .collect();
            instance.event_scope = false;
            instance.event_subscriptions.clear();
            instance.event_handler_tokens.clear();
            instance.push_audit_log("Event subprocess scope closed".into());
            ids
        };
        for id in ids {
            self.remove_pending_timer(&id);
            self.remove_persisted_timer(id).await;
            self.pending_message_catches.remove(&id);
            self.remove_persisted_message_catch(id).await;
            self.pending_signal_catches.remove(&id);
            self.remove_persisted_signal_catch(id).await;
        }
    }

    pub(crate) async fn cancel_scope_waits(&self, instance_id: Uuid) {
        let users: Vec<Uuid> = self
            .pending_user_tasks
            .iter()
            .filter(|task| task.instance_id == instance_id)
            .map(|task| task.task_id)
            .collect();
        let services: Vec<Uuid> = self
            .pending_service_tasks
            .iter()
            .filter(|task| task.instance_id == instance_id)
            .map(|task| task.id)
            .collect();
        let timers: Vec<Uuid> = self
            .pending_timers
            .iter()
            .filter(|timer| timer.instance_id == instance_id)
            .map(|timer| timer.id)
            .collect();
        let messages: Vec<Uuid> = self
            .pending_message_catches
            .iter()
            .filter(|catch| catch.instance_id == instance_id)
            .map(|catch| catch.id)
            .collect();
        let signals: Vec<Uuid> = self
            .pending_signal_catches
            .iter()
            .filter(|catch| catch.instance_id == instance_id)
            .map(|catch| catch.id)
            .collect();
        let conditionals: Vec<Uuid> = self
            .pending_conditional_catches
            .iter()
            .filter(|catch| catch.instance_id == instance_id)
            .map(|catch| catch.id)
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
        for id in messages {
            self.pending_message_catches.remove(&id);
            self.remove_persisted_message_catch(id).await;
        }
        for id in signals {
            self.pending_signal_catches.remove(&id);
            self.remove_persisted_signal_catch(id).await;
        }
        for id in conditionals {
            self.pending_conditional_catches.remove(&id);
            self.remove_persisted_conditional_catch(id).await;
        }
    }

    async fn abort_interrupted_instance(&self, instance_id: Uuid) {
        let children = {
            let Some(instance) = self.instances.get(&instance_id).await else {
                return;
            };
            let mut instance = instance.write().await;
            if is_terminal(&instance.state) {
                return;
            }
            let children: Vec<Uuid> = instance.outstanding_calls.keys().copied().collect();
            instance.outstanding_calls.clear();
            instance.tokens.clear();
            instance.active_tokens.clear();
            instance.join_barriers.clear();
            instance.multi_instance_state.clear();
            instance.event_subscriptions.clear();
            instance.event_handler_tokens.clear();
            instance.state = InstanceState::CompletedWithError {
                error_code: "SCOPE_INTERRUPTED".into(),
            };
            instance.completed_at = Some(self.now());
            instance.push_audit_log("Scope interrupted by an interrupting event subprocess".into());
            children
        };
        self.cancel_scope_waits(instance_id).await;
        self.persist_instance(instance_id).await;
        metrics::gauge!("bpmn_active_instances").decrement(1.0);
        for child in children {
            Box::pin(self.abort_interrupted_instance(child)).await;
        }
    }

    /// Stellt Subscriptions nach einem Neustart wieder her und verwirft Waits
    /// abgebrochener Tokens. Nicht geparkte Handler-Tokens laufen weiter.
    pub async fn reconcile_event_subprocesses(&self) -> EngineResult<()> {
        let instances = self.instances.all().await;
        for (instance_id, instance) in instances {
            let snapshot = instance.read().await.clone();
            let scoped = snapshot.event_scope || !snapshot.event_subscriptions.is_empty();
            if is_terminal(&snapshot.state) {
                if scoped {
                    self.close_event_scope(instance_id).await;
                    self.drop_orphan_waits(instance_id, &snapshot).await;
                }
                continue;
            }
            if matches!(snapshot.state, InstanceState::Suspended { .. }) {
                continue;
            }
            self.restore_missing_subscriptions(instance_id, &snapshot)
                .await?;
            if !scoped {
                continue;
            }
            self.drop_orphan_waits(instance_id, &snapshot).await;
            let parked = self.parked_token_ids(instance_id, &snapshot);
            let pending: Vec<Token> = snapshot
                .tokens
                .values()
                .filter(|token| !parked.contains(&token.id))
                .cloned()
                .collect();
            for token in pending {
                let Some(current) = self.instances.get(&instance_id).await else {
                    continue;
                };
                let still_unparked = {
                    let current = current.read().await;
                    current.tokens.contains_key(&token.id)
                        && !self
                            .parked_token_ids(instance_id, &current)
                            .contains(&token.id)
                        && !is_terminal(&current.state)
                };
                if still_unparked {
                    self.run_instance_batch(instance_id, token).await?;
                }
            }
        }
        Ok(())
    }

    async fn restore_missing_subscriptions(
        &self,
        instance_id: Uuid,
        snapshot: &ProcessInstance,
    ) -> EngineResult<()> {
        if snapshot.event_subscriptions.is_empty() {
            return Ok(());
        }
        let def = self
            .definitions
            .get(&snapshot.definition_key)
            .ok_or(EngineError::NoSuchDefinition(snapshot.definition_key))?;
        for subscription in &snapshot.event_subscriptions {
            let Some(handler) = def.event_subprocess_by_id(&subscription.subprocess_id) else {
                return Err(EngineError::InvalidDefinition(format!(
                    "Missing persisted event subprocess '{}'",
                    subscription.subprocess_id
                )));
            };
            match &handler.trigger {
                EventSubProcessTrigger::Timer(timer) => {
                    if self.pending_timers.contains_key(&subscription.id) {
                        continue;
                    }
                    let Some(expires_at) = subscription.next_due else {
                        continue;
                    };
                    self.insert_pending_timer(PendingTimer {
                        id: subscription.id,
                        instance_id,
                        node_id: handler.start_node.clone(),
                        expires_at,
                        token_id: subscription.id,
                        timer_def: Some(timer.clone()),
                        remaining_repetitions: subscription.remaining,
                    });
                    self.persist_timer(subscription.id).await;
                }
                EventSubProcessTrigger::Message(message_name) => {
                    if self.pending_message_catches.contains_key(&subscription.id) {
                        continue;
                    }
                    self.pending_message_catches.insert(
                        subscription.id,
                        PendingMessageCatch {
                            id: subscription.id,
                            instance_id,
                            node_id: handler.start_node.clone(),
                            message_name: message_name.clone(),
                            token_id: subscription.id,
                        },
                    );
                    self.persist_message_catch(subscription.id).await;
                }
                EventSubProcessTrigger::Signal(signal_name) => {
                    if self.pending_signal_catches.contains_key(&subscription.id) {
                        continue;
                    }
                    self.pending_signal_catches.insert(
                        subscription.id,
                        PendingSignalCatch {
                            id: subscription.id,
                            instance_id,
                            node_id: handler.start_node.clone(),
                            signal_name: signal_name.clone(),
                            token_id: subscription.id,
                        },
                    );
                    self.persist_signal_catch(subscription.id).await;
                }
                EventSubProcessTrigger::Error(_) | EventSubProcessTrigger::Escalation(_) => {}
            }
        }
        Ok(())
    }

    async fn drop_orphan_waits(&self, instance_id: Uuid, snapshot: &ProcessInstance) {
        let known = known_token_ids(snapshot);
        let subscriptions: HashSet<Uuid> = snapshot
            .event_subscriptions
            .iter()
            .map(|subscription| subscription.id)
            .collect();
        let users: Vec<Uuid> = self
            .pending_user_tasks
            .iter()
            .filter(|task| task.instance_id == instance_id && !known.contains(&task.token_id))
            .map(|task| task.task_id)
            .collect();
        let services: Vec<Uuid> = self
            .pending_service_tasks
            .iter()
            .filter(|task| task.instance_id == instance_id && !known.contains(&task.token_id))
            .map(|task| task.id)
            .collect();
        let timers: Vec<Uuid> = self
            .pending_timers
            .iter()
            .filter(|timer| {
                timer.instance_id == instance_id
                    && !known.contains(&timer.token_id)
                    && !subscriptions.contains(&timer.id)
            })
            .map(|timer| timer.id)
            .collect();
        let messages: Vec<Uuid> = self
            .pending_message_catches
            .iter()
            .filter(|catch| {
                catch.instance_id == instance_id
                    && !known.contains(&catch.token_id)
                    && !subscriptions.contains(&catch.id)
            })
            .map(|catch| catch.id)
            .collect();
        let signals: Vec<Uuid> = self
            .pending_signal_catches
            .iter()
            .filter(|catch| {
                catch.instance_id == instance_id
                    && !known.contains(&catch.token_id)
                    && !subscriptions.contains(&catch.id)
            })
            .map(|catch| catch.id)
            .collect();
        let conditionals: Vec<Uuid> = self
            .pending_conditional_catches
            .iter()
            .filter(|catch| catch.instance_id == instance_id && !known.contains(&catch.token_id))
            .map(|catch| catch.id)
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
        for id in messages {
            self.pending_message_catches.remove(&id);
            self.remove_persisted_message_catch(id).await;
        }
        for id in signals {
            self.pending_signal_catches.remove(&id);
            self.remove_persisted_signal_catch(id).await;
        }
        for id in conditionals {
            self.pending_conditional_catches.remove(&id);
            self.remove_persisted_conditional_catch(id).await;
        }
    }

    fn parked_token_ids(&self, instance_id: Uuid, snapshot: &ProcessInstance) -> HashSet<Uuid> {
        let mut parked = HashSet::new();
        for task in self.pending_user_tasks.iter() {
            if task.instance_id == instance_id {
                parked.insert(task.token_id);
            }
        }
        for task in self.pending_service_tasks.iter() {
            if task.instance_id == instance_id {
                parked.insert(task.token_id);
            }
        }
        for timer in self.pending_timers.iter() {
            if timer.instance_id == instance_id {
                parked.insert(timer.token_id);
            }
        }
        for catch in self.pending_message_catches.iter() {
            if catch.instance_id == instance_id {
                parked.insert(catch.token_id);
            }
        }
        for catch in self.pending_signal_catches.iter() {
            if catch.instance_id == instance_id {
                parked.insert(catch.token_id);
            }
        }
        for catch in self.pending_conditional_catches.iter() {
            if catch.instance_id == instance_id {
                parked.insert(catch.token_id);
            }
        }
        for call in snapshot.outstanding_calls.values() {
            parked.insert(call.token.id);
        }
        for join in snapshot.join_barriers.values() {
            for token in &join.arrived_tokens {
                parked.insert(token.id);
            }
        }
        parked
    }
}

impl EventConsumption {
    fn id(&self) -> Uuid {
        match self {
            Self::Timer(timer) => timer.id,
            Self::Message(catch) => catch.id,
            Self::Signal(catch) => catch.id,
            Self::Code => Uuid::nil(),
        }
    }
}

struct TimerReschedule {
    expires_at: chrono::DateTime<chrono::Utc>,
    remaining: Option<u32>,
    timer: crate::domain::TimerDefinition,
}

fn reschedule_timer(
    handler: &EventSubProcess,
    subscription: &EventSubscription,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<TimerReschedule> {
    if handler.is_interrupting {
        return None;
    }
    let EventSubProcessTrigger::Timer(timer) = &handler.trigger else {
        return None;
    };
    let repeat = timer.is_recurring() && !matches!(subscription.remaining, Some(0 | 1));
    if !repeat {
        return None;
    }
    timer.next_expiry(now).map(|expires_at| TimerReschedule {
        expires_at,
        remaining: subscription.remaining.map(|left| left.saturating_sub(1)),
        timer: timer.clone(),
    })
}

fn plain_subscription(id: Uuid, subprocess_id: &str) -> EventSubscription {
    EventSubscription {
        id,
        subprocess_id: subprocess_id.to_string(),
        next_due: None,
        remaining: None,
    }
}

fn is_terminal(state: &InstanceState) -> bool {
    matches!(
        state,
        InstanceState::Completed | InstanceState::CompletedWithError { .. }
    )
}

fn known_token_ids(snapshot: &ProcessInstance) -> HashSet<Uuid> {
    let mut known: HashSet<Uuid> = snapshot.tokens.keys().copied().collect();
    for call in snapshot.outstanding_calls.values() {
        known.insert(call.token.id);
    }
    for join in snapshot.join_barriers.values() {
        for token in &join.arrived_tokens {
            known.insert(token.id);
        }
    }
    known
}
