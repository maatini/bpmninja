//! Paralleles und sequentielles Multi-Instance für Service Tasks (R8).

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

fn sequential_mi(
    loop_cardinality: Option<&str>,
    collection: Option<&str>,
    element_variable: Option<&str>,
) -> MultiInstanceDef {
    MultiInstanceDef {
        is_sequential: true,
        ..parallel_mi(loop_cardinality, collection, element_variable)
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

async fn lock_single_task(engine: &WorkflowEngine) -> PendingServiceTask {
    let mut locked = engine
        .fetch_and_lock_service_tasks("worker", 10, &["work".to_string()], 600)
        .await;
    assert_eq!(locked.len(), 1, "exactly one sequential task must be open");
    locked.remove(0)
}

#[tokio::test]
async fn sequential_loop_cardinality_runs_one_task_at_a_time() {
    let (engine, def_key) = deploy_mi_engine(sequential_mi(Some("3"), None, None)).await;
    let instance_id = engine.start_instance(def_key).await.unwrap();

    let tasks = pending_tasks(&engine, instance_id);
    assert_eq!(tasks.len(), 1);
    assert_eq!(loop_counter(&tasks[0]), Some(1));

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
    assert_eq!(progress.items.len(), 3);
    assert!(progress.sequential_token.is_some());

    let mut previous_task_id = tasks[0].id;
    for counter in 1..=2u64 {
        let task = lock_single_task(&engine).await;
        assert_eq!(task.id, previous_task_id);
        assert_eq!(loop_counter(&task), Some(counter));

        engine
            .complete_service_task(task.id, "worker", HashMap::new())
            .await
            .unwrap();

        let tasks = pending_tasks(&engine, instance_id);
        assert_eq!(
            tasks.len(),
            1,
            "a new task must appear after item {counter}"
        );
        assert_ne!(tasks[0].id, previous_task_id, "next task must be new");
        assert_eq!(loop_counter(&tasks[0]), Some(counter + 1));
        previous_task_id = tasks[0].id;

        let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
        assert!(
            matches!(inst.state, InstanceState::WaitingOnServiceTask { task_id } if task_id == previous_task_id),
            "instance must wait for the next item after {counter}"
        );
        assert_eq!(inst.tokens.len(), 1, "token must survive until last item");
        let progress = inst
            .multi_instance_state
            .get("svc")
            .expect("MI progress entry");
        assert_eq!(progress.completed, counter as usize);
        assert_eq!(progress.total, 3);
    }

    let last_task = lock_single_task(&engine).await;
    assert_eq!(last_task.id, previous_task_id);
    assert_eq!(loop_counter(&last_task), Some(3));
    engine
        .complete_service_task(last_task.id, "worker", HashMap::new())
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
async fn sequential_collection_preserves_item_order_and_element_variable() {
    let (engine, def_key) =
        deploy_mi_engine(sequential_mi(None, Some("orders"), Some("order"))).await;

    let mut vars = HashMap::new();
    vars.insert("orders".to_string(), json!(["a", "b", "c"]));
    let instance_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    for (idx, expected) in ["a", "b", "c"].iter().enumerate() {
        let tasks = pending_tasks(&engine, instance_id);
        assert_eq!(tasks.len(), 1);
        assert_eq!(loop_counter(&tasks[0]), Some(idx as u64 + 1));
        assert_eq!(
            tasks[0].variables_snapshot.get("order"),
            Some(&json!(expected))
        );

        let task = lock_single_task(&engine).await;
        assert_eq!(task.id, tasks[0].id);
        engine
            .complete_service_task(task.id, "worker", HashMap::new())
            .await
            .unwrap();
    }

    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert!(inst.multi_instance_state.is_empty());
    assert!(pending_tasks(&engine, instance_id).is_empty());
    assert_eq!(inst.variables.get("order"), Some(&json!("c")));
    assert_eq!(inst.variables.get("loopCounter"), Some(&json!(3)));
}

#[tokio::test]
async fn sequential_empty_collection_skips_task_and_continues() {
    let (engine, def_key) =
        deploy_mi_engine(sequential_mi(None, Some("orders"), Some("order"))).await;

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
}

#[tokio::test]
async fn sequential_failure_keeps_progress_and_single_open_task() {
    let (engine, def_key) = deploy_mi_engine(sequential_mi(Some("2"), None, None)).await;
    let instance_id = engine.start_instance(def_key).await.unwrap();

    let first = lock_single_task(&engine).await;
    assert_eq!(loop_counter(&first), Some(1));
    engine
        .fail_service_task(
            first.id,
            "worker",
            Some(1),
            Some("transient error".into()),
            None,
        )
        .await
        .unwrap();

    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    assert_eq!(inst.tokens.len(), 1);
    let progress = inst
        .multi_instance_state
        .get("svc")
        .expect("MI progress entry");
    assert_eq!(progress.completed, 0, "failed item must not count as done");
    assert_eq!(pending_tasks(&engine, instance_id).len(), 1);

    let retried = lock_single_task(&engine).await;
    assert_eq!(retried.id, first.id);
    engine
        .complete_service_task(retried.id, "worker", HashMap::new())
        .await
        .unwrap();

    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    let progress = inst
        .multi_instance_state
        .get("svc")
        .expect("MI progress entry");
    assert_eq!(progress.completed, 1);
    let second = lock_single_task(&engine).await;
    assert_eq!(loop_counter(&second), Some(2));

    engine
        .fail_service_task(second.id, "worker", Some(0), Some("boom".into()), None)
        .await
        .unwrap();
    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    let progress = inst
        .multi_instance_state
        .get("svc")
        .expect("MI progress entry");
    assert_eq!(progress.completed, 1, "incident must not lose progress");
    assert_eq!(pending_tasks(&engine, instance_id).len(), 1);

    engine.retry_incident(second.id, Some(1)).await.unwrap();
    let retried = lock_single_task(&engine).await;
    assert_eq!(retried.id, second.id);
    engine
        .complete_service_task(retried.id, "worker", HashMap::new())
        .await
        .unwrap();

    let inst = engine.get_instance_or_archived(instance_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert!(inst.multi_instance_state.is_empty());
    assert!(pending_tasks(&engine, instance_id).is_empty());
}
