use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use std::collections::HashMap;

#[test]
fn suspended_call_activity_status_does_not_copy_parked_token_variables() {
    let state = InstanceState::Suspended {
        previous_state: Box::new(InstanceState::WaitingOnCallActivity {
            sub_instance_id: uuid::Uuid::new_v4(),
            token: Token::with_variables(
                "call",
                HashMap::from([("large".into(), serde_json::json!("x".repeat(10000)))]),
            ),
        }),
    };
    let payload = serde_json::to_string(&InstanceStatus::from(&state)).unwrap();
    assert!(payload.len() < 1024);
    assert!(!payload.contains("token"));
    assert!(!payload.contains("large"));
}

#[tokio::test]
async fn events_identify_tasks_admin_changes_and_deletion_without_variables() {
    let engine = WorkflowEngine::new();
    let mut events = engine.subscribe_events();
    let def = ProcessDefinitionBuilder::new("event-test")
        .node("start", BpmnElement::StartEvent)
        .node("user", BpmnElement::UserTask("tester".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "user")
        .flow("user", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    assert!(
        matches!(events.recv().await.unwrap(), EngineEvent::DefinitionChanged { definition_key } if definition_key == key)
    );
    let id = engine.start_instance(key).await.unwrap();
    let task = match events.recv().await.unwrap() {
        EngineEvent::TaskChanged {
            instance_id,
            task_id,
            definition_key,
        } => {
            assert_eq!(instance_id, id);
            assert_eq!(definition_key, Some(key));
            task_id
        }
        other => panic!("Unexpected event: {other:?}"),
    };
    let payload = serde_json::to_value(events.recv().await.unwrap()).unwrap();
    assert_eq!(payload["instance_id"], id.to_string());
    assert_eq!(payload["delta"]["current_node"], "user");
    assert!(payload["delta"].get("variables").is_none());
    engine.suspend_instance(id).await.unwrap();
    let payload = serde_json::to_value(events.recv().await.unwrap()).unwrap();
    assert!(payload["delta"]["state"].get("Suspended").is_some());
    engine.resume_instance(id).await.unwrap();
    events.recv().await.unwrap();
    engine
        .update_instance_variables(
            id,
            HashMap::from([("large".into(), serde_json::json!("x".repeat(10000)))]),
        )
        .await
        .unwrap();
    let payload = serde_json::to_string(&events.recv().await.unwrap()).unwrap();
    assert!(payload.len() < 1024);
    engine
        .complete_user_task(task, HashMap::new())
        .await
        .unwrap();
    assert!(
        matches!(events.recv().await.unwrap(), EngineEvent::TaskChanged { task_id, .. } if task_id == task)
    );
    let payload = serde_json::to_value(events.recv().await.unwrap()).unwrap();
    assert_eq!(payload["delta"]["state"], "Completed");
    engine.delete_instance(id).await.unwrap();
    let payload = serde_json::to_value(events.recv().await.unwrap()).unwrap();
    assert_eq!(payload["removed"], true);
    assert!(payload.get("delta").is_none());
}

#[tokio::test]
async fn service_lock_failure_and_retry_emit_scoped_task_events() {
    let engine = WorkflowEngine::new();
    let mut events = engine.subscribe_events();
    let def = ProcessDefinitionBuilder::new("service-events")
        .node("start", BpmnElement::StartEvent)
        .node(
            "service",
            BpmnElement::ServiceTask {
                topic: "events".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "service")
        .flow("service", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    events.recv().await.unwrap();
    let id = engine.start_instance(key).await.unwrap();
    events.recv().await.unwrap();
    events.recv().await.unwrap();
    let tasks = engine
        .fetch_and_lock_service_tasks("worker", 1, &["events".into()], 30)
        .await;
    let task_id = tasks[0].id;
    assert!(
        matches!(events.recv().await.unwrap(), EngineEvent::TaskChanged { instance_id, task_id: task, .. } if instance_id == id && task == task_id)
    );
    engine
        .fail_service_task(task_id, "worker", Some(0), Some("failed".into()), None)
        .await
        .unwrap();
    assert!(
        matches!(events.recv().await.unwrap(), EngineEvent::TaskChanged { task_id: task, .. } if task == task_id)
    );
    engine.retry_incident(task_id, Some(3)).await.unwrap();
    assert!(
        matches!(events.recv().await.unwrap(), EngineEvent::TaskChanged { task_id: task, .. } if task == task_id)
    );
}
