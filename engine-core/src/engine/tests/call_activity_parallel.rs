//! Call Activity: parallel resume and spawn failures must not hang the parent.

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;

fn child_process(id: &str) -> crate::domain::ProcessDefinition {
    ProcessDefinitionBuilder::new(id)
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap()
}

#[tokio::test]
async fn parallel_call_activities_both_children_complete_parent() {
    let engine = WorkflowEngine::new();
    let (_, _) = engine.deploy_definition(child_process("child_a")).await;
    let (_, _) = engine.deploy_definition(child_process("child_b")).await;

    let parent_def = ProcessDefinitionBuilder::new("parent_parallel_calls")
        .node("start", BpmnElement::StartEvent)
        .node("split", BpmnElement::ParallelGateway)
        .node(
            "call_a",
            BpmnElement::CallActivity {
                called_element: "child_a".into(),
            },
        )
        .node(
            "call_b",
            BpmnElement::CallActivity {
                called_element: "child_b".into(),
            },
        )
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "split")
        .flow("split", "call_a")
        .flow("split", "call_b")
        .flow("call_a", "join")
        .flow("call_b", "join")
        .flow("join", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(parent_def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn call_activity_target_not_found_completes_parent_with_error() {
    let engine = WorkflowEngine::new();
    let parent_def = ProcessDefinitionBuilder::new("parent_missing_child")
        .node("start", BpmnElement::StartEvent)
        .node(
            "call",
            BpmnElement::CallActivity {
                called_element: "missing_child".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "call")
        .flow("call", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(parent_def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let state = engine.get_instance_state(inst_id).await.unwrap();
    assert_eq!(
        state,
        InstanceState::CompletedWithError {
            error_code: "CALL_ACTIVITY_TARGET_NOT_FOUND".into(),
        }
    );
    assert!(
        !matches!(state, InstanceState::WaitingOnCallActivity { .. }),
        "Parent must not hang in WaitingOnCallActivity after spawn failure"
    );

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(
        log.iter()
            .any(|l| l.contains("CALL_ACTIVITY_TARGET_NOT_FOUND")),
        "Audit log should record spawn failure, got: {log:?}"
    );
}

#[tokio::test]
async fn linear_call_activity_child_complete_parent_completes() {
    let engine = WorkflowEngine::new();
    let (_, _) = engine.deploy_definition(child_process("child_proc")).await;

    let parent_def = ProcessDefinitionBuilder::new("parent_linear_call")
        .node("start", BpmnElement::StartEvent)
        .node(
            "call",
            BpmnElement::CallActivity {
                called_element: "child_proc".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "call")
        .flow("call", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(parent_def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}
