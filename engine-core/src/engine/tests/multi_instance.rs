//! Paralleles Multi-Instance für Service Tasks (R8).

use super::super::*;
use crate::domain::{MultiInstanceDef, ProcessDefinitionBuilder};
use serde_json::{Value, json};
use std::collections::HashMap;

fn parallel_mi(
    loop_cardinality: Option<&str>,
    collection: Option<&str>,
    element_variable: Option<&str>,
) -> MultiInstanceDef {
    MultiInstanceDef {
        is_sequential: false,
        loop_cardinality: loop_cardinality.map(str::to_string),
        collection: collection.map(str::to_string),
        element_variable: element_variable.map(str::to_string),
    }
}

async fn deploy_mi_engine(mi: MultiInstanceDef) -> (WorkflowEngine, Uuid) {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("mi_service")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "work".into(),
                multi_instance: Some(mi),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    (engine, def_key)
}

fn pending_tasks(engine: &WorkflowEngine, instance_id: Uuid) -> Vec<PendingServiceTask> {
    engine
        .pending_service_tasks
        .iter()
        .filter(|t| t.instance_id == instance_id)
        .map(|t| t.value().clone())
        .collect()
}

fn loop_counter(task: &PendingServiceTask) -> Option<u64> {
    task.variables_snapshot
        .get("loopCounter")
        .and_then(Value::as_u64)
}

#[tokio::test]
async fn loop_cardinality_creates_three_pending_tasks() {
    let (engine, def_key) = deploy_mi_engine(parallel_mi(Some("3"), None, None)).await;
    let instance_id = engine.start_instance(def_key).await.unwrap();

    let tasks = pending_tasks(&engine, instance_id);
    assert_eq!(tasks.len(), 3);

    let mut counters: Vec<u64> = tasks.iter().filter_map(loop_counter).collect();
    counters.sort_unstable();
    assert_eq!(counters, vec![1, 2, 3]);
    assert!(tasks.iter().all(|t| t.token_id == tasks[0].token_id));

    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    assert!(matches!(
        inst.state,
        InstanceState::WaitingOnServiceTask { .. }
    ));
    assert_eq!(inst.tokens.len(), 1);
    let progress = inst
        .multi_instance_state
        .get("svc")
        .expect("MI progress entry");
    assert_eq!(progress.total, 3);
    assert_eq!(progress.completed, 0);
}

#[tokio::test]
async fn instance_waits_until_last_item_then_advances_once() {
    let (engine, def_key) = deploy_mi_engine(parallel_mi(Some("3"), None, None)).await;
    let instance_id = engine.start_instance(def_key).await.unwrap();

    let locked = engine
        .fetch_and_lock_service_tasks("worker", 10, &["work".to_string()], 600)
        .await;
    assert_eq!(locked.len(), 3);
    let ids: HashMap<u64, Uuid> = locked
        .iter()
        .map(|t| (loop_counter(t).expect("loopCounter"), t.id))
        .collect();

    for counter in [1, 2] {
        engine
            .complete_service_task(ids[&counter], "worker", HashMap::new())
            .await
            .unwrap();

        let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
        assert!(
            matches!(inst.state, InstanceState::WaitingOnServiceTask { .. }),
            "instance must stay waiting after item {counter}: {:?}",
            inst.state
        );
        assert_eq!(inst.tokens.len(), 1, "token must survive until last item");
        let progress = inst
            .multi_instance_state
            .get("svc")
            .expect("MI progress entry");
        assert_eq!(progress.completed, counter as usize);
        assert_eq!(progress.total, 3);
    }

    engine
        .complete_service_task(ids[&3], "worker", HashMap::new())
        .await
        .unwrap();

    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert!(inst.multi_instance_state.is_empty());
    assert!(inst.tokens.is_empty());
    assert!(pending_tasks(&engine, instance_id).is_empty());
    let end_entries = inst
        .audit_log
        .iter()
        .filter(|line| line.contains("Process completed at end event"))
        .count();
    assert_eq!(end_entries, 1, "instance must advance exactly once");
}

#[tokio::test]
async fn collection_items_are_exposed_in_snapshots() {
    let (engine, def_key) =
        deploy_mi_engine(parallel_mi(None, Some("orders"), Some("order"))).await;

    let mut vars = HashMap::new();
    vars.insert("orders".to_string(), json!(["a", "b"]));
    let instance_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    let locked = engine
        .fetch_and_lock_service_tasks("worker", 10, &["work".to_string()], 600)
        .await;
    let mut tasks = locked;
    tasks.sort_by_key(loop_counter);
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].variables_snapshot.get("order"), Some(&json!("a")));
    assert_eq!(tasks[1].variables_snapshot.get("order"), Some(&json!("b")));
    assert_eq!(loop_counter(&tasks[0]), Some(1));
    assert_eq!(loop_counter(&tasks[1]), Some(2));

    for task in &tasks {
        engine
            .complete_service_task(task.id, "worker", HashMap::new())
            .await
            .unwrap();
    }

    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.variables.get("order"), Some(&json!("b")));
    assert_eq!(inst.variables.get("loopCounter"), Some(&json!(2)));
}

#[tokio::test]
async fn empty_collection_skips_task_and_continues() {
    let (engine, def_key) =
        deploy_mi_engine(parallel_mi(None, Some("orders"), Some("order"))).await;

    let mut vars = HashMap::new();
    vars.insert("orders".to_string(), json!([]));
    let instance_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    assert!(pending_tasks(&engine, instance_id).is_empty());
    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert!(inst.multi_instance_state.is_empty());
    assert!(inst.tokens.is_empty());
}

#[tokio::test]
async fn missing_collection_variable_yields_clear_error() {
    let (engine, def_key) =
        deploy_mi_engine(parallel_mi(None, Some("orders"), Some("order"))).await;

    let err = engine.start_instance(def_key).await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("orders") && msg.contains("not found") && msg.contains("svc"),
        "unexpected error: {msg}"
    );
}

#[tokio::test]
async fn collection_with_wrong_type_yields_clear_error() {
    let (engine, def_key) =
        deploy_mi_engine(parallel_mi(None, Some("orders"), Some("order"))).await;

    let mut vars = HashMap::new();
    vars.insert("orders".to_string(), json!("not-an-array"));
    let err = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("orders") && msg.contains("must be an array") && msg.contains("string"),
        "unexpected error: {msg}"
    );
}
