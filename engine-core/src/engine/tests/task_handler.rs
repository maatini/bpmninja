//! In-process task handlers (E1): dispatcher reuses fetch-and-lock semantics.

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::ProcessDefinitionBuilder;
use crate::engine::{TaskHandler, TaskOutcome};

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

fn handler<F>(f: F) -> Arc<dyn TaskHandler>
where
    F: Fn(&PendingServiceTask) -> TaskOutcome + Send + Sync + 'static,
{
    Arc::new(FnHandler(f))
}

fn service_definition(key: Option<Uuid>) -> ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new("in_process");
    if let Some(key) = key {
        builder = builder.with_key(key);
    }
    builder
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "calc".into(),
                multi_instance: None,
            },
        )
        .node(
            "bound_err",
            BpmnElement::BoundaryErrorEvent {
                attached_to: "svc".into(),
                error_code: Some("CALC_FAILED".into()),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .node("error_end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .flow("bound_err", "error_end")
        .build()
        .unwrap()
}

async fn wait_until<F, Fut>(mut condition: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !condition().await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "condition not reached in time"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn wait_for_completion(engine: &WorkflowEngine, instance_id: Uuid) -> ProcessInstance {
    wait_until(|| async {
        matches!(
            engine.get_instance_details(instance_id).await.unwrap().state,
            InstanceState::Completed
        )
    })
    .await;
    engine.get_instance_details(instance_id).await.unwrap()
}

#[tokio::test]
async fn handler_completes_task_and_merges_variables() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "calc",
        handler(|task| {
            let input = task
                .variables_snapshot
                .get("input")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            TaskOutcome::Complete(HashMap::from([("result".into(), (input * 2).into())]))
        }),
    );
    let dispatcher = engine.spawn_task_handler_dispatcher();

    let (def_key, _) = engine.deploy_definition(service_definition(None)).await;
    let instance_id = engine
        .start_instance_with_variables(def_key, HashMap::from([("input".into(), 21.into())]))
        .await
        .unwrap();

    let inst = wait_for_completion(&engine, instance_id).await;
    assert_eq!(inst.variables.get("result"), Some(&Value::from(42)));
    assert!(engine.get_pending_service_tasks().is_empty());
    dispatcher.abort();
}

#[tokio::test]
async fn handler_bpmn_error_follows_error_boundary() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "calc",
        handler(|_| TaskOutcome::BpmnError {
            code: "CALC_FAILED".into(),
            message: Some("division by zero".into()),
            variables: HashMap::from([("failed".into(), true.into())]),
        }),
    );
    let dispatcher = engine.spawn_task_handler_dispatcher();

    let (def_key, _) = engine.deploy_definition(service_definition(None)).await;
    let instance_id = engine.start_instance(def_key).await.unwrap();

    let inst = wait_for_completion(&engine, instance_id).await;
    assert_eq!(inst.current_node, "error_end");
    assert_eq!(inst.variables.get("failed"), Some(&Value::from(true)));
    dispatcher.abort();
}

#[tokio::test]
async fn handler_failure_without_retries_creates_incident() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "calc",
        handler(|_| TaskOutcome::Fail {
            message: "backend down".into(),
            details: None,
            retries: Some(0),
            retry_timeout_ms: 0,
        }),
    );
    let dispatcher = engine.spawn_task_handler_dispatcher();

    let (def_key, _) = engine.deploy_definition(service_definition(None)).await;
    engine.start_instance(def_key).await.unwrap();

    wait_until(|| async {
        engine
            .get_pending_service_tasks()
            .first()
            .is_some_and(|t| t.retries <= 0)
    })
    .await;
    let task = &engine.get_pending_service_tasks()[0];
    assert_eq!(task.error_message.as_deref(), Some("backend down"));
    assert!(task.worker_id.is_none(), "incident must not stay locked");
    dispatcher.abort();
}

#[tokio::test]
async fn panicking_handler_is_reported_as_failure() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler("calc", handler(|_| panic!("handler bug")));
    let dispatcher = engine.spawn_task_handler_dispatcher();

    let (def_key, _) = engine.deploy_definition(service_definition(None)).await;
    engine.start_instance(def_key).await.unwrap();

    // Each panic consumes one retry until the task becomes an incident.
    wait_until(|| async {
        engine
            .get_pending_service_tasks()
            .first()
            .is_some_and(|t| t.retries <= 0)
    })
    .await;
    let task = &engine.get_pending_service_tasks()[0];
    assert_eq!(
        task.error_message.as_deref(),
        Some("In-process task handler panicked")
    );
    dispatcher.abort();
}

#[tokio::test]
async fn topics_without_handler_stay_available_to_external_workers() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "other_topic",
        handler(|_| TaskOutcome::Complete(HashMap::new())),
    );
    let dispatcher = engine.spawn_task_handler_dispatcher();

    let (def_key, _) = engine.deploy_definition(service_definition(None)).await;
    engine.start_instance(def_key).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let tasks = engine
        .fetch_and_lock_service_tasks("external", 1, &["calc".into()], 60)
        .await;
    assert_eq!(tasks.len(), 1);
    dispatcher.abort();
}

#[tokio::test]
async fn unregistered_handler_no_longer_receives_tasks() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler("calc", handler(|_| TaskOutcome::Complete(HashMap::new())));
    assert!(engine.unregister_task_handler("calc"));
    assert!(!engine.unregister_task_handler("calc"));
    let dispatcher = engine.spawn_task_handler_dispatcher();

    let (def_key, _) = engine.deploy_definition(service_definition(None)).await;
    engine.start_instance(def_key).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let pending = engine.get_pending_service_tasks();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].worker_id.is_none());
    dispatcher.abort();
}

#[tokio::test]
async fn restored_pending_task_is_processed_after_restart() {
    let pers = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(pers.clone());
    let (def_key, _) = engine.deploy_definition(service_definition(None)).await;
    let instance_id = engine.start_instance(def_key).await.unwrap();
    assert_eq!(pers.list_service_tasks().await.unwrap().len(), 1);
    drop(engine);

    let engine2 = Arc::new(WorkflowEngine::new().with_persistence(pers.clone()));
    engine2
        .deploy_definition(service_definition(Some(def_key)))
        .await;
    for instance in pers.list_instances().await.unwrap() {
        engine2.restore_instance(instance).await;
    }
    for task in pers.list_service_tasks().await.unwrap() {
        engine2.restore_service_task(task);
    }

    engine2.register_task_handler(
        "calc",
        handler(|_| TaskOutcome::Complete(HashMap::from([("done".into(), true.into())]))),
    );
    let dispatcher = engine2.spawn_task_handler_dispatcher();

    let inst = wait_for_completion(&engine2, instance_id).await;
    assert_eq!(inst.variables.get("done"), Some(&Value::from(true)));
    assert!(pers.list_service_tasks().await.unwrap().is_empty());
    dispatcher.abort();
}
