//! Library-facing integration tests for in-process service-task handlers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use engine_core::adapter::InMemoryPersistence;
use engine_core::{
    BpmnElement, InstanceState, PendingServiceTask, ProcessDefinition, ProcessDefinitionBuilder,
    TaskHandler, TaskOutcome, WorkflowEngine, WorkflowPersistence,
};
use serde_json::Value;
use uuid::Uuid;

struct FnHandler<F>(F);

#[async_trait::async_trait]
impl<F> TaskHandler for FnHandler<F>
where
    F: Fn(&PendingServiceTask) -> TaskOutcome + Send + Sync,
{
    async fn handle(&self, task: &PendingServiceTask) -> TaskOutcome {
        (self.0)(task)
    }
}

fn handler<F>(function: F) -> Arc<dyn TaskHandler>
where
    F: Fn(&PendingServiceTask) -> TaskOutcome + Send + Sync + 'static,
{
    Arc::new(FnHandler(function))
}

fn definition(topic: &str, error_boundary: bool) -> ProcessDefinition {
    definition_with_key(topic, error_boundary, None)
}

fn definition_with_key(topic: &str, error_boundary: bool, key: Option<Uuid>) -> ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new(format!("embed_{topic}"))
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: topic.into(),
                multi_instance: None,
            },
        );
    if error_boundary {
        builder = builder.node(
            "bound_err",
            BpmnElement::BoundaryErrorEvent {
                attached_to: "svc".into(),
                error_code: Some("CALC_FAILED".into()),
            },
        );
    }
    builder = builder
        .node("end", BpmnElement::EndEvent)
        .node("error_end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end");
    if error_boundary {
        builder = builder.flow("bound_err", "error_end");
    }
    if let Some(key) = key {
        builder = builder.with_key(key);
    }
    builder.build().unwrap()
}

async fn wait_completed(
    engine: &WorkflowEngine,
    instance_id: Uuid,
) -> engine_core::ProcessInstance {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let instance = engine.get_instance_details(instance_id).await.unwrap();
        if instance.state == InstanceState::Completed {
            return instance;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "instance stayed in {:?}",
            instance.state
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn handler_completes_service_task_with_variables() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "calc",
        handler(|task| {
            let input = task
                .variables_snapshot
                .get("input")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            TaskOutcome::complete([("result", Value::from(input * 2))])
        }),
    );
    engine.spawn_task_handler_dispatcher().await;

    let (key, _) = engine.deploy_definition(definition("calc", false)).await;
    let instance_id = engine
        .start_instance_with_variables(key, HashMap::from([("input".into(), Value::from(21))]))
        .await
        .unwrap();

    let instance = wait_completed(&engine, instance_id).await;
    assert_eq!(instance.current_node, "end");
    assert_eq!(instance.variables.get("result"), Some(&Value::from(42)));
    assert!(engine.get_pending_service_tasks().is_empty());
    engine.shutdown().await;
}

#[tokio::test]
async fn bpmn_error_is_routed_to_the_boundary() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "calc",
        handler(|_| {
            TaskOutcome::bpmn_error(
                "CALC_FAILED",
                Some("division by zero".into()),
                [("failed", Value::from(true))],
            )
        }),
    );
    engine.spawn_task_handler_dispatcher().await;

    let (key, _) = engine.deploy_definition(definition("calc", true)).await;
    let instance_id = engine.start_instance(key).await.unwrap();
    let instance = wait_completed(&engine, instance_id).await;

    assert_eq!(instance.current_node, "error_end");
    assert_eq!(instance.variables.get("failed"), Some(&Value::from(true)));
    assert!(
        instance
            .audit_log
            .iter()
            .any(|line| line.contains("CALC_FAILED"))
    );
    engine.shutdown().await;
}

#[tokio::test]
async fn external_worker_keeps_topics_without_a_handler() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler("calc", handler(|_| TaskOutcome::success()));
    engine.spawn_task_handler_dispatcher().await;

    let (calc_key, _) = engine.deploy_definition(definition("calc", false)).await;
    let (manual_key, _) = engine.deploy_definition(definition("manual", false)).await;
    let calc_id = engine.start_instance(calc_key).await.unwrap();
    let manual_id = engine.start_instance(manual_key).await.unwrap();

    let locked = engine
        .fetch_and_lock_service_tasks("external", 10, &["calc".into(), "manual".into()], 60)
        .await;
    assert_eq!(locked.len(), 1);
    assert_eq!(locked[0].topic, "manual");
    assert_eq!(locked[0].instance_id, manual_id);

    let calc = wait_completed(&engine, calc_id).await;
    assert_eq!(calc.state, InstanceState::Completed);

    engine
        .complete_service_task(
            locked[0].id,
            "external",
            HashMap::from([("manual".to_string(), Value::from(true))]),
        )
        .await
        .unwrap();
    let manual = wait_completed(&engine, manual_id).await;
    assert_eq!(manual.variables.get("manual"), Some(&Value::from(true)));
    engine.shutdown().await;
}

#[tokio::test]
async fn restored_pending_task_is_handled_after_restart() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (key, _) = engine.deploy_definition(definition("calc", false)).await;
    let instance_id = engine.start_instance(key).await.unwrap();
    assert_eq!(persistence.list_service_tasks().await.unwrap().len(), 1);
    engine.shutdown().await;

    let restarted = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    restarted
        .deploy_definition(definition_with_key("calc", false, Some(key)))
        .await;
    for instance in persistence.list_instances().await.unwrap() {
        restarted.restore_instance(instance).await;
    }
    for task in persistence.list_service_tasks().await.unwrap() {
        restarted.restore_service_task(task);
    }
    restarted.register_task_handler(
        "calc",
        handler(|_| TaskOutcome::complete([("done", Value::from(true))])),
    );
    restarted.spawn_task_handler_dispatcher().await;

    let instance = wait_completed(&restarted, instance_id).await;
    assert_eq!(instance.variables.get("done"), Some(&Value::from(true)));
    assert!(persistence.list_service_tasks().await.unwrap().is_empty());
    restarted.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_instances_complete_without_double_execution() {
    let engine = Arc::new(WorkflowEngine::new());
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = Arc::clone(&calls);
    engine.register_task_handler(
        "calc",
        handler(move |_| {
            seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            TaskOutcome::success()
        }),
    );
    engine.spawn_task_handler_dispatcher().await;

    let (key, _) = engine.deploy_definition(definition("calc", false)).await;
    let mut ids = Vec::new();
    for _ in 0..12 {
        ids.push(engine.start_instance(key).await.unwrap());
    }

    let mut external_hits = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        external_hits += engine
            .fetch_and_lock_service_tasks("external", 12, &["calc".into()], 30)
            .await
            .len();
        let mut done = 0;
        for id in &ids {
            if engine.get_instance_details(*id).await.unwrap().state == InstanceState::Completed {
                done += 1;
            }
        }
        if done == ids.len() {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "not every instance completed"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    for id in &ids {
        let instance = wait_completed(&engine, *id).await;
        assert_eq!(instance.state, InstanceState::Completed);
    }
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), ids.len());
    assert_eq!(external_hits, 0);
    engine.shutdown().await;
}
