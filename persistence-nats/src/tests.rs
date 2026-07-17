use engine_core::model::Token;
use engine_core::persistence::WorkflowPersistence;
use std::sync::Arc;
use uuid::Uuid;

use crate::client::NatsPersistence;

pub async fn setup_nats_test() -> Option<Arc<NatsPersistence>> {
    let url = "nats://localhost:4222";
    let stream = format!("TEST_STREAM_{}", Uuid::new_v4());

    match NatsPersistence::connect(url, &stream).await {
        Ok(persistence) => Some(Arc::new(persistence)),
        Err(e) => {
            tracing::warn!("Skipping NATS test, could not connect: {}", e);
            None
        }
    }
}

#[tokio::test]
async fn test_save_and_load_token() {
    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return, // Ignore if NATS container is not running
    };

    let instance_id = Uuid::new_v4();
    let mut token = Token::new("start_node");
    token.variables.insert(
        "test_key".into(),
        serde_json::Value::String("test_value".into()),
    );

    persistence
        .save_token(instance_id, &token)
        .await
        .unwrap();

    // Event-Sourcing Light Scenario
    token.current_node = "next_node".to_string();
    persistence
        .save_token(instance_id, &token)
        .await
        .unwrap();

    let loaded_tokens = persistence.load_tokens(instance_id).await.unwrap();

    assert_eq!(loaded_tokens.len(), 1);
    let loaded_token = &loaded_tokens[0];

    assert_eq!(loaded_token.id, token.id);
    assert_eq!(loaded_token.current_node, "next_node");
    assert_eq!(
        loaded_token
            .variables
            .get("test_key")
            .unwrap()
            .as_str()
            .unwrap(),
        "test_value"
    );
}

#[tokio::test]
async fn test_history_append_and_load() {
    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return, // Ignore if NATS container is not running
    };

    let instance_id = Uuid::new_v4();
    let entry1 = engine_core::history::HistoryEntry::new(
        instance_id,
        engine_core::history::HistoryEventType::InstanceStarted,
        "Instance Started",
        engine_core::history::ActorType::Engine,
        None,
    );

    let entry2 = engine_core::history::HistoryEntry::new(
        instance_id,
        engine_core::history::HistoryEventType::TokenAdvanced,
        "Token moved",
        engine_core::history::ActorType::Engine,
        None,
    )
    .with_node("task_1");

    // Append to stream
    persistence.append_history_entry(&entry1).await.unwrap();
    persistence.append_history_entry(&entry2).await.unwrap();

    // Give NATS JetStream a tiny bit of time to flush/index
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let history = persistence
        .query_history(engine_core::persistence::HistoryQuery {
            instance_id,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, entry1.id);
    assert_eq!(
        history[0].event_type,
        engine_core::history::HistoryEventType::InstanceStarted
    );
    assert_eq!(history[1].id, entry2.id);
    assert_eq!(
        history[1].event_type,
        engine_core::history::HistoryEventType::TokenAdvanced
    );
    assert_eq!(history[1].node_id.as_deref(), Some("task_1"));
}

/// Definition + instance roundtrip and restore via a fresh connection (M2).
#[tokio::test]
async fn test_definition_instance_restore_roundtrip() {
    use engine_core::domain::{BpmnElement, ProcessDefinitionBuilder};
    use engine_core::runtime::{InstanceState, ProcessInstance};
    use std::collections::HashMap;

    let url = "nats://localhost:4222";
    let stream = format!("TEST_RESTORE_{}", Uuid::new_v4());

    let persistence = match NatsPersistence::connect(url, &stream).await {
        Ok(p) => Arc::new(p),
        Err(e) => {
            tracing::warn!("Skipping NATS restore test, could not connect: {}", e);
            return;
        }
    };

    let def = ProcessDefinitionBuilder::new("restore_proc")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .expect("valid definition");
    let def_key = def.key;

    persistence.save_definition(&def).await.unwrap();

    let instance_id = Uuid::new_v4();
    let mut variables = HashMap::new();
    variables.insert("foo".into(), serde_json::json!("bar"));
    let instance = ProcessInstance {
        id: instance_id,
        definition_key: def_key,
        business_key: "bk-restore".into(),
        parent_instance_id: None,
        state: InstanceState::Running,
        current_node: "start".into(),
        audit_log: vec!["started".into()],
        variables,
        tokens: HashMap::new(),
        active_tokens: vec![],
        join_barriers: HashMap::new(),
        multi_instance_state: HashMap::new(),
        compensation_log: vec![],
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
    };

    persistence.save_instance(&instance).await.unwrap();

    // Simulate crash recovery: new connection, reload from KV.
    let restored = match NatsPersistence::connect(url, &stream).await {
        Ok(p) => Arc::new(p),
        Err(e) => {
            tracing::warn!("Skipping restore reconnect: {}", e);
            return;
        }
    };

    let definitions = restored.list_definitions().await.unwrap();
    let found_def = definitions
        .iter()
        .find(|d| d.key == def_key)
        .expect("definition must be restored");
    assert_eq!(found_def.id, "restore_proc");

    let instances = restored.list_instances().await.unwrap();
    let found = instances
        .iter()
        .find(|i| i.id == instance_id)
        .expect("instance must be restored");
    assert_eq!(found.business_key, "bk-restore");
    assert_eq!(found.current_node, "start");
    assert_eq!(found.variables.get("foo").and_then(|v| v.as_str()), Some("bar"));
    assert_eq!(found.state, InstanceState::Running);

    // Delete and verify gone
    restored
        .delete_instance(&instance_id.to_string())
        .await
        .unwrap();
    restored
        .delete_definition(&def_key.to_string())
        .await
        .unwrap();

    let after_del = restored.list_instances().await.unwrap();
    assert!(
        after_del.iter().all(|i| i.id != instance_id),
        "instance should be deleted"
    );
}

/// User-task save/list/delete roundtrip (M2 restore path for pending work).
#[tokio::test]
async fn test_user_task_restore_roundtrip() {
    use engine_core::runtime::PendingUserTask;

    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let task = PendingUserTask {
        task_id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        node_id: "Task_1".into(),
        assignee: "alice".into(),
        token_id: Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        business_key: Some("order-42".into()),
    };

    persistence.save_user_task(&task).await.unwrap();
    let listed = persistence.list_user_tasks().await.unwrap();
    let found = listed
        .iter()
        .find(|t| t.task_id == task.task_id)
        .expect("user task must be listed");
    assert_eq!(found.assignee, "alice");
    assert_eq!(found.business_key.as_deref(), Some("order-42"));

    persistence.delete_user_task(task.task_id).await.unwrap();
    let after = persistence.list_user_tasks().await.unwrap();
    assert!(after.iter().all(|t| t.task_id != task.task_id));
}
