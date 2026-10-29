use engine_core::model::Token;
use engine_core::persistence::WorkflowPersistence;
use std::sync::Arc;
use uuid::Uuid;

use crate::client::NatsPersistence;

fn nats_url() -> String {
    std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string())
}

fn nats_required() -> bool {
    std::env::var("CI").is_ok() || std::env::var("REQUIRE_NATS_TESTS").is_ok()
}

pub async fn setup_nats_test() -> Option<Arc<NatsPersistence>> {
    connect_nats(&format!("TEST_STREAM_{}", Uuid::new_v4())).await
}

async fn connect_nats(stream: &str) -> Option<Arc<NatsPersistence>> {
    let url = nats_url();
    match NatsPersistence::connect(&url, stream).await {
        Ok(persistence) => Some(Arc::new(persistence)),
        Err(e) => {
            if nats_required() {
                panic!("NATS JetStream required in CI ({url}): {e}");
            }
            tracing::warn!("Skipping NATS test, could not connect: {e}");
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

    persistence.save_token(instance_id, &token).await.unwrap();

    // Event-Sourcing Light Scenario
    token.current_node = "next_node".to_string();
    persistence.save_token(instance_id, &token).await.unwrap();

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

    let url = nats_url();
    let stream = format!("TEST_RESTORE_{}", Uuid::new_v4());

    let persistence = match connect_nats(&stream).await {
        Some(p) => p,
        None => return,
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
        outstanding_calls: HashMap::new(),
        incidents: Vec::new(),
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
    };

    persistence.save_instance(&instance).await.unwrap();

    // Simulate crash recovery: new connection, reload from KV.
    let restored = match NatsPersistence::connect(&url, &stream).await {
        Ok(p) => Arc::new(p),
        Err(e) => {
            if nats_required() {
                panic!("NATS JetStream reconnect required in CI ({url}): {e}");
            }
            tracing::warn!("Skipping restore reconnect: {e}");
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
    assert_eq!(
        found.variables.get("foo").and_then(|v| v.as_str()),
        Some("bar")
    );
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

/// R7: neu angelegte Workflow-Streams tragen explizite Retention-Limits.
#[tokio::test]
async fn test_stream_retention_limits_are_applied() {
    use std::time::Duration;

    let stream = format!("TEST_RETENTION_{}", Uuid::new_v4());
    let persistence = match connect_nats(&stream).await {
        Some(p) => p,
        None => return,
    };

    let mut handle = persistence
        .jetstream()
        .get_stream(&stream)
        .await
        .expect("stream must exist after connect");
    let info = handle.info().await.expect("stream info must be available");
    let config = &info.config;

    assert!(
        config.max_age > Duration::ZERO,
        "WORKFLOW_EVENTS must have a bounded max_age"
    );
    assert!(
        config.max_bytes > 0,
        "WORKFLOW_EVENTS must have a bounded max_bytes"
    );
    assert!(
        config.num_replicas >= 1,
        "WORKFLOW_EVENTS must have at least one replica"
    );
    assert_eq!(config.subjects, vec![format!("{stream}.*")]);

    // Der Bootstrap-Bericht muss den History-Stream ausweisen.
    let report = persistence.bootstrap_report();
    assert!(
        report
            .existing()
            .iter()
            .chain(report.created().iter())
            .any(|resource| resource.as_str() == "stream:WORKFLOW_HISTORY"),
        "bootstrap report must mention WORKFLOW_HISTORY: {report:?}"
    );
}

/// Signal-Catches überleben den Reload aus dem KV-Bucket.
#[tokio::test]
async fn test_signal_catch_save_list_and_delete() {
    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let catch = engine_core::PendingSignalCatch {
        id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        node_id: "wait_signal".into(),
        signal_name: "ORDER_READY".into(),
        token_id: Uuid::new_v4(),
    };

    persistence.save_signal_catch(&catch).await.unwrap();

    let listed = persistence.list_signal_catches().await.unwrap();
    let found = listed
        .iter()
        .find(|c| c.id == catch.id)
        .expect("signal catch must be listed after reload");
    assert_eq!(found.instance_id, catch.instance_id);
    assert_eq!(found.signal_name, "ORDER_READY");
    assert_eq!(found.token_id, catch.token_id);

    persistence.delete_signal_catch(catch.id).await.unwrap();
    let after = persistence.list_signal_catches().await.unwrap();
    assert!(after.iter().all(|c| c.id != catch.id));
}

/// Conditional-Catches inklusive `last_result` überleben den Reload aus dem KV-Bucket.
#[tokio::test]
async fn test_conditional_catch_save_list_and_delete() {
    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let catch = engine_core::PendingConditionalCatch {
        id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        node_id: "wait_condition".into(),
        condition: "amount > 100".into(),
        token_id: Uuid::new_v4(),
        last_result: true,
    };

    persistence.save_conditional_catch(&catch).await.unwrap();

    let listed = persistence.list_conditional_catches().await.unwrap();
    let found = listed
        .iter()
        .find(|c| c.id == catch.id)
        .expect("conditional catch must be listed after reload");
    assert_eq!(found.instance_id, catch.instance_id);
    assert_eq!(found.node_id, "wait_condition");
    assert_eq!(found.condition, "amount > 100");
    assert_eq!(found.token_id, catch.token_id);
    assert!(found.last_result);

    persistence
        .delete_conditional_catch(catch.id)
        .await
        .unwrap();
    let after = persistence.list_conditional_catches().await.unwrap();
    assert!(after.iter().all(|c| c.id != catch.id));
}

/// Service-Tasks, Timer und Message-Catches überleben Save/List/Delete.
#[tokio::test]
async fn test_service_task_timer_and_message_catch_roundtrip() {
    use engine_core::runtime::{PendingMessageCatch, PendingServiceTask, PendingTimer};
    use std::collections::HashMap;

    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let instance_id = Uuid::new_v4();
    let token_id = Uuid::new_v4();
    let definition_key = Uuid::new_v4();

    let service_task = PendingServiceTask {
        id: Uuid::new_v4(),
        instance_id,
        definition_key,
        node_id: "Service_1".into(),
        topic: "orders".into(),
        token_id,
        business_key: Some("order-7".into()),
        variables_snapshot: HashMap::from([("amount".into(), serde_json::json!(42))]),
        created_at: chrono::Utc::now(),
        worker_id: Some("worker-a".into()),
        lock_expiration: None,
        retries: 3,
        error_message: None,
        error_details: None,
    };
    persistence.save_service_task(&service_task).await.unwrap();
    let tasks = persistence.list_service_tasks().await.unwrap();
    let found = tasks
        .iter()
        .find(|t| t.id == service_task.id)
        .expect("service task must be listed");
    assert_eq!(found.topic, "orders");
    assert_eq!(found.business_key.as_deref(), Some("order-7"));
    assert_eq!(found.retries, 3);
    persistence
        .delete_service_task(service_task.id)
        .await
        .unwrap();
    assert!(
        persistence
            .list_service_tasks()
            .await
            .unwrap()
            .iter()
            .all(|t| t.id != service_task.id)
    );

    let timer = PendingTimer {
        id: Uuid::new_v4(),
        instance_id,
        node_id: "Timer_1".into(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
        token_id,
        timer_def: None,
        remaining_repetitions: Some(2),
    };
    persistence.save_timer(&timer).await.unwrap();
    let timers = persistence.list_timers().await.unwrap();
    let found = timers
        .iter()
        .find(|t| t.id == timer.id)
        .expect("timer must be listed");
    assert_eq!(found.remaining_repetitions, Some(2));
    persistence.delete_timer(timer.id).await.unwrap();
    assert!(
        persistence
            .list_timers()
            .await
            .unwrap()
            .iter()
            .all(|t| t.id != timer.id)
    );

    let catch = PendingMessageCatch {
        id: Uuid::new_v4(),
        instance_id,
        node_id: "Wait_1".into(),
        message_name: "OrderPaid".into(),
        token_id,
    };
    persistence.save_message_catch(&catch).await.unwrap();
    let catches = persistence.list_message_catches().await.unwrap();
    let found = catches
        .iter()
        .find(|c| c.id == catch.id)
        .expect("message catch must be listed");
    assert_eq!(found.message_name, "OrderPaid");
    persistence.delete_message_catch(catch.id).await.unwrap();
    assert!(
        persistence
            .list_message_catches()
            .await
            .unwrap()
            .iter()
            .all(|c| c.id != catch.id)
    );
}

/// Token-Delete und Datei-Lifecycle im Object Store.
#[tokio::test]
async fn test_token_delete_and_file_lifecycle() {
    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let instance_id = Uuid::new_v4();
    let first = Token::new("node_a");
    let second = Token::new("node_b");
    persistence.save_token(instance_id, &first).await.unwrap();
    persistence.save_token(instance_id, &second).await.unwrap();

    let loaded = persistence.load_tokens(instance_id).await.unwrap();
    assert_eq!(loaded.len(), 2);

    persistence
        .delete_token(instance_id, first.id)
        .await
        .unwrap();
    let loaded = persistence.load_tokens(instance_id).await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, second.id);

    let object_key = format!("instance-{instance_id}/hello.txt");
    persistence.save_file(&object_key, b"hello").await.unwrap();
    assert_eq!(persistence.load_file(&object_key).await.unwrap(), b"hello");

    let entries = persistence
        .get_bucket_entries("instance_files", 0, 10)
        .await
        .unwrap();
    assert!(entries.iter().any(|e| e.key == object_key));

    let detail = persistence
        .get_bucket_entry_detail("instance_files", &object_key)
        .await
        .unwrap();
    assert_eq!(detail.encoding, "base64");
    assert_eq!(detail.data, "aGVsbG8=");

    persistence.delete_file(&object_key).await.unwrap();
    assert!(persistence.load_file(&object_key).await.is_err());
}

/// Archivierte Instanzen inklusive Filter, Pagination und Bucket-Details.
#[tokio::test]
async fn test_completed_instance_archive_and_query() {
    use engine_core::persistence::CompletedInstanceQuery;
    use engine_core::runtime::{InstanceState, ProcessInstance};
    use std::collections::HashMap;

    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let completed_at = chrono::Utc::now();
    let instance = ProcessInstance {
        id: Uuid::new_v4(),
        definition_key: Uuid::new_v4(),
        business_key: "order-archived".into(),
        parent_instance_id: None,
        state: InstanceState::Completed,
        current_node: "end".into(),
        audit_log: vec!["finished".into()],
        variables: HashMap::new(),
        tokens: HashMap::new(),
        active_tokens: vec![],
        join_barriers: HashMap::new(),
        multi_instance_state: HashMap::new(),
        compensation_log: vec![],
        outstanding_calls: HashMap::new(),
        incidents: Vec::new(),
        started_at: Some(completed_at - chrono::Duration::seconds(5)),
        completed_at: Some(completed_at),
    };

    persistence.save_instance(&instance).await.unwrap();
    persistence
        .save_completed_instance(&instance)
        .await
        .unwrap();

    let stored = persistence
        .get_completed_instance(&instance.id.to_string())
        .await
        .unwrap()
        .expect("completed instance must be loadable");
    assert_eq!(stored.business_key, "order-archived");
    assert_eq!(stored.state, InstanceState::Completed);

    let by_business_key = persistence
        .query_completed_instances(CompletedInstanceQuery {
            business_key: Some("archived".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(by_business_key.iter().any(|i| i.id == instance.id));

    let completed = persistence
        .query_completed_instances(CompletedInstanceQuery {
            state_filter: Some("completed".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(completed.iter().any(|i| i.id == instance.id));

    let errors_only = persistence
        .query_completed_instances(CompletedInstanceQuery {
            state_filter: Some("error".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(errors_only.iter().all(|i| i.id != instance.id));

    let paged = persistence
        .query_completed_instances(CompletedInstanceQuery {
            limit: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(paged.len() <= 1);

    // Das KV-Bucket ist global und sammelt Einträge aller Testläufe. Deshalb
    // über alle Seiten blättern, statt einen Treffer im ersten Fenster zu erwarten.
    let instance_key = instance.id.to_string();
    let mut offset = 0;
    let mut found = false;
    loop {
        let page = persistence
            .get_bucket_entries("instances", offset, 50)
            .await
            .unwrap();
        if page.is_empty() {
            break;
        }
        found |= page.iter().any(|entry| entry.key == instance_key);
        offset += page.len();
    }
    assert!(found, "archivierte Instanz muss im Bucket auffindbar sein");

    let detail = persistence
        .get_bucket_entry_detail("instances", &instance.id.to_string())
        .await
        .unwrap();
    assert!(detail.data.contains("order-archived"));

    let missing = persistence
        .get_bucket_entry_detail("definitions", "missing-key")
        .await;
    assert!(missing.is_err());
}

/// BPMN-XML, History-Filter und Storage-Info decken die Monitoring-Pfade ab.
#[tokio::test]
async fn test_bpmn_xml_history_filters_and_storage_info() {
    use engine_core::domain::{BpmnElement, ProcessDefinitionBuilder};
    use engine_core::history::{ActorType, HistoryEntry, HistoryEventType};
    use engine_core::persistence::HistoryQuery;

    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let xml = r#"<?xml version="1.0" encoding="UTF-8"?><definitions id="d"/>"#;
    let definition = ProcessDefinitionBuilder::new("xml_proc")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .expect("valid definition");
    let key = definition.key.to_string();

    persistence.save_definition(&definition).await.unwrap();
    persistence.save_bpmn_xml(&key, xml).await.unwrap();
    assert_eq!(persistence.load_bpmn_xml(&key).await.unwrap(), xml);
    assert!(
        persistence
            .list_bpmn_xml_ids()
            .await
            .unwrap()
            .contains(&key)
    );

    let xml_entries = persistence
        .get_bucket_entries("bpmn_xml", 0, 10)
        .await
        .unwrap();
    assert!(xml_entries.iter().any(|e| e.key == key));
    let xml_detail = persistence
        .get_bucket_entry_detail("bpmn_xml", &key)
        .await
        .unwrap();
    assert_eq!(xml_detail.encoding, "utf8");
    assert_eq!(xml_detail.data, xml);

    persistence.delete_definition(&key).await.unwrap();
    assert!(persistence.load_bpmn_xml(&key).await.is_err());

    let instance_id = Uuid::new_v4();
    let started = HistoryEntry::new(
        instance_id,
        HistoryEventType::InstanceStarted,
        "started",
        ActorType::Engine,
        None,
    );
    let advanced = HistoryEntry::new(
        instance_id,
        HistoryEventType::TokenAdvanced,
        "advanced",
        ActorType::Engine,
        None,
    )
    .with_node("task_1");
    let completed = HistoryEntry::new(
        instance_id,
        HistoryEventType::TaskCompleted,
        "completed",
        ActorType::ServiceWorker,
        None,
    )
    .with_node("task_1");
    for entry in [&started, &advanced, &completed] {
        persistence.append_history_entry(entry).await.unwrap();
    }
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let by_type = persistence
        .query_history(HistoryQuery {
            instance_id,
            event_types: Some(vec![HistoryEventType::InstanceStarted]),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_type.len(), 1);
    assert_eq!(by_type[0].id, started.id);

    let by_node = persistence
        .query_history(HistoryQuery {
            instance_id,
            node_id: Some("task_1".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_node.len(), 2);

    let by_actor = persistence
        .query_history(HistoryQuery {
            instance_id,
            actor_type: Some(ActorType::ServiceWorker),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(by_actor.len(), 1);
    assert_eq!(by_actor[0].id, completed.id);

    let paged = persistence
        .query_history(HistoryQuery {
            instance_id,
            limit: Some(1),
            offset: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(paged.len(), 1);
    assert_eq!(paged[0].id, advanced.id);

    let info = persistence
        .get_storage_info()
        .await
        .unwrap()
        .expect("NATS must report storage info");
    assert!(!info.backend_name.is_empty());
    assert!(!info.version.is_empty());
    assert!(info.buckets.iter().any(|b| b.name == "instances"));
    assert!(info.buckets.iter().any(|b| b.name == "bpmn_xml"));
}

/// KV- und Object-Store-Handles überleben den ersten Lookup und schreiben danach
/// weiter den aktuellen Wert.
#[tokio::test]
async fn test_handles_are_cached_and_still_write() {
    use engine_core::runtime::{InstanceState, ProcessInstance};
    use std::collections::HashMap;

    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    assert!(!persistence.kv_handle_cached("instances"));
    assert!(!persistence.object_store_handle_cached("bpmn_xml"));

    let instance_id = Uuid::new_v4();
    let mut instance = ProcessInstance {
        id: instance_id,
        definition_key: Uuid::new_v4(),
        business_key: "cache".into(),
        parent_instance_id: None,
        state: InstanceState::Running,
        current_node: "start".into(),
        audit_log: Vec::new(),
        variables: HashMap::from([("n".into(), serde_json::json!(1))]),
        tokens: HashMap::new(),
        active_tokens: vec![],
        join_barriers: HashMap::new(),
        multi_instance_state: HashMap::new(),
        compensation_log: vec![],
        outstanding_calls: HashMap::new(),
        incidents: Vec::new(),
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
    };

    persistence.save_instance(&instance).await.unwrap();
    assert!(persistence.kv_handle_cached("instances"));

    instance.variables.insert("n".into(), serde_json::json!(2));
    persistence.save_instance(&instance).await.unwrap();

    let stored = persistence
        .list_instances()
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == instance_id)
        .expect("cached handle must read the instance");
    assert_eq!(stored.variables.get("n"), Some(&serde_json::json!(2)));

    let definition_id = Uuid::new_v4().to_string();
    persistence
        .save_bpmn_xml(&definition_id, "<definitions/>")
        .await
        .unwrap();
    assert!(persistence.object_store_handle_cached("bpmn_xml"));
    let xml = persistence.load_bpmn_xml(&definition_id).await.unwrap();
    assert_eq!(xml, "<definitions/>");
}
