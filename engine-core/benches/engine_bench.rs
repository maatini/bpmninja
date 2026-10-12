use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use engine_core::engine::WorkflowEngine;
use engine_core::model::{BpmnElement, ProcessDefinitionBuilder};
use serde_json::json;

fn linear_definition(key: &str, node_count: usize) -> engine_core::model::ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new(key).node("start", BpmnElement::StartEvent);
    builder = builder.flow("start", "t_0");

    for i in 0..node_count {
        let node_id = format!("t_{}", i);
        builder = builder.node(
            &node_id,
            BpmnElement::ServiceTask {
                topic: "topic_1".into(),
                multi_instance: None,
            },
        );

        if i < node_count - 1 {
            builder = builder.flow(&node_id, format!("t_{}", i + 1));
        }
    }

    builder = builder
        .node("end", BpmnElement::EndEvent)
        .flow(format!("t_{}", node_count - 1), "end");

    builder.build().unwrap()
}

/// Completes all currently pending service tasks by fetching and executing them repeatedly until none are left.
async fn drain_service_tasks(engine: &WorkflowEngine, worker: &str) {
    loop {
        let topics: Vec<String> = engine
            .get_pending_service_tasks()
            .iter()
            .map(|t| t.topic.clone())
            .collect();
        if topics.is_empty() {
            break;
        }

        let tasks = engine
            .fetch_and_lock_service_tasks(worker, 1000, &topics, 30_000)
            .await;
        if tasks.is_empty() {
            break;
        }
        for task in tasks {
            engine
                .complete_service_task(task.id, worker, std::collections::HashMap::new())
                .await
                .unwrap();
        }
    }
}

pub fn bench_execution_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("Engine Throughput");

    for size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("Linear Process Execution", size),
            size,
            |b, &s| {
                b.to_async(&rt).iter(|| async {
                    let engine = WorkflowEngine::new();
                    let def = linear_definition("bench_linear", s);
                    let (def_key, _) = engine.deploy_definition(def).await;

                    let inst_id = engine.start_instance(def_key).await.unwrap();
                    drain_service_tasks(&engine, "bench_worker").await;

                    assert_eq!(
                        engine.get_instance_state(inst_id).await.unwrap(),
                        engine_core::InstanceState::Completed
                    );
                });
            },
        );
    }

    group.finish();
}

/// start → fork → N script branches → join → end (no wait states).
fn parallel_definition(branches: usize) -> engine_core::model::ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new("bench_parallel")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("join", "end");
    for i in 0..branches {
        let id = format!("b_{i}");
        builder = builder
            .node(
                &id,
                BpmnElement::ScriptTask {
                    script: format!("let r{i} = {i} * 2;"),
                    multi_instance: None,
                },
            )
            .flow("fork", &id)
            .flow(&id, "join");
    }
    builder.build().unwrap()
}

/// start → N chained script tasks → end.
fn script_chain_definition(len: usize) -> engine_core::model::ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new("bench_scripts")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "s_0");
    for i in 0..len {
        let id = format!("s_{i}");
        builder = builder.node(
            &id,
            BpmnElement::ScriptTask {
                script: "counter = counter + 1;".into(),
                multi_instance: None,
            },
        );
        let next = if i + 1 < len {
            format!("s_{}", i + 1)
        } else {
            "end".to_string()
        };
        builder = builder.flow(&id, next);
    }
    builder.build().unwrap()
}

/// start → XOR (3 conditional branches + default) → end.
fn xor_definition() -> engine_core::model::ProcessDefinition {
    ProcessDefinitionBuilder::new("bench_xor")
        .node("start", BpmnElement::StartEvent)
        .node(
            "xor",
            BpmnElement::ExclusiveGateway {
                default: Some("low".into()),
            },
        )
        .node("high", BpmnElement::EndEvent)
        .node("mid", BpmnElement::EndEvent)
        .node("low", BpmnElement::EndEvent)
        .flow("start", "xor")
        .conditional_flow("xor", "high", "amount > 1000")
        .conditional_flow("xor", "mid", "amount > 100")
        .flow("xor", "low")
        .build()
        .unwrap()
}

pub fn bench_gateways_and_scripts(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut group = c.benchmark_group("Gateways & Scripts");

    for branches in [4usize, 16] {
        let engine = WorkflowEngine::new();
        let def_key = rt.block_on(engine.deploy_definition(parallel_definition(branches))).0;
        group.bench_with_input(
            BenchmarkId::new("Parallel fork/join (script branches)", branches),
            &branches,
            |b, _| {
                b.to_async(&rt)
                    .iter(|| async { engine.start_instance(def_key).await.unwrap() });
            },
        );
    }

    let engine = WorkflowEngine::new();
    let def_key = rt.block_on(engine.deploy_definition(script_chain_definition(10))).0;
    group.bench_function("Script chain (10 tasks)", |b| {
        b.to_async(&rt).iter(|| async {
            let vars = std::collections::HashMap::from([("counter".to_string(), json!(0))]);
            engine
                .start_instance_with_variables(def_key, vars)
                .await
                .unwrap()
        });
    });

    let engine = WorkflowEngine::new();
    let def_key = rt.block_on(engine.deploy_definition(xor_definition())).0;
    group.bench_function("XOR routing", |b| {
        b.to_async(&rt).iter(|| async {
            let vars = std::collections::HashMap::from([("amount".to_string(), json!(500))]);
            engine
                .start_instance_with_variables(def_key, vars)
                .await
                .unwrap()
        });
    });

    group.finish();
}

criterion_group!(benches, bench_execution_throughput, bench_gateways_and_scripts);
criterion_main!(benches);
