//! Embedded sub-processes must run without a synthetic "noop" service task.
//!
//! The parser flattens an embedded sub-process into the parent graph and points
//! `EmbeddedSubProcess::start_node_id` directly at the node following the internal
//! start event. Entering the scope therefore never creates an external task.

use super::super::*;
use super::helpers::complete_all_service_tasks;
use crate::domain::ProcessDefinitionBuilder;

fn embedded_sub_process_definition(inner: BpmnElement) -> ProcessDefinition {
    ProcessDefinitionBuilder::new("embedded_sub_process")
        .node("start", BpmnElement::StartEvent)
        .node(
            "sub",
            BpmnElement::EmbeddedSubProcess {
                start_node_id: "sub_task".into(),
            },
        )
        .node("sub_task", inner)
        .node(
            "sub_end",
            BpmnElement::SubProcessEndEvent {
                sub_process_id: "sub".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "sub")
        .flow("sub", "end")
        .flow("sub_task", "sub_end")
        .build()
        .unwrap()
}

#[tokio::test]
async fn embedded_sub_process_completes_without_noop_worker() {
    let engine = WorkflowEngine::new();
    let def = embedded_sub_process_definition(BpmnElement::ScriptTask {
        script: "let sub_ran = true;".into(),
        multi_instance: None,
    });
    let (def_key, _) = engine.deploy_definition(def).await;

    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed,
        "embedded sub-process must run to its end event"
    );
    assert!(
        engine.pending_service_tasks.is_empty(),
        "no external task may be created for the sub-process entry"
    );
}

#[tokio::test]
async fn embedded_sub_process_runs_its_own_service_task() {
    let engine = WorkflowEngine::new();
    let def = embedded_sub_process_definition(BpmnElement::ServiceTask {
        topic: "sub_work".into(),
        multi_instance: None,
    });
    let (def_key, _) = engine.deploy_definition(def).await;

    let inst_id = engine.start_instance(def_key).await.unwrap();

    let topics: Vec<String> = engine
        .pending_service_tasks
        .iter()
        .map(|task| task.value().topic.clone())
        .collect();
    assert_eq!(
        topics,
        vec!["sub_work".to_string()],
        "only the real task inside the sub-process may be pending"
    );

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert!(engine.pending_service_tasks.is_empty());
}

#[tokio::test]
async fn nested_embedded_sub_process_completes_without_external_task() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("nested_embedded_sub_process")
        .node("start", BpmnElement::StartEvent)
        .node(
            "outer",
            BpmnElement::EmbeddedSubProcess {
                start_node_id: "inner".into(),
            },
        )
        .node(
            "inner",
            BpmnElement::EmbeddedSubProcess {
                start_node_id: "inner_task".into(),
            },
        )
        .node(
            "inner_task",
            BpmnElement::ScriptTask {
                script: "let inner_ran = true;".into(),
                multi_instance: None,
            },
        )
        .node(
            "inner_end",
            BpmnElement::SubProcessEndEvent {
                sub_process_id: "inner".into(),
            },
        )
        .node(
            "outer_end",
            BpmnElement::SubProcessEndEvent {
                sub_process_id: "outer".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "outer")
        .flow("outer", "end")
        .flow("inner", "outer_end")
        .flow("inner_task", "inner_end")
        .build()
        .unwrap();
    let (def_key, _) = engine.deploy_definition(def).await;

    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed,
        "nested sub-processes must run through to the top-level end event"
    );
    assert!(
        engine.pending_service_tasks.is_empty(),
        "nesting must not create an external task"
    );
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(
        inst.tokens.is_empty(),
        "no token may remain after the nested sub-process completed"
    );
}

#[tokio::test]
async fn sub_process_default_path_with_multiple_end_events_completes_once() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("sub_process_default_path")
        .node("start", BpmnElement::StartEvent)
        .node(
            "sub",
            BpmnElement::EmbeddedSubProcess {
                start_node_id: "sub_gw".into(),
            },
        )
        .node(
            "sub_gw",
            BpmnElement::ExclusiveGateway {
                default: Some("sub_other_end".into()),
            },
        )
        .node(
            "sub_primary_end",
            BpmnElement::SubProcessEndEvent {
                sub_process_id: "sub".into(),
            },
        )
        .node(
            "sub_other_end",
            BpmnElement::SubProcessEndEvent {
                sub_process_id: "sub".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "sub")
        .flow("sub", "end")
        .conditional_flow("sub_gw", "sub_primary_end", "approved == true")
        .flow("sub_gw", "sub_other_end")
        .build()
        .unwrap();
    let (def_key, _) = engine.deploy_definition(def).await;

    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed,
        "the default path must reach the top-level end event"
    );
    assert!(engine.pending_service_tasks.is_empty());

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(
        log.iter()
            .any(|entry| entry.contains("Exclusive gateway 'sub_gw'")
                && entry.contains("sub_other_end")),
        "the gateway must take the default path: {log:?}"
    );
    let completions = log
        .iter()
        .filter(|entry| entry.contains("Process completed at end event 'end'"))
        .count();
    assert_eq!(
        completions, 1,
        "the instance must complete exactly once: {log:?}"
    );
}
