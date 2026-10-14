//! Loop and multi-instance activities (X4 / X5).
//!
//! A token executing one iteration carries a [`LoopInstance`] marker. The
//! executor calls [`WorkflowEngine::enter_loop_activity`] when a token reaches
//! a loop activity and [`WorkflowEngine::leave_loop_instance`] when a marked
//! token has left the activity (normal completion or boundary event). The
//! progress lives in `ProcessInstance.multi_instance_state` and is persisted
//! with the instance.
//!
//! `loopCounter`, `nrOfInstances`, `nrOfCompletedInstances`,
//! `nrOfActiveInstances`, the element and the output element variable are local to the iterations:
//! their previous values are restored when the activity completes.

use crate::domain::multi_instance::strip_expression;
use crate::domain::{
    BpmnElement, EngineError, EngineResult, LoopCharacteristics, LoopInstance, MultiInstanceDef,
    ProcessDefinition, Token,
};
use crate::engine::WorkflowEngine;
use crate::engine::executor::resolve_next_target;
use crate::runtime::{InstanceState, MultiInstanceProgress, NextAction};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

const LOOP_COUNTER: &str = "loopCounter";
const NR_OF_INSTANCES: &str = "nrOfInstances";
const NR_OF_COMPLETED: &str = "nrOfCompletedInstances";
const NR_OF_ACTIVE: &str = "nrOfActiveInstances";

fn local_variables(characteristics: &LoopCharacteristics) -> Vec<String> {
    match characteristics {
        LoopCharacteristics::MultiInstance(mi) => {
            let mut names: Vec<String> =
                [LOOP_COUNTER, NR_OF_INSTANCES, NR_OF_COMPLETED, NR_OF_ACTIVE]
                    .iter()
                    .map(|n| n.to_string())
                    .collect();
            names.extend(mi.element_variable.iter().cloned());
            names.extend(mi.output_element.iter().cloned());
            names
        }
        LoopCharacteristics::Standard(_) => vec![LOOP_COUNTER.to_string()],
    }
}

/// Collection elements or `0..cardinality` placeholders.
fn instance_elements(
    node_id: &str,
    mi: &MultiInstanceDef,
    variables: &HashMap<String, Value>,
) -> EngineResult<Vec<Value>> {
    if let Some(collection) = &mi.collection {
        let name = strip_expression(collection);
        return match variables.get(name) {
            Some(Value::Array(items)) => Ok(items.clone()),
            Some(Value::Null) | None => Err(EngineError::InvalidDefinition(format!(
                "Multi-instance '{node_id}': collection variable '{name}' is not set"
            ))),
            Some(_) => Err(EngineError::InvalidDefinition(format!(
                "Multi-instance '{node_id}': collection variable '{name}' is not an array"
            ))),
        };
    }
    let expr = mi
        .loop_cardinality
        .as_deref()
        .map(strip_expression)
        .unwrap_or_default();
    let count = expr
        .parse::<i64>()
        .ok()
        .or_else(|| variables.get(expr).and_then(Value::as_i64))
        .ok_or_else(|| {
            EngineError::InvalidDefinition(format!(
                "Multi-instance '{node_id}': loopCardinality '{expr}' is not an integer or integer variable"
            ))
        })?;
    let count = usize::try_from(count).map_err(|_| {
        EngineError::InvalidDefinition(format!(
            "Multi-instance '{node_id}': loopCardinality must not be negative"
        ))
    })?;
    Ok(vec![Value::Null; count])
}

fn condition_holds(
    def: &ProcessDefinition,
    expr: &Option<String>,
    vars: &HashMap<String, Value>,
) -> bool {
    expr.as_deref()
        .map(|e| def.evaluate_condition(strip_expression(e), vars))
        .unwrap_or(false)
}

/// Variables of one multi-instance iteration.
fn set_iteration_variables(
    vars: &mut HashMap<String, Value>,
    mi: &MultiInstanceDef,
    index: usize,
    progress: &MultiInstanceProgress,
    element: &Value,
) {
    vars.insert(LOOP_COUNTER.into(), index.into());
    vars.insert(NR_OF_INSTANCES.into(), progress.total.into());
    vars.insert(NR_OF_COMPLETED.into(), progress.completed.into());
    vars.insert(
        NR_OF_ACTIVE.into(),
        progress.active_tokens.len().max(1).into(),
    );
    if let Some(name) = &mi.element_variable {
        vars.insert(name.clone(), element.clone());
    }
}

enum Running {
    /// Same token runs the next iteration (sequential / standard loop).
    NextIteration,
    /// A parallel child finished; others are still running.
    ChildDone,
    /// The activity is complete.
    Done,
}

enum Outcome {
    NextIteration,
    ChildDone,
    Finished(Box<MultiInstanceProgress>),
}

/// Records one finished multi-instance iteration (no I/O; called under the instance lock).
fn count_multi_instance_iteration(
    def: &ProcessDefinition,
    mi: &MultiInstanceDef,
    token: &mut Token,
    index: usize,
    progress: &mut MultiInstanceProgress,
) -> Running {
    progress.completed += 1;
    progress.active_tokens.retain(|id| *id != token.id);
    if let Some(name) = &mi.output_element
        && let Some(slot) = progress.outputs.get_mut(index)
    {
        *slot = token.variables.get(name).cloned().unwrap_or(Value::Null);
    }
    if !mi.is_sequential {
        progress.merged_variables.extend(token.variables.clone());
    }

    let mut condition_vars = token.variables.clone();
    condition_vars.insert(NR_OF_COMPLETED.into(), progress.completed.into());
    condition_vars.insert(NR_OF_ACTIVE.into(), progress.active_tokens.len().into());
    condition_vars.insert(NR_OF_INSTANCES.into(), progress.total.into());
    if progress.completed >= progress.total
        || condition_holds(def, &mi.completion_condition, &condition_vars)
    {
        return Running::Done;
    }
    if !mi.is_sequential {
        return Running::ChildDone;
    }

    let next = index + 1;
    let element = progress.elements.get(next).cloned().unwrap_or(Value::Null);
    set_iteration_variables(&mut token.variables, mi, next, progress, &element);
    token.current_node = progress.node_id.clone();
    token.loop_instance = Some(LoopInstance {
        node_id: progress.node_id.clone(),
        index: next,
    });
    Running::NextIteration
}

fn restore_shadowed(vars: &mut HashMap<String, Value>, progress: &MultiInstanceProgress) {
    for (name, previous) in &progress.shadowed {
        match previous {
            Some(value) => vars.insert(name.clone(), value.clone()),
            None => vars.remove(name),
        };
    }
}

impl WorkflowEngine {
    /// Entry hook: a token reached a loop activity outside of an iteration.
    pub(crate) async fn enter_loop_activity(
        &self,
        instance_id: Uuid,
        mut token: Token,
        def: &ProcessDefinition,
        node_id: &str,
        characteristics: &LoopCharacteristics,
    ) -> EngineResult<NextAction> {
        let shadowed = local_variables(characteristics)
            .into_iter()
            .map(|name| {
                let previous = token.variables.get(&name).cloned();
                (name, previous)
            })
            .collect();
        let mut progress = MultiInstanceProgress {
            node_id: node_id.to_string(),
            total: 0,
            completed: 0,
            active_tokens: Vec::new(),
            elements: Vec::new(),
            outputs: Vec::new(),
            merged_variables: HashMap::new(),
            shadowed,
        };

        let action = match characteristics {
            LoopCharacteristics::MultiInstance(mi) => {
                let elements = instance_elements(node_id, mi, &token.variables)?;
                if elements.is_empty() {
                    return self
                        .skip_loop_activity(instance_id, token, def, node_id)
                        .await;
                }
                progress.total = elements.len();
                progress.outputs = vec![Value::Null; elements.len()];
                if mi.is_sequential {
                    set_iteration_variables(&mut token.variables, mi, 0, &progress, &elements[0]);
                    progress.elements = elements;
                    token.loop_instance = Some(LoopInstance {
                        node_id: node_id.to_string(),
                        index: 0,
                    });
                    NextAction::Continue(token)
                } else {
                    progress.merged_variables = token.variables.clone();
                    let children: Vec<Token> = (0..elements.len())
                        .map(|index| {
                            let mut child = Token::with_variables(node_id, token.variables.clone());
                            child.loop_instance = Some(LoopInstance {
                                node_id: node_id.to_string(),
                                index,
                            });
                            child
                        })
                        .collect();
                    progress.active_tokens = children.iter().map(|c| c.id).collect();
                    let mut children = children;
                    for (index, child) in children.iter_mut().enumerate() {
                        set_iteration_variables(
                            &mut child.variables,
                            mi,
                            index,
                            &progress,
                            &elements[index],
                        );
                    }
                    NextAction::MultiInstanceFork {
                        node_id: node_id.to_string(),
                        tokens: children,
                    }
                }
            }
            LoopCharacteristics::Standard(standard) => {
                if standard.test_before
                    && !condition_holds(def, &standard.loop_condition, &token.variables)
                {
                    return self
                        .skip_loop_activity(instance_id, token, def, node_id)
                        .await;
                }
                token.variables.insert(LOOP_COUNTER.into(), 0.into());
                token.loop_instance = Some(LoopInstance {
                    node_id: node_id.to_string(),
                    index: 0,
                });
                NextAction::Continue(token)
            }
        };

        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;
        let mut inst = inst_arc.write().await;
        if inst.multi_instance_state.contains_key(node_id) {
            return Err(EngineError::InvalidDefinition(format!(
                "Loop activity '{node_id}' is already active in instance {instance_id}"
            )));
        }
        inst.push_audit_log(format!(
            "🔁 Loop activity '{node_id}' started ({} planned instance(s))",
            progress.total
        ));
        inst.multi_instance_state
            .insert(node_id.to_string(), progress);
        Ok(action)
    }

    /// An interrupting boundary event `boundary_id` fired for `token`. When the
    /// token runs an iteration of the activity the boundary is attached to,
    /// all iterations end: siblings are cancelled and local variables restored.
    /// Boundary paths move the token past the boundary node themselves, so they
    /// call this before continuing.
    pub(crate) async fn interrupt_loop_activity(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def: &ProcessDefinition,
        boundary_id: &str,
    ) {
        let Some(marker) = token.loop_instance.clone() else {
            return;
        };
        if !is_boundary_of(def, boundary_id, &marker.node_id) {
            return;
        }
        token.discard_data_scope();
        token.loop_instance = None;
        let Some(inst_arc) = self.instances.get(&instance_id).await else {
            return;
        };
        let Some(progress) = inst_arc
            .write()
            .await
            .multi_instance_state
            .remove(&marker.node_id)
        else {
            return;
        };
        let others: Vec<Uuid> = progress
            .active_tokens
            .iter()
            .copied()
            .filter(|id| *id != token.id)
            .collect();
        self.cancel_loop_children(instance_id, &others).await;
        restore_shadowed(&mut token.variables, &progress);
        inst_arc.write().await.push_audit_log(format!(
            "🔁 Loop activity '{}' interrupted by boundary '{boundary_id}'",
            marker.node_id
        ));
    }

    /// Zero instances or a false `testBefore` condition: continue after the activity.
    async fn skip_loop_activity(
        &self,
        instance_id: Uuid,
        mut token: Token,
        def: &ProcessDefinition,
        node_id: &str,
    ) -> EngineResult<NextAction> {
        let next = resolve_next_target(def, node_id, &token.variables)?;
        token.current_node = next;
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            inst_arc.write().await.push_audit_log(format!(
                "🔁 Loop activity '{node_id}' skipped (no iterations)"
            ));
        }
        Ok(NextAction::Continue(token))
    }

    /// Exit hook: a marked token left its loop activity. Returns `None` when
    /// the token should be dispatched normally at its current node (boundary
    /// event or stale marker).
    pub(crate) async fn leave_loop_instance(
        &self,
        instance_id: Uuid,
        token: &mut Token,
        def: &ProcessDefinition,
        marker: LoopInstance,
    ) -> EngineResult<Option<NextAction>> {
        let node_id = marker.node_id.as_str();
        // Paths that stop at the boundary node itself (e.g. call-activity errors).
        if is_boundary_of(def, &token.current_node, node_id) {
            let boundary_id = token.current_node.clone();
            self.interrupt_loop_activity(instance_id, token, def, &boundary_id)
                .await;
            return Ok(None);
        }
        token.loop_instance = None;
        let Some(characteristics) = def.loops.get(node_id) else {
            return Ok(None);
        };
        let inst_arc = self
            .instances
            .get(&instance_id)
            .await
            .ok_or(EngineError::NoSuchInstance(instance_id))?;

        // Count and decide under one write lock: parallel children may finish concurrently.
        let outcome = {
            let mut inst = inst_arc.write().await;
            {
                let Some(progress) = inst.multi_instance_state.get_mut(node_id) else {
                    return Ok(None);
                };
                let running = match characteristics {
                    LoopCharacteristics::MultiInstance(mi) => {
                        count_multi_instance_iteration(def, mi, token, marker.index, progress)
                    }
                    LoopCharacteristics::Standard(standard) => {
                        progress.total += 1;
                        let exhausted = standard
                            .loop_maximum
                            .is_some_and(|max| progress.total >= max as usize);
                        if !exhausted
                            && condition_holds(def, &standard.loop_condition, &token.variables)
                        {
                            token
                                .variables
                                .insert(LOOP_COUNTER.into(), progress.total.into());
                            token.current_node = node_id.to_string();
                            token.loop_instance = Some(LoopInstance {
                                node_id: node_id.to_string(),
                                index: progress.total,
                            });
                            Running::NextIteration
                        } else {
                            Running::Done
                        }
                    }
                };
                match running {
                    Running::NextIteration => Outcome::NextIteration,
                    Running::ChildDone => Outcome::ChildDone,
                    Running::Done => match inst.multi_instance_state.remove(node_id) {
                        Some(progress) => Outcome::Finished(Box::new(progress)),
                        None => return Ok(None),
                    },
                }
            }
        };

        match outcome {
            Outcome::NextIteration => Ok(Some(NextAction::Continue(token.clone()))),
            Outcome::ChildDone => {
                self.complete_branch_token(instance_id, token.id).await?;
                inst_arc.write().await.tokens.remove(&token.id);
                Ok(Some(NextAction::Complete))
            }
            Outcome::Finished(progress) => {
                let mut progress = *progress;
                let parallel = matches!(
                    characteristics,
                    LoopCharacteristics::MultiInstance(mi) if !mi.is_sequential
                );
                let mut vars = if parallel {
                    self.complete_branch_token(instance_id, token.id).await?;
                    {
                        let mut inst = inst_arc.write().await;
                        inst.tokens.remove(&token.id);
                        if matches!(inst.state, InstanceState::ParallelExecution { .. }) {
                            inst.state = InstanceState::Running;
                        }
                    }
                    let remaining = std::mem::take(&mut progress.active_tokens);
                    self.cancel_loop_children(instance_id, &remaining).await;
                    std::mem::take(&mut progress.merged_variables)
                } else {
                    token.variables.clone()
                };
                restore_shadowed(&mut vars, &progress);
                if let LoopCharacteristics::MultiInstance(mi) = characteristics
                    && let Some(name) = &mi.output_collection
                {
                    vars.insert(name.clone(), Value::Array(progress.outputs.clone()));
                }
                let mut next = if parallel {
                    let mut merged = Token::with_variables(node_id, vars);
                    merged.is_merged = true;
                    merged
                } else {
                    token.variables = vars;
                    token.clone()
                };
                next.loop_instance = None;
                let iterations = match characteristics {
                    LoopCharacteristics::MultiInstance(_) => progress.completed,
                    LoopCharacteristics::Standard(_) => progress.total,
                };
                let action = self
                    .continue_after_loop(instance_id, next, def, node_id, iterations)
                    .await?;
                Ok(Some(action))
            }
        }
    }

    /// Moves the token past the loop activity, evaluating the outgoing flows
    /// with the final variables.
    async fn continue_after_loop(
        &self,
        instance_id: Uuid,
        mut token: Token,
        def: &ProcessDefinition,
        node_id: &str,
        iterations: usize,
    ) -> EngineResult<NextAction> {
        token.loop_instance = None;
        token.current_node = resolve_next_target(def, node_id, &token.variables)?;
        if let Some(inst_arc) = self.instances.get(&instance_id).await {
            let mut inst = inst_arc.write().await;
            inst.multi_instance_state.remove(node_id);
            inst.variables = token.variables.clone();
            inst.current_node = token.current_node.clone();
            inst.push_audit_log(format!(
                "🔁 Loop activity '{node_id}' completed after {iterations} iteration(s)"
            ));
        }
        Ok(NextAction::Continue(token))
    }

    /// Cancels still-running parallel iterations (completion condition or
    /// interrupting boundary): wait states, pending tasks and their tokens.
    async fn cancel_loop_children(&self, instance_id: Uuid, token_ids: &[Uuid]) {
        for token_id in token_ids {
            self.clear_wait_states_for_token(instance_id, token_id)
                .await;

            let user_tasks: Vec<Uuid> = self
                .pending_user_tasks
                .iter()
                .filter(|t| t.instance_id == instance_id && t.token_id == *token_id)
                .map(|t| t.task_id)
                .collect();
            for id in user_tasks {
                self.pending_user_tasks.remove(&id);
                self.remove_persisted_user_task(id).await;
            }

            let service_tasks: Vec<Uuid> = self
                .pending_service_tasks
                .iter()
                .filter(|t| t.instance_id == instance_id && t.token_id == *token_id)
                .map(|t| t.id)
                .collect();
            for id in service_tasks {
                self.remove_pending_service_task(&id);
                self.remove_persisted_service_task(id).await;
            }

            if let Some(inst_arc) = self.instances.get(&instance_id).await {
                let mut inst = inst_arc.write().await;
                inst.tokens.remove(token_id);
                if let Some(active) = inst
                    .active_tokens
                    .iter_mut()
                    .find(|at| at.token.id == *token_id)
                {
                    active.completed = true;
                }
            }
        }
        if !token_ids.is_empty()
            && let Some(inst_arc) = self.instances.get(&instance_id).await
        {
            inst_arc.write().await.push_audit_log(format!(
                "🔁 {} remaining loop iteration(s) cancelled",
                token_ids.len()
            ));
        }
    }
}

fn is_boundary_of(def: &ProcessDefinition, node: &str, activity: &str) -> bool {
    matches!(
        def.get_node(node),
        Some(
            BpmnElement::BoundaryTimerEvent { attached_to, .. }
            | BpmnElement::BoundaryMessageEvent { attached_to, .. }
            | BpmnElement::BoundarySignalEvent { attached_to, .. }
            | BpmnElement::BoundaryErrorEvent { attached_to, .. }
            | BpmnElement::BoundaryEscalationEvent { attached_to, .. }
        ) if attached_to == activity
    )
}
