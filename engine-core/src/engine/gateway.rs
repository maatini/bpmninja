use crate::condition::evaluate_condition;
use crate::error::{EngineError, EngineResult};
use crate::model::{ProcessDefinition, Token};
use crate::engine::types::NextAction;

pub(crate) fn execute_parallel_gateway(
    def: &ProcessDefinition,
    current_id: &str,
    token: &mut Token,
) -> EngineResult<NextAction> {
    let outgoing = def.next_nodes(current_id);
    let incoming_count = def.incoming_flow_count(current_id);

    if incoming_count >= 2 && !token.is_merged {
        // --- JOIN LOGIC ---
        return Ok(NextAction::WaitForJoin {
            gateway_id: current_id.to_string(),
            token: token.clone(),
        });
    }
    token.is_merged = false;

    // --- SPLIT LOGIC ---
    let forked: Vec<Token> = outgoing
        .iter()
        .map(|sf| Token::with_variables(&sf.target, token.variables.clone()))
        .collect();

    if forked.len() == 1 {
        Ok(NextAction::Continue(
            forked
                .into_iter()
                .next()
                .expect("BUG: forked vec verified as len()==1 but is empty"),
        ))
    } else {
        Ok(NextAction::ContinueMultiple(forked))
    }
}

pub(crate) fn execute_exclusive_gateway(
    def: &ProcessDefinition,
    current_id: &str,
    token: &mut Token,
    default: &Option<String>,
) -> EngineResult<NextAction> {
    let outgoing = def.next_nodes(current_id);
    let mut chosen_target: Option<String> = None;

    // Evaluate conditions in order; first match wins
    for sf in outgoing {
        if let Some(ref cond) = sf.condition {
            if evaluate_condition(cond, &token.variables) {
                chosen_target = Some(sf.target.clone());
                break;
            }
        }
    }

    // Fallback to default flow if no condition matched
    if chosen_target.is_none() {
        if let Some(default_target) = default {
            chosen_target = Some(default_target.clone());
        }
    }

    let target = chosen_target.ok_or_else(|| EngineError::NoMatchingCondition(current_id.to_string()))?;

    token.current_node = target.clone();
    Ok(NextAction::Continue(token.clone()))
}

pub(crate) fn execute_inclusive_gateway(
    def: &ProcessDefinition,
    current_id: &str,
    token: &mut Token,
) -> EngineResult<NextAction> {
    let outgoing = def.next_nodes(current_id);
    let incoming_count = def.incoming_flow_count(current_id);

    if incoming_count >= 2 && !token.is_merged {
        // --- JOIN LOGIC ---
        return Ok(NextAction::WaitForJoin {
            gateway_id: current_id.to_string(),
            token: token.clone(),
        });
    }
    token.is_merged = false;

    // --- SPLIT LOGIC ---
    let mut matched_targets: Vec<String> = Vec::new();

    // Evaluate all conditions; every match is taken
    for sf in outgoing {
        if let Some(ref cond) = sf.condition {
            if evaluate_condition(cond, &token.variables) {
                matched_targets.push(sf.target.clone());
            }
        } else {
            // Unconditional flows are always taken
            matched_targets.push(sf.target.clone());
        }
    }

    if matched_targets.is_empty() {
        return Err(EngineError::NoMatchingCondition(current_id.to_string()));
    }

    // Fork tokens — each gets a copy of the current variables
    let forked: Vec<Token> = matched_targets
        .into_iter()
        .map(|target| Token::with_variables(&target, token.variables.clone()))
        .collect();

    if forked.len() == 1 {
        let single = forked
            .into_iter()
            .next()
            .expect("BUG: forked vec verified as len()==1 but is empty");
        Ok(NextAction::Continue(single))
    } else {
        Ok(NextAction::ContinueMultiple(forked))
    }
}
