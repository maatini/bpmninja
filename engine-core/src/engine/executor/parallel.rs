use uuid::Uuid;

use crate::condition::evaluate_condition;
use crate::domain::{BpmnElement, EngineError, EngineResult, Token};
use crate::engine::WorkflowEngine;
use crate::runtime::{ActiveToken, JoinBarrier};

use super::helpers::find_downstream_join;

impl WorkflowEngine {
    pub(crate) async fn register_join_barrier_if_needed(
        &self,
        instance_id: Uuid,
        split_gateway_id: &str,
        branch_count: usize,
    ) -> EngineResult<()> {
        let def_key_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let def_key = def_key_arc.read().await.definition_key;
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?
            .clone();

        if let Some(join_id) = find_downstream_join(&def, split_gateway_id) {
            let inst_arc = self
                .instances
                .get(&instance_id)
                .await
                .ok_or(EngineError::NoSuchInstance(instance_id))?;
            let mut inst = inst_arc.write().await;
            inst.join_barriers.insert(
                join_id.clone(),
                JoinBarrier {
                    gateway_node_id: join_id.clone(),
                    expected_count: branch_count,
                    arrived_tokens: Vec::new(),
                },
            );
            tracing::debug!(
                "Registered JoinBarrier for join '{join_id}' (expected: {branch_count})"
            );
        }
        Ok(())
    }

    pub(crate) async fn register_active_token(
        &self,
        instance_id: Uuid,
        fork_id: &str,
        branch_index: usize,
        token: &Token,
    ) -> EngineResult<()> {
        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;
        inst.active_tokens.push(ActiveToken {
            token: token.clone(),
            fork_id: Some(fork_id.to_string()),
            branch_index,
            completed: false,
        });
        Ok(())
    }

    pub(crate) async fn arrive_at_join(
        &self,
        instance_id: Uuid,
        gateway_id: &str,
        token: Token,
    ) -> EngineResult<Option<Token>> {
        let def_key_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let def_key = def_key_arc.read().await.definition_key;
        let def = self
            .definitions
            .get(&def_key)
            .ok_or(EngineError::NoSuchDefinition(def_key))?
            .clone();

        let structural_expected = def.incoming_flow_count(gateway_id);
        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;

        let expected_count;
        let current_arrived;

        {
            let barrier = inst
                .join_barriers
                .entry(gateway_id.to_string())
                .or_insert_with(|| JoinBarrier {
                    gateway_node_id: gateway_id.to_string(),
                    expected_count: structural_expected,
                    arrived_tokens: Vec::new(),
                });
            expected_count = barrier.expected_count;
            barrier.arrived_tokens.push(token.clone());
            current_arrived = barrier.arrived_tokens.len();
        }

        inst.audit_log.push(format!(
            "➔ Token arrived at join '{}' ({}/{})",
            gateway_id, current_arrived, expected_count
        ));

        let mut condition_met = false;
        if let Some(BpmnElement::ComplexGateway {
            join_condition: Some(cond),
            ..
        }) = def.get_node(gateway_id)
        {
            let mut temp_vars = std::collections::HashMap::new();
            if let Some(barrier) = inst.join_barriers.get(gateway_id) {
                for t in &barrier.arrived_tokens {
                    temp_vars.extend(t.variables.clone());
                }
            }
            if evaluate_condition(cond, &temp_vars) {
                condition_met = true;
                inst.audit_log.push(format!(
                    "⟡ Complex gateway join condition met early at '{gateway_id}'"
                ));
            }
        }

        if current_arrived >= expected_count || condition_met {
            let all_tokens = inst
                .join_barriers
                .remove(gateway_id)
                .ok_or_else(|| {
                    EngineError::InvalidDefinition(format!(
                        "Join barrier for gateway '{}' not found in instance {}",
                        gateway_id, instance_id
                    ))
                })?
                .arrived_tokens;

            for t in &all_tokens {
                if let Some(active) = inst.active_tokens.iter_mut().find(|at| at.token.id == t.id) {
                    active.completed = true;
                }
            }

            let mut merged_vars = std::collections::HashMap::new();
            for t in &all_tokens {
                merged_vars.extend(t.variables.clone());
            }

            let mut merged_token = Token::with_variables(gateway_id, merged_vars);
            merged_token.is_merged = true;
            inst.audit_log.push(format!(
                "🔗 Join '{}' completed. Tokens merged.",
                gateway_id
            ));

            drop(inst);

            self.record_history_event(
                instance_id,
                crate::history::HistoryEventType::TokenJoined,
                &format!("Joined {} tokens at '{}'", current_arrived, gateway_id),
                crate::history::ActorType::Engine,
                None,
                None,
            )
            .await;

            Ok(Some(merged_token))
        } else {
            Ok(None)
        }
    }
}
