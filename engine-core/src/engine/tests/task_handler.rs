//! In-process service-task handlers: registration, locking, retries, shutdown.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;

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

struct AsyncHandler<F>(F);

#[async_trait::async_trait]
impl<F, Fut> TaskHandler for AsyncHandler<F>
where
    F: Fn(PendingServiceTask) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = TaskOutcome> + Send,
{
    async fn handle(&self, task: &PendingServiceTask) -> TaskOutcome {
        (self.0)(task.clone()).await
    }
}

fn handler<F>(function: F) -> Arc<dyn TaskHandler>
where
    F: Fn(&PendingServiceTask) -> TaskOutcome + Send + Sync + 'static,
{
    Arc::new(FnHandler(function))
}

fn async_handler<F, Fut>(function: F) -> Arc<dyn TaskHandler>
where
    F: Fn(PendingServiceTask) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = TaskOutcome> + Send + 'static,
{
    Arc::new(AsyncHandler(function))
}

fn service_definition(topic: &str, error_boundary: bool) -> ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new(format!("in_process_{topic}"))
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
    builder.build().unwrap()
}

async fn completed(engine: &WorkflowEngine, instance_id: Uuid) -> ProcessInstance {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let state = engine
            .get_instance_details(instance_id)
            .await
            .unwrap()
            .state;
        if state == InstanceState::Completed {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "instance stayed in {state:?}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    engine.get_instance_details(instance_id).await.unwrap()
}

async fn wait_for_incident(engine: &WorkflowEngine) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if engine
            .get_pending_service_tasks()
            .first()
            .is_some_and(|task| task.retries <= 0)
        {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "incident was not opened in time"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[test]
fn register_unregister_and_reject_empty_topic() {
    let engine = WorkflowEngine::new();
    assert!(!engine.register_task_handler("", handler(|_| TaskOutcome::success())));
    assert!(!engine.register_task_handler("calc", handler(|_| TaskOutcome::success())));
    assert!(engine.register_task_handler("calc", handler(|_| TaskOutcome::success())));
    assert!(engine.unregister_task_handler("calc"));
    assert!(!engine.unregister_task_handler("calc"));
}

#[tokio::test]
async fn registered_topic_is_hidden_from_external_fetch() {
    let engine = WorkflowEngine::new();
    engine.register_task_handler("calc", handler(|_| TaskOutcome::success()));
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    engine.start_instance(key).await.unwrap();

    let stolen = engine
        .fetch_and_lock_service_tasks(IN_PROCESS_WORKER_ID, 10, &["calc".into()], 60)
        .await;
    assert!(
        stolen.is_empty(),
        "the reserved worker id must not bypass in-process precedence"
    );
    let external = engine
        .fetch_and_lock_service_tasks("external", 10, &["calc".into(), "other".into()], 60)
        .await;
    assert!(external.is_empty());

    assert!(engine.unregister_task_handler("calc"));
    let available = engine
        .fetch_and_lock_service_tasks("external", 10, &["calc".into()], 60)
        .await;
    assert_eq!(available.len(), 1);
    assert_eq!(available[0].worker_id.as_deref(), Some("external"));
}

#[tokio::test]
async fn retry_after_hides_task_until_deadline() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    engine.start_instance(key).await.unwrap();
    let task = engine
        .fetch_and_lock_service_tasks("worker", 1, &["calc".into()], 60)
        .await
        .remove(0);

    engine
        .fail_service_task_with_retry_timeout(
            task.id,
            "worker",
            Some(2),
            Some("busy".into()),
            None,
            Duration::from_millis(200),
        )
        .await
        .unwrap();

    let pending = engine.get_pending_service_tasks();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].worker_id.is_none());
    assert!(pending[0].lock_expiration.is_some());
    assert!(
        engine
            .fetch_and_lock_service_tasks("worker", 1, &["calc".into()], 60)
            .await
            .is_empty()
    );

    tokio::time::sleep(Duration::from_millis(250)).await;
    let again = engine
        .fetch_and_lock_service_tasks("worker", 1, &["calc".into()], 60)
        .await;
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].retries, 2);
}

#[tokio::test]
async fn handler_failure_opens_incident_and_retries_then_complete() {
    let engine = Arc::new(WorkflowEngine::new());
    let attempts = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&attempts);
    engine.register_task_handler(
        "calc",
        handler(move |_| {
            let attempt = seen.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                TaskOutcome::failure(
                    "backend down",
                    Some("timeout".into()),
                    Some(1),
                    Duration::from_millis(40),
                )
            } else {
                TaskOutcome::complete([("result", Value::from(7))])
            }
        }),
    );
    engine.spawn_task_handler_dispatcher().await;

    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    let instance_id = engine.start_instance(key).await.unwrap();
    let inst = completed(&engine, instance_id).await;
    assert_eq!(inst.variables.get("result"), Some(&Value::from(7)));
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(engine.get_pending_service_tasks().is_empty());
    engine.shutdown().await;
}

#[tokio::test]
async fn handler_failure_without_retries_creates_unlocked_incident() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "calc",
        handler(|_| TaskOutcome::failure("backend down", None, Some(0), Duration::from_secs(30))),
    );
    engine.spawn_task_handler_dispatcher().await;
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    let instance_id = engine.start_instance(key).await.unwrap();

    wait_for_incident(&engine).await;
    let task = &engine.get_pending_service_tasks()[0];
    assert_eq!(task.error_message.as_deref(), Some("backend down"));
    assert!(task.worker_id.is_none());
    assert!(task.lock_expiration.is_none());
    let inst = engine.get_instance_details(instance_id).await.unwrap();
    assert!(inst.audit_log.iter().any(|line| line.contains("INCIDENT")));
    engine.shutdown().await;
}

#[tokio::test]
async fn panicking_handler_is_reported_as_failure() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler("calc", handler(|_| panic!("handler bug")));
    engine.spawn_task_handler_dispatcher().await;
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    engine.start_instance(key).await.unwrap();

    wait_for_incident(&engine).await;
    let task = &engine.get_pending_service_tasks()[0];
    assert_eq!(
        task.error_message.as_deref(),
        Some("In-process task handler panicked")
    );
    assert!(
        task.error_details
            .as_deref()
            .is_some_and(|details| details.contains("handler bug"))
    );
    assert!(task.worker_id.is_none());
    engine.shutdown().await;
}

#[tokio::test]
async fn unhandled_bpmn_error_becomes_incident_with_variables() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler(
        "calc",
        handler(|_| {
            TaskOutcome::bpmn_error(
                "NO_BOUNDARY",
                Some("missing handler".into()),
                [("failed", Value::from(true))],
            )
        }),
    );
    engine.spawn_task_handler_dispatcher().await;
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    let instance_id = engine.start_instance(key).await.unwrap();

    wait_for_incident(&engine).await;
    let task = &engine.get_pending_service_tasks()[0];
    assert_eq!(
        task.error_message.as_deref(),
        Some("Unhandled BPMN error 'NO_BOUNDARY'")
    );
    assert_eq!(task.error_details.as_deref(), Some("missing handler"));
    let inst = engine.get_instance_details(instance_id).await.unwrap();
    assert_eq!(inst.variables.get("failed"), Some(&Value::from(true)));
    engine.shutdown().await;
}

#[tokio::test]
async fn external_topic_stays_available_while_dispatcher_runs() {
    let engine = Arc::new(WorkflowEngine::new());
    engine.register_task_handler("calc", handler(|_| TaskOutcome::success()));
    engine.spawn_task_handler_dispatcher().await;

    let (calc_key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    let (manual_key, _) = engine
        .deploy_definition(service_definition("manual", false))
        .await;
    let calc_id = engine.start_instance(calc_key).await.unwrap();
    engine.start_instance(manual_key).await.unwrap();

    let external = engine
        .fetch_and_lock_service_tasks("external", 10, &["calc".into(), "manual".into()], 60)
        .await;
    assert_eq!(external.len(), 1);
    assert_eq!(external[0].topic, "manual");

    let inst = completed(&engine, calc_id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    engine
        .complete_service_task(external[0].id, "external", HashMap::new())
        .await
        .unwrap();
    engine.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn handlers_run_concurrently() {
    let engine = Arc::new(WorkflowEngine::new());
    let barrier = Arc::new(tokio::sync::Barrier::new(4));
    let gate = Arc::clone(&barrier);
    engine.register_task_handler(
        "calc",
        async_handler(move |_| {
            let gate = Arc::clone(&gate);
            async move {
                gate.wait().await;
                TaskOutcome::success()
            }
        }),
    );
    engine.spawn_task_handler_dispatcher().await;
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    let mut ids = Vec::new();
    for _ in 0..4 {
        ids.push(engine.start_instance(key).await.unwrap());
    }
    for id in ids {
        let inst = completed(&engine, id).await;
        assert_eq!(inst.state, InstanceState::Completed);
    }
    engine.shutdown().await;
}

#[tokio::test]
async fn shutdown_waits_for_in_flight_handler_and_stops_dispatcher() {
    let engine = Arc::new(WorkflowEngine::new());
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Barrier::new(2));
    let entered_handler = Arc::clone(&entered);
    let release_handler = Arc::clone(&release);
    engine.register_task_handler(
        "calc",
        async_handler(move |_| {
            let entered_handler = Arc::clone(&entered_handler);
            let release_handler = Arc::clone(&release_handler);
            async move {
                entered_handler.notify_one();
                release_handler.wait().await;
                TaskOutcome::complete([("finished", Value::from(true))])
            }
        }),
    );
    engine.spawn_task_handler_dispatcher().await;
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    let instance_id = engine.start_instance(key).await.unwrap();

    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .expect("handler entered");

    let shutting_down = Arc::clone(&engine);
    let shutdown = tokio::spawn(async move { shutting_down.shutdown().await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !shutdown.is_finished(),
        "shutdown must wait for the in-flight handler"
    );

    let second_id = engine.start_instance(key).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let pending = engine.get_pending_service_tasks();
    assert!(
        pending
            .iter()
            .any(|task| task.instance_id == second_id && task.worker_id.is_none()),
        "dispatcher must not lock tasks after shutdown was requested"
    );

    release.wait().await;
    tokio::time::timeout(Duration::from_secs(2), shutdown)
        .await
        .expect("shutdown finished")
        .expect("shutdown task");

    let first = engine.get_instance_details(instance_id).await.unwrap();
    assert_eq!(first.state, InstanceState::Completed);
    assert_eq!(first.variables.get("finished"), Some(&Value::from(true)));

    tokio::time::sleep(Duration::from_millis(80)).await;
    let second = engine.get_instance_details(second_id).await.unwrap();
    assert_ne!(second.state, InstanceState::Completed);
    assert!(
        engine
            .get_pending_service_tasks()
            .iter()
            .any(|task| task.instance_id == second_id && task.worker_id.is_none())
    );
}

#[tokio::test]
async fn replaced_handler_receives_new_tasks() {
    let engine = Arc::new(WorkflowEngine::new());
    let which = Arc::new(AtomicUsize::new(0));
    let first = Arc::clone(&which);
    let second = Arc::clone(&which);
    engine.register_task_handler(
        "calc",
        handler(move |_| {
            first.store(1, Ordering::SeqCst);
            TaskOutcome::success()
        }),
    );
    assert!(engine.register_task_handler(
        "calc",
        handler(move |_| {
            second.store(2, Ordering::SeqCst);
            TaskOutcome::success()
        }),
    ));
    engine.spawn_task_handler_dispatcher().await;
    let (key, _) = engine
        .deploy_definition(service_definition("calc", false))
        .await;
    let instance_id = engine.start_instance(key).await.unwrap();
    completed(&engine, instance_id).await;
    assert_eq!(which.load(Ordering::SeqCst), 2);
    engine.shutdown().await;
}
