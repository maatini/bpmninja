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
