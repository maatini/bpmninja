use super::WorkflowEngine;
use crate::InstanceState;
use crate::domain::{EngineError, EngineResult, IncidentKind};

/// Delay before a timer that failed processing becomes due again.
///
/// A failed timer stays pending. Without this delay the event-driven scheduler
/// would see it as actionable and retry it in a tight loop.
const FAILED_TIMER_RETRY_DELAY_MS: i64 = 2_000;

impl WorkflowEngine {
    /// Processes all currently due timers.
    ///
    /// A failing timer is isolated: the error is counted and logged, the timer is
    /// kept and re-scheduled with a short delay, and the remaining due timers of
    /// the same round are still processed. The returned count is the number of
    /// due timers found at the start of the round.
    pub async fn process_timers(&self) -> EngineResult<usize> {
        let now = self.now();
        let mut expired = Vec::new();

        for tid in self.timer_due_index.due_ids(now) {
            let Some(timer) = self.pending_timers.get(&tid) else {
                continue;
            };
            if timer.expires_at > now {
                continue;
            }
            // Skip timers for suspended instances
            if let Some(inst_arc) = self.instances.get(&timer.instance_id).await {
                let inst = inst_arc.read().await;
                if matches!(inst.state, InstanceState::Suspended { .. }) {
                    continue;
                }
            }
            expired.push(tid);
        }

        let count = expired.len();

        for tid in expired {
            // Take the timer out of the pending map while it is being processed,
            // so a concurrent round cannot pick it up twice.
            let Some(timer) = self.remove_pending_timer(&tid).map(|(_, v)| v) else {
                tracing::debug!(
                    timer_id = %tid,
                    "Timer disappeared before processing (event-based sibling already cancelled)"
                );
                continue;
            };

            match self.process_expired_timer(&timer).await {
                Ok(()) => {
                    metrics::counter!("bpmn_timer_fired_total").increment(1);
                    // Persisted removal happens only after successful processing.
                    self.remove_persisted_timer(tid).await;
                }
                Err(e) => {
                    self.timer_processing_errors
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    metrics::counter!("bpmn_timer_errors_total").increment(1);
                    tracing::warn!(
                        timer_id = %tid,
                        instance_id = %timer.instance_id,
                        node_id = %timer.node_id,
                        error = %e,
                        "Timer processing failed; re-scheduling in {}ms",
                        FAILED_TIMER_RETRY_DELAY_MS
                    );
                    // Dedupliziert: wiederholte Fehler derselben Node erhöhen nur
                    // `occurrences`, statt die Incident-Liste zu fluten.
                    let recorded = self
                        .record_incident(
                            timer.instance_id,
                            IncidentKind::Timer,
                            Some(timer.node_id.clone()),
                            None,
                            format!("Timer processing failed: {e}"),
                            None,
                        )
                        .await;
                    if matches!(recorded, Some((_, true))) {
                        self.persist_instance(timer.instance_id).await;
                    }
                    self.reschedule_failed_timer(timer).await;
                }
            }
        }

        Ok(count)
    }

    /// Executes a single expired timer. Returns an error if the timer could not
    /// be processed; the caller then keeps the timer for a later attempt.
    async fn process_expired_timer(
        &self,
        timer: &crate::runtime::PendingTimer,
    ) -> EngineResult<()> {
        // Event-Based Gateway support: If this timer triggered, clear any sibling wait states
        self.clear_wait_states_for_token(timer.instance_id, &timer.token_id)
            .await;

        let old_state = if let Some(lk) = self.instances.get(&timer.instance_id).await {
            Some(lk.read().await.clone())
        } else {
            None
        };
        let def_key = {
            let inst_arc = self
                .instances
                .get(&timer.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(timer.instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.state = InstanceState::Running;
            inst.audit_log
                .push(format!("⏱ Timer '{}' expired, resuming", timer.node_id));
            inst.definition_key
        };

        self.record_history_event(
            timer.instance_id,
            crate::history::HistoryEventType::TokenAdvanced,
            "Timer expired",
            crate::history::ActorType::Timer,
            None,
            old_state.as_ref(),
        )
        .await;

        if timer.token_id.is_nil() {
            // This is a Scope Event Listener (Event Sub-Process) trigger
            let child_bpmn_id = timer.node_id.clone();
            let child_def_key = {
                let (k, _) = self
                    .definitions
                    .find_latest_by_bpmn_id(&child_bpmn_id)
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition(format!(
                            "Event Subprocess '{child_bpmn_id}' not found"
                        ))
                    })?;
                k
            };

            // Get the instance variables to pass down
            let instance_vars = {
                let inst_arc = self
                    .instances
                    .get(&timer.instance_id)
                    .await
                    .ok_or(EngineError::NoSuchInstance(timer.instance_id))?;
                let inst = inst_arc.read().await;
                inst.variables.clone()
            };

            // Spawn the call activity loosely (it will track parent_instance_id automatically)
            let _child_id = self
                .spawn_call_activity(
                    child_def_key,
                    timer.instance_id,
                    child_bpmn_id.clone(),
                    instance_vars,
                )
                .await?;

            return Ok(());
        }

        // Retrieve token from central store
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;

        let mut is_non_interrupting = false;
        if let Some(crate::domain::BpmnElement::BoundaryTimerEvent {
            cancel_activity: false,
            ..
        }) = def.nodes.get(&timer.node_id)
        {
            is_non_interrupting = true;
        }

        let mut token = {
            let inst_arc = self
                .instances
                .get(&timer.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(timer.instance_id))?;
            let mut inst = inst_arc.write().await;

            if is_non_interrupting {
                // Clone the token, leave the original waiting
                let mut original = inst
                    .tokens
                    .get(&timer.token_id)
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition(format!(
                            "Token {} not found in instance",
                            timer.token_id
                        ))
                    })?
                    .clone();

                original.id = uuid::Uuid::new_v4();

                // Add cloned token as parallel token
                inst.tokens.insert(original.id, original.clone());

                let active = crate::runtime::ActiveToken {
                    token: original.clone(),
                    completed: false,
                    fork_id: Some(original.current_node.clone()),
                    branch_index: inst.active_tokens.len(),
                };
                inst.active_tokens.push(active);

                if !matches!(
                    inst.state,
                    crate::runtime::InstanceState::ParallelExecution { .. }
                ) {
                    inst.state = crate::runtime::InstanceState::ParallelExecution {
                        active_token_count: inst.tokens.len(),
                    };
                }

                original
            } else {
                // Interrupting timer: Remove original token

                // Clean up orphaned pending tasks that matched this token
                inst.tokens.remove(&timer.token_id).ok_or_else(|| {
                    EngineError::InvalidDefinition(format!(
                        "Token {} not found in instance",
                        timer.token_id
                    ))
                })?
            }
        };

        // Clean up orphaned tasks outside the instance lock
        if !is_non_interrupting {
            let node_to_cancel = &token.current_node;

            // Remove User Tasks
            let mut to_remove_ut = Vec::new();
            for r in self.pending_user_tasks.iter() {
                if r.value().instance_id == timer.instance_id
                    && r.value().node_id == *node_to_cancel
                    && r.value().token_id == timer.token_id
                {
                    to_remove_ut.push(*r.key());
                }
            }
            for id in to_remove_ut {
                self.pending_user_tasks.remove(&id);
                self.remove_persisted_user_task(id).await;
            }

            // Remove Service Tasks
            let mut to_remove_st = Vec::new();
            for r in self.pending_service_tasks.iter() {
                if r.value().instance_id == timer.instance_id
                    && r.value().node_id == *node_to_cancel
                    && r.value().token_id == timer.token_id
                {
                    to_remove_st.push(*r.key());
                }
            }
            for id in to_remove_st {
                self.pending_service_tasks.remove(&id);
                self.remove_persisted_service_task(id).await;
            }
        }

        let next =
            crate::engine::executor::resolve_next_target(&def, &timer.node_id, &token.variables)?;
        token.current_node = next.clone();

        {
            let inst_arc = self
                .instances
                .get(&timer.instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(timer.instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.current_node = next;
        }

        self.run_instance_batch(timer.instance_id, token).await?;

        // Re-schedule recurring timers.
        // ISO R3 = exactly 3 fires: after a fire, Some(1) is the last one (do not insert).
        if let Some(ref def) = timer.timer_def
            && def.is_recurring()
        {
            // None = infinite; Some(1) = this fire was the last; Some(0) = already done.
            let should_repeat = !matches!(timer.remaining_repetitions, Some(0 | 1));
            if should_repeat {
                let now = self.now();
                if let Some(next_expiry) = def.next_expiry(now) {
                    let new_remaining = timer.remaining_repetitions.map(|r| r.saturating_sub(1));
                    let new_pending = crate::runtime::PendingTimer {
                        id: uuid::Uuid::new_v4(),
                        instance_id: timer.instance_id,
                        node_id: timer.node_id.clone(),
                        expires_at: next_expiry,
                        token_id: timer.token_id,
                        timer_def: Some(def.clone()),
                        remaining_repetitions: new_remaining,
                    };
                    let new_id = new_pending.id;
                    self.insert_pending_timer(new_pending);
                    self.persist_timer(new_id).await;
                }
            }
        }

        Ok(())
    }

    /// Puts a timer that failed processing back into the pending map and
    /// persists it. The new expiry is pushed into the future to avoid a hot loop.
    async fn reschedule_failed_timer(&self, mut timer: crate::runtime::PendingTimer) {
        let timer_id = timer.id;
        timer.expires_at = self.now() + chrono::Duration::milliseconds(FAILED_TIMER_RETRY_DELAY_MS);
        self.insert_pending_timer(timer);
        self.persist_timer(timer_id).await;
    }
}
