//! P3: `persist_instance` must not hold the instance lock across the network save,
//! and concurrent saves of one instance must land in order.

use super::super::*;
use super::helpers::InstrumentedPersistence;
use crate::domain::ProcessDefinitionBuilder;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Engine with slow-save persistence and one instance parked at a user task.
async fn setup(
    first_delay: Duration,
    later_delay: Duration,
) -> (Arc<WorkflowEngine>, Arc<InstrumentedPersistence>, Uuid) {
    let persistence = Arc::new(InstrumentedPersistence::new(first_delay, later_delay));
    let engine = Arc::new(
        WorkflowEngine::new().with_persistence(persistence.clone() as Arc<dyn WorkflowPersistence>),
    );
    let def = ProcessDefinitionBuilder::new("persist_lock")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("tester".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    persistence.slow.store(true, Ordering::SeqCst);
    (engine, persistence, id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn instance_stays_readable_while_save_is_in_flight() {
    let (engine, _p, id) = setup(Duration::from_millis(800), Duration::from_millis(800)).await;

    let writer = {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine
                .update_instance_variables(id, HashMap::from([("x".to_string(), json!(1))]))
                .await
        })
    };
    // Let the update reach the slow save.
    tokio::time::sleep(Duration::from_millis(150)).await;

    let read = tokio::time::timeout(Duration::from_millis(300), engine.get_instance_details(id))
        .await
        .expect("instance read blocked by an in-flight save");
    assert_eq!(read.unwrap().variables.get("x"), Some(&json!(1)));

    writer.await.unwrap().unwrap();
    engine.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_saves_persist_the_latest_state() {
    // First save is slow, the second fast: without ordering the older
    // snapshot would be written last.
    let (engine, persistence, id) =
        setup(Duration::from_millis(400), Duration::from_millis(5)).await;

    let first = {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine
                .update_instance_variables(id, HashMap::from([("x".to_string(), json!(1))]))
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    engine
        .update_instance_variables(id, HashMap::from([("x".to_string(), json!(2))]))
        .await
        .unwrap();
    first.await.unwrap().unwrap();

    let live = engine.get_instance_details(id).await.unwrap();
    let persisted = persistence
        .inner
        .list_instances()
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.id == id)
        .expect("instance was persisted");
    assert_eq!(live.variables.get("x"), Some(&json!(2)));
    assert_eq!(persisted.variables.get("x"), live.variables.get("x"));
    engine.shutdown().await;
}
