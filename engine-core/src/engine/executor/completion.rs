use uuid::Uuid;

use crate::domain::{EngineError, EngineResult, ProcessDefinition, Token};
use crate::engine::WorkflowEngine;
use crate::scripting;

impl WorkflowEngine {
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
        let inst = inst_arc.read().await;
        if !inst.tokens.is_empty() {
            return Ok(false);
        }
        if inst.active_tokens.is_empty() {
            // Linear flow
            return Ok(true);
        }
        Ok(inst.active_tokens.iter().all(|t| t.completed))
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
        let crate::ProcessInstance {
            audit_log,
            variables,
            ..
        } = &mut *inst;
        scripting::run_end_scripts(
            &self.script_config,
            instance_id,
            token,
            def,
            node_id,
            audit_log,
            variables,
        )
        .await
    }
}
