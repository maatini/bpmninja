//! `persist_instance` lässt den Instanz-Lock vor dem I/O los und serialisiert
//! Speicherungen derselben Instanz über [`PersistLocks`].
//!
//! Die Tests laufen auf dem Current-Thread-Runtime. Ein `yield_now` gibt jeder
//! anderen bereiten Task genau einen Poll. Eine Task, die am Persistenz-Lock
//! hängt, ist nicht bereit — sie kann also keinen zweiten `save_instance`
//! starten, egal wie oft geyieldet wird. Es gibt keine zeitbasierten Schlaf-
//! Synchronisationen.

use super::super::persistence_ops::PersistLocks;
use super::super::retry_queue::PersistJob;
use super::super::*;
use super::helpers::{InstrumentedPersistence, SaveArrival};
use crate::domain::ProcessDefinitionBuilder;
use serde_json::json;
use std::time::Duration;

const HANG_GUARD: Duration = Duration::from_secs(5);
/// Polls, die eine bereite Task auf dem Current-Thread-Runtime sicher erreichen.
const READY_POLLS: usize = 64;

async fn parked_user_task() -> (
    Arc<WorkflowEngine>,
    Arc<InstrumentedPersistence>,
    Uuid,
    Uuid,
) {
    let persistence = Arc::new(InstrumentedPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
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
    (engine, persistence, key, id)
}

async fn rekey(engine: &WorkflowEngine, from: Uuid, to: Uuid) {
    let arc = engine
        .instances
        .remove(&from)
        .await
        .expect("instance present");
    let mut inst = arc.read().await.clone();
    inst.id = to;
    engine.instances.insert(to, inst).await;
}

fn x_values(commits: &[HashMap<String, Value>]) -> Vec<Value> {
    commits
        .iter()
        .filter_map(|vars| vars.get("x").cloned())
        .collect()
}

async fn set_x(engine: &WorkflowEngine, id: Uuid, value: i64) -> EngineResult<()> {
    engine
        .update_instance_variables(id, HashMap::from([("x".to_string(), json!(value))]))
        .await
}

/// Solange eine Speicherung im Gate hängt, darf kein weiterer `save_instance`
/// derselben (oder überhaupt, je nach Test) ankommen.
async fn assert_no_arrival(arrivals: &mut tokio::sync::mpsc::Receiver<SaveArrival>) {
    for _ in 0..READY_POLLS {
        tokio::task::yield_now().await;
        match arrivals.try_recv() {
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {}
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                panic!("save gate closed");
            }
            Ok(arrival) => panic!(
                "save of {} entered while another save was still in flight",
                arrival.instance_id
            ),
        }
    }
}

async fn recv_arrival(arrivals: &mut tokio::sync::mpsc::Receiver<SaveArrival>) -> SaveArrival {
    tokio::time::timeout(HANG_GUARD, arrivals.recv())
        .await
        .expect("save did not reach the backend")
        .expect("save gate open")
}

#[test]
fn stripe_index_is_stable_and_bounded() {
    let id = Uuid::from_u128(1);
    assert_eq!(
        PersistLocks::stripe_index(id),
        PersistLocks::stripe_index(id)
    );
    assert!(PersistLocks::stripe_index(id) < 256);
    assert_ne!(
        PersistLocks::stripe_index(Uuid::from_u128(1)),
        PersistLocks::stripe_index(Uuid::from_u128(2))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn instance_stays_readable_while_save_is_in_flight() {
    let (engine, persistence, _key, id) = parked_user_task().await;
    let mut arrivals = persistence.arm_save_gate().await;

    let engine_for_write = Arc::clone(&engine);
    let writer = tokio::spawn(async move { set_x(&engine_for_write, id, 1).await });
    let arrival = recv_arrival(&mut arrivals).await;
    assert_eq!(arrival.variables.get("x"), Some(&json!(1)));

    let engine_for_read = Arc::clone(&engine);
    let read = tokio::spawn(async move { engine_for_read.get_instance_details(id).await });
    for _ in 0..READY_POLLS {
        if read.is_finished() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        read.is_finished(),
        "in-flight save still held the instance lock"
    );
    let details = read.await.unwrap().unwrap();
    assert_eq!(details.variables.get("x"), Some(&json!(1)));

    arrival.release_and_wait().await;
    writer.await.unwrap().unwrap();
    engine.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn concurrent_saves_commit_older_snapshot_first() {
    let (engine, persistence, _key, id) = parked_user_task().await;
    let mut arrivals = persistence.arm_save_gate().await;

    let engine_for_first = Arc::clone(&engine);
    let first = tokio::spawn(async move { set_x(&engine_for_first, id, 1).await });
    let older = recv_arrival(&mut arrivals).await;
    assert_eq!(older.variables.get("x"), Some(&json!(1)));

    let engine_for_second = Arc::clone(&engine);
    let second = tokio::spawn(async move { set_x(&engine_for_second, id, 2).await });
    tokio::time::timeout(HANG_GUARD, async {
        loop {
            let live = engine.get_instance_details(id).await.unwrap();
            if live.variables.get("x") == Some(&json!(2)) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("newer variable update did not become visible during the in-flight save");

    assert_no_arrival(&mut arrivals).await;

    older.release_and_wait().await;
    let newer = recv_arrival(&mut arrivals).await;
    assert_eq!(newer.variables.get("x"), Some(&json!(2)));
    newer.release_and_wait().await;

    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();

    assert_eq!(
        x_values(&persistence.committed_variables().await),
        vec![json!(1), json!(2)]
    );
    let stored = persistence
        .inner
        .list_instances()
        .await
        .unwrap()
        .into_iter()
        .find(|instance| instance.id == id)
        .expect("instance persisted");
    assert_eq!(stored.variables.get("x"), Some(&json!(2)));
    engine.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn retry_save_waits_for_inflight_snapshot() {
    let (engine, persistence, _key, id) = parked_user_task().await;
    let mut arrivals = persistence.arm_save_gate().await;

    let engine_for_write = Arc::clone(&engine);
    let writer = tokio::spawn(async move { set_x(&engine_for_write, id, 1).await });
    let older = recv_arrival(&mut arrivals).await;
    assert_eq!(older.variables.get("x"), Some(&json!(1)));

    {
        let arc = engine.instances.get(&id).await.unwrap();
        let mut inst = arc.write().await;
        inst.variables.insert("x".to_string(), json!(2));
    }
    engine
        .retry_tx
        .as_ref()
        .unwrap()
        .try_send(PersistJob::SaveInstance(id))
        .unwrap();

    assert_no_arrival(&mut arrivals).await;

    older.release_and_wait().await;
    let retried = recv_arrival(&mut arrivals).await;
    assert_eq!(retried.instance_id, id);
    assert_eq!(
        retried.variables.get("x"),
        Some(&json!(2)),
        "retry must re-read after the in-flight snapshot commits"
    );
    retried.release_and_wait().await;
    writer.await.unwrap().unwrap();

    assert_eq!(
        x_values(&persistence.committed_variables().await),
        vec![json!(1), json!(2)]
    );
    engine.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn distinct_stripes_enter_save_concurrently() {
    let (engine, persistence, key, first_id) = parked_user_task().await;
    let second_id = engine.start_instance(key).await.unwrap();
    let id_a = Uuid::from_u128(1);
    let id_b = Uuid::from_u128(2);
    assert_ne!(
        PersistLocks::stripe_index(id_a),
        PersistLocks::stripe_index(id_b)
    );
    rekey(&engine, first_id, id_a).await;
    rekey(&engine, second_id, id_b).await;

    let mut arrivals = persistence.arm_save_gate().await;
    let engine_a = Arc::clone(&engine);
    let engine_b = Arc::clone(&engine);
    let task_a = tokio::spawn(async move { set_x(&engine_a, id_a, 1).await });
    let task_b = tokio::spawn(async move { set_x(&engine_b, id_b, 2).await });

    let left = recv_arrival(&mut arrivals).await;
    let right = recv_arrival(&mut arrivals).await;
    let mut ids = [left.instance_id, right.instance_id];
    ids.sort();
    assert_eq!(ids, [id_a, id_b]);

    left.release_and_wait().await;
    right.release_and_wait().await;
    task_a.await.unwrap().unwrap();
    task_b.await.unwrap().unwrap();
    engine.shutdown().await;
}
