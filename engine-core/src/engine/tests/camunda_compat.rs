use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use std::collections::HashMap;

async fn service() -> (WorkflowEngine, uuid::Uuid, uuid::Uuid) {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("lease-compat")
        .node("start", BpmnElement::StartEvent)
        .node(
            "service",
            BpmnElement::ServiceTask {
                topic: "compat".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "service")
        .flow("service", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    (engine, key, id)
}

#[tokio::test]
async fn retry_deadline_survives_restore_and_blocks_fetch_until_expiry() {
    let (engine, _, id) = service().await;
    let task = engine
        .fetch_and_lock_service_tasks_ms("worker", 1, &["compat".into()], 12345)
        .await
        .remove(0);
    let before = chrono::Utc::now();
    engine
        .fail_service_task_with_retry_timeout(task.id, "worker", Some(2), None, None, 60000)
        .await
        .unwrap();
    let pending = engine.get_pending_service_tasks().remove(0);
    assert!(pending.lock_expiration.unwrap() >= before + chrono::TimeDelta::milliseconds(60000));
    assert!(pending.worker_id.is_none());
    let restored = WorkflowEngine::new();
    restored
        .restore_instance(
            serde_json::from_slice(
                &serde_json::to_vec(&engine.get_instance_details(id).await.unwrap()).unwrap(),
            )
            .unwrap(),
        )
        .await;
    restored.restore_service_task(
        serde_json::from_slice(&serde_json::to_vec(&pending).unwrap()).unwrap(),
    );
    assert!(
        restored
            .fetch_and_lock_service_tasks_ms("other", 1, &["compat".into()], 30000)
            .await
            .is_empty()
    );
    restored
        .pending_service_tasks
        .get_mut(&task.id)
        .unwrap()
        .lock_expiration = Some(chrono::Utc::now() - chrono::TimeDelta::milliseconds(1));
    assert_eq!(
        restored
            .fetch_and_lock_service_tasks_ms("other", 1, &["compat".into()], 30000)
            .await
            .len(),
        1
    );
}

#[tokio::test]
async fn suspended_instances_are_not_fetchable_and_millisecond_extension_is_exact() {
    let (engine, _, id) = service().await;
    engine.suspend_instance(id).await.unwrap();
    assert!(
        engine
            .fetch_and_lock_service_tasks_ms("worker", 1, &["compat".into()], 30000)
            .await
            .is_empty()
    );
    engine.resume_instance(id).await.unwrap();
    let task = engine
        .fetch_and_lock_service_tasks_ms("worker", 1, &["compat".into()], 30000)
        .await
        .remove(0);
    let before = chrono::Utc::now();
    engine
        .extend_lock_ms(task.id, "worker", 12345)
        .await
        .unwrap();
    let after = chrono::Utc::now();
    let expiry = engine
        .service_task_lock(task.id)
        .unwrap()
        .lock_expiration
        .unwrap();
    assert!(expiry >= before + chrono::TimeDelta::milliseconds(12345));
    assert!(expiry <= after + chrono::TimeDelta::milliseconds(12345));
    assert!(
        engine
            .fail_service_task_with_retry_timeout(task.id, "worker", Some(2), None, None, -1)
            .await
            .is_err()
    );
    assert_eq!(
        engine
            .service_task_lock(task.id)
            .unwrap()
            .worker_id
            .as_deref(),
        Some("worker")
    );
}

#[tokio::test]
async fn legacy_snapshots_default_type_hints_and_native_updates_clear_them() {
    let (engine, _, id) = service().await;
    let mut instance = engine.get_instance_details(id).await.unwrap();
    instance
        .variable_types
        .insert("amount".into(), "Long".into());
    instance
        .variables
        .insert("amount".into(), serde_json::json!(5));
    let bytes = serde_json::to_vec(&instance).unwrap();
    let restored: crate::ProcessInstance = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored.variable_types["amount"], "Long");
    let mut old = serde_json::to_value(&restored).unwrap();
    old.as_object_mut().unwrap().remove("variable_types");
    assert!(
        serde_json::from_value::<crate::ProcessInstance>(old)
            .unwrap()
            .variable_types
            .is_empty()
    );
    engine.restore_instance(restored).await;
    engine
        .update_instance_variables(
            id,
            HashMap::from([("amount".into(), serde_json::json!("now a string"))]),
        )
        .await
        .unwrap();
    assert!(
        engine
            .get_instance_details(id)
            .await
            .unwrap()
            .variable_types
            .is_empty()
    );
}
