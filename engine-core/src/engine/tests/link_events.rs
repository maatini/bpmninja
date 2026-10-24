//! Link throw events jump directly to the successor of the matching link catch.
//!
//! The parser drops the catch node and points the throw at the catch's single
//! outgoing target. These tests build that flattened shape explicitly.

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn link_throw_jumps_to_catch_target_without_halting() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("link_events")
        .node("start", BpmnElement::StartEvent)
        .node(
            "throw1",
            BpmnElement::LinkThrowEvent {
                target: "after".into(),
            },
        )
        .node(
            "after",
            BpmnElement::ScriptTask {
                script: "let jumped = true;".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "throw1")
        // Synthetic edge produced by the parser for the dropped catch node.
        .flow("throw1", "after")
        .flow("after", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed,
        "the link throw must continue at the catch target and complete"
    );
    assert!(
        engine.pending_service_tasks.is_empty(),
        "no task may be created for a link event"
    );

    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(
        details.variables.get("jumped"),
        Some(&serde_json::json!(true)),
        "the target node must have been executed after the jump"
    );
}

#[tokio::test]
async fn link_throw_inside_sub_process_reaches_target() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("link_events_sub_process")
        .node("start", BpmnElement::StartEvent)
        .node(
            "sub",
            BpmnElement::EmbeddedSubProcess {
                start_node_id: "sub_throw".into(),
            },
        )
        .node(
            "sub_throw",
            BpmnElement::LinkThrowEvent {
                target: "sub_after".into(),
            },
        )
        .node(
            "sub_after",
            BpmnElement::ScriptTask {
                script: "let sub_jumped = true;".into(),
                multi_instance: None,
            },
        )
        .node(
            "sub_end",
            BpmnElement::SubProcessEndEvent {
                sub_process_id: "sub".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "sub")
        .flow("sub", "end")
        .flow("sub_throw", "sub_after")
        .flow("sub_after", "sub_end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(
        details.variables.get("sub_jumped"),
        Some(&serde_json::json!(true)),
        "a link pair inside a sub-process must reach its target"
    );
}
