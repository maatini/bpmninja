//! Shared helpers for split unit tests (M5).

use super::super::*;
use crate::domain::ListenerEvent;
use crate::domain::ProcessDefinitionBuilder;
use serde_json::Value;
use std::collections::HashMap;

pub(super) async fn complete_all_service_tasks(
    engine: &WorkflowEngine,
    worker: &str,
    vars: HashMap<String, Value>,
) {
    let mut to_complete = Vec::new();
    for task in engine
        .pending_service_tasks
        .iter()
        .map(|r| r.value().clone())
    {
        to_complete.push((task.id, task.topic.clone()));
    }
    for (id, topic) in to_complete {
        let _ = engine
            .fetch_and_lock_service_tasks(worker, 10, std::slice::from_ref(&topic), 60000)
            .await;
        engine
            .complete_service_task(id, worker, vars.clone())
            .await
            .unwrap();
    }
}

pub(super) async fn setup_linear_engine() -> (WorkflowEngine, Uuid) {
    let engine = WorkflowEngine::new();

    // Register a simple service handler

    let def = ProcessDefinitionBuilder::new("linear")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "validate".into(),
                multi_instance: None,
            },
        )
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    (engine, def_key)
}

// -----------------------------------------------------------------------
// E2E: XOR Gateway with 2 UserTasks (condition: x > 0 / default)
// -----------------------------------------------------------------------

/// Helper: builds a workflow with XOR gateway routing to two user tasks.
///
/// ```text
/// Start → XOR Gateway → (x > 0) → user-task-1 ("author")   → End
///                     → (default) → user-task-2 ("reviewer") → End
/// ```
pub(super) fn build_xor_user_task_definition() -> ProcessDefinition {
    ProcessDefinitionBuilder::new("xor_user_tasks")
        .node("start", BpmnElement::StartEvent)
        .node(
            "gw",
            BpmnElement::ExclusiveGateway {
                default: Some("user-task-2".into()),
            },
        )
        .node("user-task-1", BpmnElement::UserTask("author".into()))
        .node("user-task-2", BpmnElement::UserTask("reviewer".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "user-task-1", "x > 0")
        .flow("gw", "user-task-2")
        .flow("user-task-1", "end")
        .flow("user-task-2", "end")
        .build()
        .unwrap()
}

pub(super) fn build_script_test_definition() -> ProcessDefinition {
    ProcessDefinitionBuilder::new("script_test")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "calculate".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .listener(
            "svc",
            ListenerEvent::Start,
            "x = x * 2; let result = \"small\"; if x > 10 { result = \"big\" }",
        )
        .build()
        .unwrap()
}

pub(super) fn build_child_error_process(code: &str) -> ProcessDefinition {
    ProcessDefinitionBuilder::new("child_proc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "err_end",
            BpmnElement::ErrorEndEvent {
                error_code: String::from(code),
            },
        )
        .flow("start", "err_end")
        .build()
        .unwrap()
}

// ---------------------------------------------------------------------------
// migrate_instance tests
// ---------------------------------------------------------------------------

/// Hilfsfunktion: deployt zwei Versionen eines Prozesses mit gleicher bpmn_id.
/// v1 hat Nodes start→ut→end, v2 hat dieselben IDs (kein Mapping nötig).
pub(super) async fn setup_migration_engine_same_ids() -> (WorkflowEngine, Uuid, Uuid) {
    let engine = WorkflowEngine::new();

    let def_v1 = ProcessDefinitionBuilder::new("migratable")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();

    let def_v2 = ProcessDefinitionBuilder::new("migratable")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();

    let (key_v1, _) = engine.deploy_definition(def_v1).await;
    let (key_v2, _) = engine.deploy_definition(def_v2).await;

    (engine, key_v1, key_v2)
}
