//! XOR merge: an unconditional outgoing flow is a match when it is not the default.

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn exclusive_gateway_merge_unconditional_without_default_completes() {
    let engine = WorkflowEngine::new();

    // `merge_in` is only a second incoming edge. Gateway validation requires
    // at least two incoming or two outgoing flows; the token path is start → gw → end.
    let def = ProcessDefinitionBuilder::new("xor_merge")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::ExclusiveGateway { default: None })
        .node("merge_in", BpmnElement::UserTask("nobody".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .flow("merge_in", "gw")
        .flow("gw", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn exclusive_gateway_true_condition_beats_unconditional_non_default() {
    let engine = WorkflowEngine::new();

    // Unconditional flow is declared first and is not the default target.
    let def = ProcessDefinitionBuilder::new("xor_cond_over_uncond")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::ExclusiveGateway { default: None })
        .node("via_unconditional", BpmnElement::EndEvent)
        .node("via_condition", BpmnElement::EndEvent)
        .flow("start", "gw")
        .flow("gw", "via_unconditional")
        .conditional_flow("gw", "via_condition", "x == 1")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let mut vars = HashMap::new();
    vars.insert("x".into(), Value::Number(1.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    let log = engine.get_audit_log(inst_id).await.unwrap();
    let gw_entry = log
        .iter()
        .find(|entry| entry.contains("Exclusive gateway"))
        .unwrap();
    assert!(
        gw_entry.contains("via_condition"),
        "true condition must win over unconditional non-default: {gw_entry}"
    );
    assert!(
        !gw_entry.contains("via_unconditional"),
        "unconditional path must not be taken: {gw_entry}"
    );
}
