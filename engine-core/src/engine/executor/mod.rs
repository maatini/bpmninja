use std::collections::VecDeque;
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{BpmnElement, ListenerEvent, Token};
use crate::domain::{EngineError, EngineResult};
use crate::engine::WorkflowEngine;
use crate::runtime::*;
use crate::scripting;

mod completion;
pub(crate) mod helpers;
mod next_action;
mod parallel;

pub(crate) use helpers::{
    find_any_boundary_escalation_event, find_boundary_error_event, find_compensation_handler,
    resolve_next_target,
};

impl WorkflowEngine {
    /// Non-recursive batched execution loop.
    pub(crate) async fn run_instance_batch(
        &self,
        instance_id: Uuid,
        initial_token: Token,
    ) -> EngineResult<()> {
        // History of this batch is buffered and written in one call — also when
        // the batch fails, so no recorded step is lost.
        let mut history = Vec::new();
        let driven = self
            .drive_instance_batch(instance_id, initial_token, &mut history)
            .await;
        self.flush_history(instance_id, history).await;
        if let Err(error) = driven {
            if matches!(error, EngineError::DataMappingError(_)) {
                if let Some(instance) = self.instances.get(&instance_id).await {
                    let mut instance = instance.write().await;
                    instance.state = InstanceState::CompletedWithError {
                        error_code: "DATA_MAPPING_ERROR".into(),
                    };
                    instance.completed_at = Some(chrono::Utc::now());
                    instance.push_audit_log(error.to_string());
                }
                self.persist_instance(instance_id).await;
            }
            self.emit_instance_changed(instance_id).await;
            self.deliver_queued_signals().await;
            return Err(error);
        }

        let finished = self.finish_instance_batch(instance_id).await;
        self.deliver_queued_signals().await;
        finished
    }

    /// Runs tokens until the queue is empty; collects history entries.
    async fn drive_instance_batch(
        &self,
        instance_id: Uuid,
        initial_token: Token,
        history: &mut Vec<crate::history::HistoryEntry>,
    ) -> EngineResult<()> {
        let mut queue = VecDeque::new();
        queue.push_back(initial_token);
        let mut step_count: u32 = 0;

        while let Some(mut token) = queue.pop_front() {
            step_count += 1;

            // Cooperative scheduling: yield to Tokio every N steps
            if step_count.is_multiple_of(crate::runtime::YIELD_EVERY_N_STEPS) {
                tokio::task::yield_now().await;
            }

            // Hard abort: prevent infinite BPMN loops
            if step_count > crate::runtime::MAX_EXECUTION_STEPS {
                tracing::error!(
                    "Instance {} exceeded {} execution steps — aborting (possible infinite loop)",
                    instance_id,
                    crate::runtime::MAX_EXECUTION_STEPS
                );
                if let Some(inst_arc) = self.instances.get(&instance_id).await {
                    let mut inst = inst_arc.write().await;
                    inst.state = InstanceState::CompletedWithError {
                        error_code: "EXECUTION_LIMIT_EXCEEDED".to_string(),
                    };
                    inst.completed_at = Some(chrono::Utc::now());
                    inst.push_audit_log(format!(
                        "ABORTED: Exceeded {} execution steps",
                        crate::runtime::MAX_EXECUTION_STEPS
                    ));
                }
                self.persist_instance(instance_id).await;
                return Err(EngineError::ExecutionLimitExceeded(format!(
                    "Instance {} exceeded execution step limit ({})",
                    instance_id,
                    crate::runtime::MAX_EXECUTION_STEPS
                )));
            }

            // The pre-step snapshot only feeds history, which needs persistence.
            let old_snapshot = match &self.persistence {
                Some(_) => match self.instances.get(&instance_id).await {
                    Some(lk) => Some(crate::history::DiffSnapshot::from_instance(
                        &*lk.read().await,
                    )),
                    None => None,
                },
                None => None,
            };
            let current_gateway_id = token.current_node.clone();

            let action = self.execute_step(instance_id, &mut token).await?;

            let (event_type, description) = match &action {
                NextAction::Continue(_) => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Token advanced".to_string(),
                ),
                NextAction::ContinueMultiple(_) => (
                    crate::history::HistoryEventType::TokenForked,
                    "Token forked at gateway".to_string(),
                ),
                NextAction::WaitForJoin { .. } => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Token arrived at join".to_string(),
                ),
                NextAction::WaitForUser(_) => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Waiting for user task".to_string(),
                ),
                NextAction::WaitForServiceTask(_) => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Waiting for service task".to_string(),
                ),
                NextAction::WaitForTimer(_) => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Waiting for timer".to_string(),
                ),
                NextAction::WaitForMessage(_) => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Waiting for message".to_string(),
                ),
                NextAction::WaitForCallActivity { .. } => (
                    crate::history::HistoryEventType::CallActivityStarted,
                    "Spawned call activity".to_string(),
                ),
                NextAction::Complete => (
                    crate::history::HistoryEventType::BranchCompleted,
                    "Execution path completed".to_string(),
                ),
                NextAction::ErrorEnd { error_code } => (
                    crate::history::HistoryEventType::BranchCompleted,
                    format!("Execution path completed with error '{}'", error_code),
                ),
                NextAction::EscalationEnd { escalation_code } => (
                    crate::history::HistoryEventType::EscalationThrown,
                    format!("Escalation '{}' thrown at end event", escalation_code),
                ),
                NextAction::SpawnAndContinue { .. } => (
                    crate::history::HistoryEventType::TokenForked,
                    "Escalation handler spawned (non-interrupting)".to_string(),
                ),
                NextAction::Terminate => (
                    crate::history::HistoryEventType::BranchCompleted,
                    "Process terminated".to_string(),
                ),
                NextAction::WaitForEventGroup(_) => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Waiting for multiple alternative events".to_string(),
                ),
                NextAction::MultiInstanceFork { .. } => (
                    crate::history::HistoryEventType::TokenForked,
                    "Spawned Multi-Instance parallel tokens".to_string(),
                ),
                NextAction::MultiInstanceNext { .. } => (
                    crate::history::HistoryEventType::TokenAdvanced,
                    "Advanced to next Multi-Instance sequential iteration".to_string(),
                ),
            };

            if let Some(entry) = self
                .build_history_entry_from_snapshot(
                    instance_id,
                    event_type,
                    &description,
                    crate::history::ActorType::Engine,
                    None,
                    old_snapshot.as_ref(),
                )
                .await
            {
                history.push(entry);
            }

            let token_id = token.id;
            self.handle_next_action(
                action,
                instance_id,
                token_id,
                &current_gateway_id,
                &mut queue,
            )
            .await?;
        } // end while

        Ok(())
    }

    /// Persists the instance after a batch and handles completion.
    async fn finish_instance_batch(&self, instance_id: Uuid) -> EngineResult<()> {
        // Flush persistence for the entire batch
        self.persist_instance(instance_id).await;

        // After batch finishes for this instance, if it completed, check parent
        let mut completed = false;
        let mut error_code_to_propagate = None;
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let inst = inst_arc.read().await;
            if matches!(inst.state, InstanceState::Completed) {
                completed = true;
            } else if let InstanceState::CompletedWithError { error_code } = &inst.state {
                completed = true;
                error_code_to_propagate = Some(error_code.clone());
            }
        }
        if completed {
            metrics::counter!("bpmn_instance_completed_total").increment(1);
            metrics::gauge!("bpmn_active_instances").decrement(1.0);
            self.resume_parent_if_needed(instance_id, error_code_to_propagate)
                .await?;
            // Archive to history store and remove from active map
            self.archive_completed_instance(instance_id).await;
        }

        self.emit_instance_changed(instance_id).await;

        Ok(())
    }

    pub(crate) async fn execute_step(
        &self,
        instance_id: Uuid,
        token: &mut Token,
    ) -> EngineResult<NextAction> {
        let def_key = {
            let instance_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let instance = instance_arc.read().await;
            instance.definition_key
        };

        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?;

        let current_id = token.current_node.clone();
        let element = def
            .get_node(&current_id)
            .ok_or_else(|| EngineError::NoSuchNode(current_id.clone()))?
            .clone();

        let def_clone = Arc::clone(&def);

        // X4/X5: a marked token that left its loop activity finishes an iteration.
        if let Some(marker) = token.loop_instance.clone()
            && marker.node_id != current_id
            && let Some(action) = self
                .leave_loop_instance(instance_id, token, &def_clone, marker)
                .await?
        {
            return Ok(action);
        }
        // X4/X5: entering a loop activity starts its iterations.
        if let Some(characteristics) = def_clone.loops.get(&current_id)
            && token
                .loop_instance
                .as_ref()
                .is_none_or(|marker| marker.node_id != current_id)
        {
            return self
                .enter_loop_activity(
                    instance_id,
                    token.clone(),
                    &def_clone,
                    &current_id,
                    characteristics,
                )
                .await;
        }

        crate::engine::data_mappings::enter(token, &def_clone, &current_id)?;

        let mut start_audits = Vec::new();
        scripting::run_node_scripts(
            &self.scripts,
            instance_id,
            token,
            &def_clone,
            &current_id,
            ListenerEvent::Start,
            &mut start_audits,
        )
        .await?;
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let mut inst = inst_arc.write().await;
            inst.audit_log.append(&mut start_audits);
            // Only sync variables if a script listener potentially modified them
            if def_clone.listeners.contains_key(&current_id) {
                inst.variables = token.process_variables();
            }
        }

        match &element {
            BpmnElement::StartEvent
            | BpmnElement::TimerStartEvent(_)
            | BpmnElement::MessageStartEvent { .. }
            | BpmnElement::SignalStartEvent { .. } => {
                self.handle_start_event(instance_id, token, &def_clone, &current_id)
                    .await
            }
            BpmnElement::EndEvent => {
                self.handle_end_event(instance_id, token, &def_clone, &current_id)
                    .await
            }
            BpmnElement::TerminateEndEvent => {
                self.handle_terminate_end_event(instance_id, token, &def_clone, &current_id)
                    .await
            }
            BpmnElement::ErrorEndEvent { error_code } => {
                self.handle_error_end_event(instance_id, token, &def_clone, &current_id, error_code)
                    .await
            }
            BpmnElement::UserTask(assignee) => {
                self.handle_user_task(instance_id, token, &def_clone, &current_id, assignee)
                    .await
            }
            BpmnElement::ScriptTask { script, .. } => {
                self.handle_script_task(instance_id, token, &def_clone, &current_id, script)
                    .await
            }
            BpmnElement::SendTask { message_name, .. } => {
                self.handle_send_task(instance_id, token, &def_clone, &current_id, message_name)
                    .await
            }
            BpmnElement::ServiceTask { topic, .. } => {
                self.handle_service_task(instance_id, token, &def_clone, &current_id, topic)
                    .await
            }
            BpmnElement::ParallelGateway => {
                self.handle_parallel_gateway(instance_id, token, &def_clone, &current_id)
                    .await
            }
            BpmnElement::ExclusiveGateway { default } => {
                self.handle_exclusive_gateway(instance_id, token, &def_clone, &current_id, default)
                    .await
            }
            BpmnElement::InclusiveGateway => {
                self.handle_inclusive_gateway(instance_id, token, &def_clone, &current_id)
                    .await
            }
            BpmnElement::ComplexGateway { default, .. } => {
                self.handle_complex_gateway(instance_id, token, &def_clone, &current_id, default)
                    .await
            }
            BpmnElement::EventBasedGateway => {
                self.handle_event_based_gateway(instance_id, token, &def_clone, &current_id)
                    .await
            }
            BpmnElement::TimerCatchEvent(timer_def) => {
                self.handle_timer_catch_event(instance_id, token, &current_id, timer_def)
                    .await
            }
            BpmnElement::MessageCatchEvent { message_name } => {
                self.handle_message_catch_event(
                    instance_id,
                    token,
                    &current_id,
                    message_name,
                    crate::runtime::CatchKind::Message,
                )
                .await
            }
            BpmnElement::SignalCatchEvent { signal_name } => {
                self.handle_message_catch_event(
                    instance_id,
                    token,
                    &current_id,
                    signal_name,
                    crate::runtime::CatchKind::Signal,
                )
                .await
            }
            BpmnElement::SignalThrowEvent { signal_name } => {
                self.handle_signal_throw_event(
                    instance_id,
                    token,
                    &def_clone,
                    &current_id,
                    signal_name,
                )
                .await
            }
            BpmnElement::SignalEndEvent { signal_name } => {
                let action = self
                    .handle_end_event(instance_id, token, &def_clone, &current_id)
                    .await?;
                self.signal_outbox.push(signal_name.clone());
                Ok(action)
            }
            BpmnElement::CallActivity { called_element } => {
                self.handle_call_activity(
                    instance_id,
                    token,
                    &def_clone,
                    &current_id,
                    called_element,
                )
                .await
            }
            BpmnElement::EmbeddedSubProcess { start_node_id } => {
                self.handle_embedded_sub_process(token, start_node_id).await
            }
            BpmnElement::SubProcessEndEvent { sub_process_id } => {
                self.handle_sub_process_end_event(token, &def_clone, sub_process_id)
                    .await
            }
            BpmnElement::EscalationEndEvent { escalation_code } => {
                self.handle_escalation_end_event(
                    instance_id,
                    token,
                    &def_clone,
                    &current_id,
                    escalation_code,
                )
                .await
            }
            BpmnElement::EscalationThrowEvent { escalation_code } => {
                self.handle_escalation_throw_event(
                    instance_id,
                    token,
                    &def_clone,
                    &current_id,
                    escalation_code,
                )
                .await
            }
            BpmnElement::CompensationThrowEvent { activity_ref } => {
                self.handle_compensation_throw_event(
                    instance_id,
                    token,
                    &def_clone,
                    &current_id,
                    activity_ref,
                    false,
                )
                .await
            }
            BpmnElement::CompensationEndEvent { activity_ref } => {
                self.handle_compensation_throw_event(
                    instance_id,
                    token,
                    &def_clone,
                    &current_id,
                    activity_ref,
                    true,
                )
                .await
            }
            BpmnElement::BoundaryTimerEvent { .. }
            | BpmnElement::BoundaryMessageEvent { .. }
            | BpmnElement::BoundarySignalEvent { .. }
            | BpmnElement::BoundaryErrorEvent { .. }
            | BpmnElement::BoundaryEscalationEvent { .. } => {
                self.handle_boundary_event(instance_id, token, &def_clone, &current_id)
                    .await
            }
            BpmnElement::BoundaryCompensationEvent { .. } => {
                // Compensation boundary events are not directly executed —
                // they register handlers when the attached activity completes.
                Ok(NextAction::Complete)
            }
        }
    }
}
