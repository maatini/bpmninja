//! Service-task incident tests (E5 fetch-and-lock skip, E6 unhandled bpmnError).

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn fetch_and_lock_skips_incidents() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("inc_skip")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "inc_topic".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    engine.start_instance(def_key).await.unwrap();

    let tasks = engine
        .fetch_and_lock_service_tasks("worker", 1, &["inc_topic".into()], 60)
        .await;
    assert_eq!(tasks.len(), 1);
    let task_id = tasks[0].id;

    engine
        .fail_service_task(task_id, "worker", Some(0), Some("Fatal".into()), None)
        .await
        .unwrap();

    let locked = engine
        .fetch_and_lock_service_tasks("worker2", 10, &["inc_topic".into()], 60)
        .await;
    assert!(locked.is_empty());

    let pending = engine.get_pending_service_tasks();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, task_id);
    assert!(pending[0].retries <= 0);
}

#[tokio::test]
async fn unhandled_bpmn_error_becomes_incident() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("err_inc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "err".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    let tasks = engine
        .fetch_and_lock_service_tasks("worker", 1, &["err".into()], 60)
        .await;
    assert_eq!(tasks.len(), 1);

    engine
        .handle_bpmn_error(tasks[0].id, "worker", "ERR_CODE")
        .await
        .unwrap();

    let pending = engine.get_pending_service_tasks();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].retries <= 0);
    assert!(pending[0].worker_id.is_none());
    assert!(pending[0].lock_expiration.is_none());

    let locked = engine
        .fetch_and_lock_service_tasks("worker2", 10, &["err".into()], 60)
        .await;
    assert!(locked.is_empty());

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("ERR_CODE")));
}
