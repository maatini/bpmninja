//! Signal events (X1): broadcast to every waiting signal catch and boundary in
//! all instances, and start every latest definition with a matching signal
//! start event.
//!
//! Signals thrown by a process are queued in the [`SignalOutbox`] and
//! delivered after the throwing batch has finished, so no instance lock is held
//! and no batch re-enters the same instance. A model whose signal start throws
//! the same signal again loops, as in other engines.

use crate::domain::{BpmnElement, EngineError, EngineResult, ProcessDefinition, Token};
use crate::engine::WorkflowEngine;
use crate::engine::executor::resolve_next_target;
use crate::runtime::{CatchKind, NextAction};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;

/// Signals thrown during execution, waiting for delivery.
#[derive(Default)]
pub(crate) struct SignalOutbox {
    queue: std::sync::Mutex<VecDeque<String>>,
    /// Set while a delivery loop runs; nested batches leave delivery to it.
    draining: AtomicBool,
}

impl SignalOutbox {
    pub(crate) fn push(&self, signal_name: String) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(signal_name);
    }

    fn pop(&self) -> Option<String> {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }

    fn is_empty(&self) -> bool {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }
}

impl WorkflowEngine {
    /// Broadcasts a signal. Returns the resumed and the newly started instances.
    pub async fn broadcast_signal(
        &self,
        signal_name: &str,
        variables: HashMap<String, Value>,
    ) -> EngineResult<Vec<Uuid>> {
        let label = format!("Signal '{signal_name}'");
        let catches: Vec<(Uuid, Uuid)> = self
            .pending_message_catches
            .iter()
            .filter(|c| c.kind == CatchKind::Signal && c.message_name == signal_name)
            .map(|c| (c.id, c.instance_id))
            .collect();

        let mut affected = Vec::new();
        for (catch_id, instance_id) in catches {
            // A sibling catch (event gateway, interrupting boundary) may have fired first.
            if !self.pending_message_catches.contains_key(&catch_id) {
                continue;
            }
            match self.resume_catch(catch_id, &variables, &label).await {
                Ok(()) => affected.push(instance_id),
                Err(e) => tracing::warn!("{label}: resuming instance {instance_id} failed: {e}"),
            }
        }

        let to_start: Vec<Uuid> = self
            .definitions
            .all()
            .into_iter()
            .filter(|(key, def)| {
                matches!(
                    def.start_event(),
                    Some((_, BpmnElement::SignalStartEvent { signal_name: name })) if name == signal_name
                ) && self
                    .definitions
                    .find_latest_by_bpmn_id(&def.id)
                    .is_some_and(|(latest, _)| latest == *key)
            })
            .map(|(key, _)| key)
            .collect();
        for def_key in to_start {
            match self
                .start_instance_with_variables(def_key, variables.clone())
                .await
            {
                Ok(id) => affected.push(id),
                Err(e) => tracing::warn!("{label}: starting definition {def_key} failed: {e}"),
            }
        }

        tracing::info!("{label} broadcast to {} instance(s)", affected.len());
        Ok(affected)
    }

    /// Delivers signals queued by throw/end events. Only the outermost caller
    /// loops; nested batches return immediately.
    pub(crate) async fn deliver_queued_signals(&self) {
        loop {
            if self.signal_outbox.draining.swap(true, Ordering::SeqCst) {
                return;
            }
            while let Some(signal_name) = self.signal_outbox.pop() {
                if let Err(e) = Box::pin(self.broadcast_signal(&signal_name, HashMap::new())).await
                {
                    tracing::warn!("Signal '{signal_name}' delivery failed: {e}");
                }
            }
            self.signal_outbox.draining.store(false, Ordering::SeqCst);
            // A signal queued between the last pop and the reset is picked up here.
            if self.signal_outbox.is_empty() {
                return;
            }
        }
    }

    pub(crate) async fn handle_signal_throw_event(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def_clone: &Arc<ProcessDefinition>,
        current_id: &str,
        signal_name: &str,
    ) -> EngineResult<NextAction> {
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
            "📡 Signal '{signal_name}' thrown at '{current_id}'"
        ));
        drop(inst);

        self.signal_outbox.push(signal_name.to_string());
        Ok(NextAction::Continue(token.clone()))
    }
}
