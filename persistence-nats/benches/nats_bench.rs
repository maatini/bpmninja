//! NATS JetStream persistence benchmarks (raw adapter ops + engine hot path).
//!
//! Requires a running NATS with JetStream (`NATS_URL`, default `nats://localhost:4222`).
//! Skips silently when NATS is unreachable.

use std::collections::HashMap;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use engine_core::history::{ActorType, HistoryEntry, HistoryEventType};
use engine_core::model::{BpmnElement, ProcessDefinitionBuilder, Token};
use engine_core::persistence::WorkflowPersistence;
use engine_core::{ProcessInstance, WorkflowEngine};
use persistence_nats::NatsPersistence;
use serde_json::json;
use uuid::Uuid;

fn nats_url() -> String {
    std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string())
}

/// A realistic waiting instance (parked at a user task) built via the in-memory engine.
async fn sample_instance() -> ProcessInstance {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("bench_wait")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("bench".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let vars = HashMap::from([
        ("customer".to_string(), json!("ACME Corp")),
        ("amount".to_string(), json!(4711)),
        ("items".to_string(), json!(["a", "b", "c"])),
    ]);
    let id = engine
        .start_instance_with_variables(key, vars)
        .await
        .unwrap();
    engine.get_instance_details(id).await.unwrap()
}

/// start → N chained script tasks → end (every step persists history + instance).
fn script_chain(len: usize) -> engine_core::model::ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new("bench_nats_scripts")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "s_0");
    for i in 0..len {
        let id = format!("s_{i}");
        let next = if i + 1 < len {
            format!("s_{}", i + 1)
        } else {
            "end".to_string()
        };
        builder = builder
            .node(
                &id,
                BpmnElement::ScriptTask {
                    script: "counter = counter + 1;".into(),
                    multi_instance: None,
                },
            )
            .flow(&id, next);
    }
    builder.build().unwrap()
}

fn bench_nats(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let stream = format!("BENCH_STREAM_{}", Uuid::new_v4().simple());
    let nats = match rt.block_on(NatsPersistence::connect(&nats_url(), &stream)) {
        Ok(p) => Arc::new(p),
        Err(e) => {
            eprintln!("Skipping NATS benchmarks — NATS not reachable: {e}");
            return;
        }
    };

    let mut group = c.benchmark_group("NATS persistence");

    let instance = rt.block_on(sample_instance());
    group.bench_function("save_instance", |b| {
        b.to_async(&rt)
            .iter(|| async { nats.save_instance(&instance).await.unwrap() });
    });

    let token = Token::new("task");
    let instance_id = instance.id;
    group.bench_function("save_token", |b| {
        b.to_async(&rt)
            .iter(|| async { nats.save_token(instance_id, &token).await.unwrap() });
    });

    let entry = HistoryEntry::new(
        instance_id,
        HistoryEventType::TokenAdvanced,
        "bench",
        ActorType::Engine,
        None,
    );
    group.bench_function("append_history_entry", |b| {
        b.to_async(&rt)
            .iter(|| async { nats.append_history_entry(&entry).await.unwrap() });
    });

    // Engine hot path with NATS persistence attached.
    let engine = {
        let _guard = rt.enter();
        WorkflowEngine::new().with_persistence(nats.clone() as Arc<dyn WorkflowPersistence>)
    };
    let def_key = rt.block_on(engine.deploy_definition(script_chain(5))).0;
    group.bench_function("engine: start → 5 script tasks → end", |b| {
        b.to_async(&rt).iter(|| async {
            let vars = HashMap::from([("counter".to_string(), json!(0))]);
            engine
                .start_instance_with_variables(def_key, vars)
                .await
                .unwrap()
        });
    });

    group.finish();
    rt.block_on(engine.shutdown());
}

criterion_group!(benches, bench_nats);
criterion_main!(benches);
