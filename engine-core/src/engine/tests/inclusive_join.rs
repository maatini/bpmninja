//! Inclusive join follows the branches actually taken at the matching split,
//! not every structural incoming flow.

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;

fn inclusive_split_join_definition() -> ProcessDefinition {
    ProcessDefinitionBuilder::new("inclusive_split_join")
        .node("start", BpmnElement::StartEvent)
        .node("split", BpmnElement::InclusiveGateway)
        .node("join", BpmnElement::InclusiveGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "split")
        .conditional_flow("split", "join", "a > 0")
        .conditional_flow("split", "join", "b > 0")
        .conditional_flow("split", "join", "c > 0")
        .flow("join", "end")
        .build()
        .unwrap()
}

async fn start_inclusive(vars: HashMap<String, Value>) -> InstanceState {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(inclusive_split_join_definition())
        .await;
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();
    engine.get_instance_state(inst_id).await.unwrap()
}

#[tokio::test]
async fn inclusive_join_one_of_three_completes() {
    let mut vars = HashMap::new();
    vars.insert("a".into(), Value::Number(1.into()));
    vars.insert("b".into(), Value::Number(0.into()));
    vars.insert("c".into(), Value::Number(0.into()));

    assert_eq!(start_inclusive(vars).await, InstanceState::Completed);
}

#[tokio::test]
async fn inclusive_join_two_of_three_completes() {
    let mut vars = HashMap::new();
    vars.insert("a".into(), Value::Number(1.into()));
    vars.insert("b".into(), Value::Number(1.into()));
    vars.insert("c".into(), Value::Number(0.into()));

    assert_eq!(start_inclusive(vars).await, InstanceState::Completed);
}

/// Inner join Continue must not overwrite the outer join's expected_count.
#[tokio::test]
async fn inclusive_nested_inner_join_does_not_collapse_outer() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("inclusive_nested")
        .node("start", BpmnElement::StartEvent)
        .node("outer_split", BpmnElement::InclusiveGateway)
        .node("inner_split", BpmnElement::InclusiveGateway)
        .node("inner_join", BpmnElement::InclusiveGateway)
        .node("outer_join", BpmnElement::InclusiveGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "outer_split")
        .conditional_flow("outer_split", "inner_split", "a > 0")
        .conditional_flow("outer_split", "outer_join", "b > 0")
        .conditional_flow("inner_split", "inner_join", "c > 0")
        .conditional_flow("inner_split", "inner_join", "d > 0")
        .flow("inner_join", "outer_join")
        .flow("outer_join", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let mut vars = HashMap::new();
    vars.insert("a".into(), Value::Number(1.into()));
    vars.insert("b".into(), Value::Number(1.into()));
    vars.insert("c".into(), Value::Number(1.into()));
    vars.insert("d".into(), Value::Number(0.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}
