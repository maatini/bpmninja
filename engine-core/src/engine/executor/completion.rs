use uuid::Uuid;

use crate::domain::{EngineError, EngineResult, ProcessDefinition, Token};
use crate::engine::WorkflowEngine;
use crate::scripting;

impl WorkflowEngine {
    pub(crate) fn scope_execution_complete(instance: &crate::runtime::ProcessInstance) -> bool {
        instance
            .event_state
            .as_ref()
            .is_none_or(|state| state.main_completed && state.activations.is_empty())
            && instance.outstanding_calls.is_empty()
            && instance.tokens.is_empty()
            && instance.active_tokens.iter().all(|token| token.completed)
    }
    pub(crate) async fn complete_branch_token(
        &self,
        instance_id: Uuid,
        token_id: Uuid,
    ) -> EngineResult<()> {
        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;
        if let Some(active) = inst
            .active_tokens
            .iter_mut()
            .find(|at| at.token.id == token_id)
        {
            active.completed = true;
        }
        Ok(())
    }

    pub(crate) async fn all_tokens_completed(&self, instance_id: Uuid) -> EngineResult<bool> {
        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;
        self.refresh_event_completion(&mut inst).await?;
        Ok(Self::scope_execution_complete(&inst))
    }

    /// Runs End scripts, commits variables to instance state.
    pub(crate) async fn run_end_scripts(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def: &ProcessDefinition,
        node_id: &str,
    ) -> EngineResult<()> {
        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;
        if !crate::engine::event_subprocess::token_valid(&inst, token) {
            return Err(EngineError::AlreadyCompleted);
        }
        let crate::ProcessInstance {
            audit_log,
            variables,
            ..
        } = &mut *inst;
        scripting::run_end_scripts(
            &self.scripts,
            instance_id,
            token,
            def,
            node_id,
            audit_log,
            variables,
        )
        .await?;
        Self::evaluate_conditionals(&mut inst, def)?;
        Ok(())
    }
}
