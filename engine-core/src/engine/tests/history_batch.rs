//! P2: history entries of one `run_instance_batch` are written in a single
//! backend call — also when the batch fails part-way.

use super::super::*;
use super::helpers::InstrumentedPersistence;
use crate::domain::ProcessDefinitionBuilder;
use crate::history::HistoryEntry;
use crate::port::HistoryQuery;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// start → s0 → s1 → s2 → end; `failing_script` replaces s1's script.
fn script_chain(failing_script: Option<&str>) -> ProcessDefinition {
    let script = |i: usize| match (i, failing_script) {
        (1, Some(f)) => f.to_string(),
        _ => format!("counter = counter + 1; let step{i} = true;"),
    };
    let mut builder = ProcessDefinitionBuilder::new("history_batch")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "s0")
        .flow("s2", "end");
    for i in 0..3 {
        builder = builder.node(
            format!("s{i}"),
            BpmnElement::ScriptTask {
                script: script(i),
                multi_instance: None,
            },
        );
    }
    builder.flow("s0", "s1").flow("s1", "s2").build().unwrap()
}

async fn engine_with(
    def: ProcessDefinition,
) -> (WorkflowEngine, Arc<InstrumentedPersistence>, Uuid) {
    let persistence = Arc::new(InstrumentedPersistence::new(
        Duration::ZERO,
        Duration::ZERO,
    ));
    let engine =
        WorkflowEngine::new().with_persistence(persistence.clone() as Arc<dyn WorkflowPersistence>);
    let (key, _) = engine.deploy_definition(def).await;
    (engine, persistence, key)
}

async fn history_of(persistence: &InstrumentedPersistence, id: Uuid) -> Vec<HistoryEntry> {
    persistence
        .inner
        .query_history(HistoryQuery {
            instance_id: id,
            ..Default::default()
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn batch_history_is_written_in_one_call() {
    let (engine, persistence, key) = engine_with(script_chain(None)).await;
    let vars = HashMap::from([("counter".to_string(), json!(0))]);

    let id = engine.start_instance_with_variables(key, vars).await.unwrap();

    assert_eq!(
        persistence.history_batches.load(Ordering::SeqCst),
        1,
        "one start → end run must flush history exactly once"
    );
    let steps = history_of(&persistence, id)
        .await
        .into_iter()
        .filter(|e| e.actor_type == crate::history::ActorType::Engine)
        .count();
    assert!(steps >= 3, "expected an entry per script step, got {steps}");
    engine.shutdown().await;
}

#[tokio::test]
async fn history_is_flushed_when_the_batch_fails() {
    let (engine, persistence, key) =
        engine_with(script_chain(Some("throw \"boom\";"))).await;
    let vars = HashMap::from([("counter".to_string(), json!(0))]);

    let result = engine.start_instance_with_variables(key, vars).await;
    assert!(result.is_err(), "s1 must fail the batch");

    let id = engine
        .list_instances()
        .await
        .into_iter()
        .find(|i| i.definition_key == key)
        .expect("instance exists")
        .id;
    let history = history_of(&persistence, id).await;
    assert_eq!(persistence.history_batches.load(Ordering::SeqCst), 1);
    assert!(
        history
            .iter()
            .any(|e| e.actor_type == crate::history::ActorType::Engine),
        "steps before the failure must not be lost"
    );
    engine.shutdown().await;
}

#[tokio::test]
async fn no_history_work_without_persistence() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(script_chain(None)).await;
    let vars = HashMap::from([("counter".to_string(), json!(0))]);
    let id = engine.start_instance_with_variables(key, vars).await.unwrap();
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}
