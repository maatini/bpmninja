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

#[tokio::test]
async fn x11_mapped_external_task_survives_nats_reconnect() {
    use engine_core::domain::{ActivityMappings, BpmnElement, DataMapping, MappingValue, ProcessDefinitionBuilder};
    use engine_core::WorkflowEngine;
    use std::collections::HashMap;
    use serde_json::json;
    let stream = format!("TEST_X11_{}", Uuid::new_v4());
    let Some(persistence) = connect_nats(&stream).await else { return; };
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let mut def = ProcessDefinitionBuilder::new("mapped_restore")
        .node("start", BpmnElement::StartEvent)
        .node("work", BpmnElement::ServiceTask { topic: "mapped".into(), multi_instance: None })
        .node("end", BpmnElement::EndEvent).flow("start", "work").flow("work", "end").build().unwrap();
    def.variable_defaults.insert("customerId".into(), json!(7));
    def.data_mappings.insert("work".into(), ActivityMappings {
        inputs: vec![DataMapping { target: "id".into(), value: MappingValue::Variable("customerId".into()) }],
        outputs: vec![DataMapping { target: "receipt".into(), value: MappingValue::Variable("result".into()) }],
        ..Default::default()
    });
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = engine.fetch_and_lock_service_tasks("worker", 1, &["mapped".into()], 60).await.remove(0);
    assert_eq!(task.variables_snapshot["id"], json!(7));
    engine.fail_service_task(task.id, "worker", Some(2), Some("retry".into()), None).await.unwrap();
    engine.update_instance_variables(id, HashMap::from([("customerId".into(), json!(99))])).await.unwrap();
    drop(engine);
    let fresh = Arc::new(NatsPersistence::connect(&nats_url(), &stream).await.unwrap());
    let restored = WorkflowEngine::new().with_persistence(fresh.clone());
    for def in fresh.list_definitions().await.unwrap().into_iter().filter(|def| def.key == key) { restored.deploy_definition(def).await; }
    for instance in fresh.list_instances().await.unwrap().into_iter().filter(|instance| instance.id == id) { restored.restore_instance(instance).await; }
    for task in fresh.list_service_tasks().await.unwrap().into_iter().filter(|task| task.instance_id == id) { restored.restore_service_task(task); }
    let task = restored.fetch_and_lock_service_tasks("worker", 1, &["mapped".into()], 60).await.remove(0);
    assert_eq!(restored.get_instance_details(id).await.unwrap().tokens[&task.token_id].variables["id"], json!(7));
    restored.complete_service_task(task.id, "worker", HashMap::from([("result".into(), json!(42))])).await.unwrap();
    let result = restored.get_instance_details(id).await.unwrap();
    assert_eq!(result.state, engine_core::InstanceState::Completed);
    assert_eq!(result.variables["receipt"], json!(42));
    assert_eq!(result.variables["customerId"], json!(99));
    assert!(!result.variables.contains_key("id"));
    fresh.delete_instance(&id.to_string()).await.unwrap();
    fresh.delete_definition(&key.to_string()).await.unwrap();
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
        variable_types: std::collections::HashMap::new(),
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
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
    };

    persistence.save_instance(&instance).await.unwrap();
    let mut serialized_instance = instance.clone();
    serialized_instance.id = Uuid::new_v4();
    serialized_instance
        .variables
        .insert("unicode".into(), serde_json::json!("مرحبا\n世界"));
    let payload = serde_json::to_vec(&serialized_instance).unwrap();
    persistence
        .save_instance_json(&serialized_instance, &payload)
        .await
        .unwrap();
    let prepared = persistence
        .prepare_instance_payload(&serialized_instance, payload.clone())
        .unwrap();
    persistence
        .save_instance_payload(&serialized_instance, &prepared)
        .await
        .unwrap();
    persistence
        .save_instance_payload(&serialized_instance, &prepared)
        .await
        .unwrap();
    let store = persistence.kv("instances").await.unwrap();
    let stored = store
        .get(serialized_instance.id.to_string())
        .await
        .unwrap()
        .unwrap();
    let stored_document = persistence.resolve_snapshot(&stored).await.unwrap();
    #[cfg(not(feature = "separate-audit-log"))]
    assert_eq!(stored_document.as_slice(), prepared.as_slice());
    #[cfg(feature = "separate-audit-log")]
    {
        let (record, audit) = crate::codec::split_prepared(&prepared).unwrap();
        assert_eq!(stored_document.as_slice(), record);
        let (state, key) = crate::codec::decode_instance_record(record).unwrap();
        assert!(state.audit_log.is_empty());
        let key = key.unwrap();
        let audit_store = persistence.kv("instance_audit").await.unwrap();
        let entry = audit_store.entry(&key).await.unwrap().unwrap();
        assert_eq!(entry.value.as_ref(), audit);
        let revision = entry.revision;
        // Reusing a snapshot never rewrites the immutable audit object.
        persistence
            .save_instance_payload(&serialized_instance, &prepared)
            .await
            .unwrap();
        assert_eq!(
            audit_store.entry(&key).await.unwrap().unwrap().revision,
            revision
        );
        // Crash between the audit write and state write: the old document
        // still references the old log, even though a newer audit exists.
        let mut changed = serialized_instance.clone();
        changed
            .audit_log
            .push("new entry before interrupted state save".into());
        let pending = crate::codec::prepare_detached(&changed).unwrap();
        let (pending_record, pending_audit) = crate::codec::split_prepared(&pending).unwrap();
        let (_, pending_key) = crate::codec::decode_instance_record(pending_record).unwrap();
        audit_store
            .create(pending_key.as_ref().unwrap(), pending_audit.to_vec().into())
            .await
            .unwrap();
        let prior = persistence.restore_instance_record(&stored).await.unwrap();
        assert_eq!(prior.audit_log, serialized_instance.audit_log);
        // Missing/corrupt audit snapshots fail loudly; do not restore an empty log.
        audit_store.purge(&key).await.unwrap();
        assert!(
            persistence
                .restore_instance_record(&stored)
                .await
                .unwrap_err()
                .to_string()
                .contains("Missing audit")
        );
        audit_store
            .create(&key, b"[\"corrupt\"]".to_vec().into())
            .await
            .unwrap();
        assert!(
            persistence
                .restore_instance_record(&stored)
                .await
                .unwrap_err()
                .to_string()
                .contains("digest mismatch")
        );
        audit_store.purge(&key).await.unwrap();
        audit_store
            .create(&key, audit.to_vec().into())
            .await
            .unwrap();
    }
    let decoded = persistence.restore_instance_record(&stored).await.unwrap();
    assert_eq!(
        serde_json::to_value(decoded).unwrap(),
        serde_json::to_value(&serialized_instance).unwrap()
    );
    #[cfg(not(any(feature = "compact-instances", feature = "separate-audit-log")))]
    assert_eq!(stored_document.as_slice(), payload.as_slice());
    let detail = persistence
        .get_bucket_entry_detail("instances", &serialized_instance.id.to_string())
        .await
        .unwrap();
    assert_eq!(detail.encoding, "utf8");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&detail.data).unwrap(),
        serde_json::to_value(&serialized_instance).unwrap()
    );
    // A legacy JSON record must restore beside compact records.
    let mut legacy_instance = instance.clone();
    legacy_instance.id = Uuid::new_v4();
    store
        .put(
            legacy_instance.id.to_string(),
            serde_json::to_vec(&legacy_instance).unwrap().into(),
        )
        .await
        .unwrap();
    // Exercise version-2 hydration even with detached writes disabled.
    let mut detached_fixture = instance.clone();
    detached_fixture.id = Uuid::new_v4();
    detached_fixture.audit_log.push("detached fixture".into());
    let detached_payload = crate::codec::prepare_detached(&detached_fixture).unwrap();
    let (detached_record, detached_audit) =
        crate::codec::split_prepared(&detached_payload).unwrap();
    let (_, detached_key) = crate::codec::decode_instance_record(detached_record).unwrap();
    persistence
        .kv("instance_audit")
        .await
        .unwrap()
        .create(
            detached_key.as_ref().unwrap(),
            detached_audit.to_vec().into(),
        )
        .await
        .unwrap();
    store
        .put(
            detached_fixture.id.to_string(),
            detached_record.to_vec().into(),
        )
        .await
        .unwrap();

    // A current state and archive may select different audit versions.
    let mut cleanup_archive = detached_fixture.clone();
    cleanup_archive.state = InstanceState::Completed;
    cleanup_archive
        .audit_log
        .push("archive-specific audit".into());
    let archived_payload = crate::codec::prepare_detached(&cleanup_archive).unwrap();
    let (archived_record, archived_audit) =
        crate::codec::split_prepared(&archived_payload).unwrap();
    let (_, archived_audit_key) = crate::codec::decode_instance_record(archived_record).unwrap();
    let audits = persistence.kv("instance_audit").await.unwrap();
    audits
        .create(
            archived_audit_key.as_ref().unwrap(),
            archived_audit.to_vec().into(),
        )
        .await
        .unwrap();
    persistence
        .kv("history_instances")
        .await
        .unwrap()
        .put(
            cleanup_archive.id.to_string(),
            archived_record.to_vec().into(),
        )
        .await
        .unwrap();
    let orphan_audit = serde_json::to_vec(&vec!["interrupted save"]).unwrap();
    let orphan_key = crate::codec::audit_key(detached_fixture.id, &orphan_audit);
    audits
        .create(&orphan_key, orphan_audit.into())
        .await
        .unwrap();
    // Seed the cache as if audit publication succeeded but the state save failed.
    persistence
        .audit_cache
        .lock()
        .await
        .insert(orphan_key.clone());
    let dry_run = persistence
        .prune_instance_audits(detached_fixture.id, true)
        .await
        .unwrap();
    assert_eq!(dry_run.retained.len(), 2);
    assert_eq!(dry_run.candidates, vec![orphan_key.clone()]);
    assert_eq!(dry_run.purged, 0);
    assert!(audits.get(&orphan_key).await.unwrap().is_some());
    // Bad live references abort before any candidate can be purged.
    audits.purge(detached_key.as_ref().unwrap()).await.unwrap();
    assert!(
        persistence
            .prune_instance_audits(detached_fixture.id, false)
            .await
            .is_err()
    );
    assert!(audits.get(&orphan_key).await.unwrap().is_some());
    audits
        .create(
            detached_key.as_ref().unwrap(),
            detached_audit.to_vec().into(),
        )
        .await
        .unwrap();
    let applied = persistence
        .prune_instance_audits(detached_fixture.id, false)
        .await
        .unwrap();
    assert_eq!(applied.purged, 1);
    assert!(audits.get(&orphan_key).await.unwrap().is_none());
    assert!(!persistence.audit_cache.lock().await.contains(&orphan_key));
    assert!(
        audits
            .get(detached_key.as_ref().unwrap())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        audits
            .get(archived_audit_key.as_ref().unwrap())
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        persistence
            .prune_instance_audits(detached_fixture.id, false)
            .await
            .unwrap()
            .purged,
        0
    );

    // A process-wide gate excludes cleanup from an in-flight local save,
    // even when cleanup uses a separate connection.
    let other = Arc::new(NatsPersistence::connect(&url, &stream).await.unwrap());
    let access = crate::audit::AUDIT_ACCESS.read().await;
    let target = detached_fixture.id;
    let mut cleanup = tokio::spawn(async move { other.prune_instance_audits(target, true).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut cleanup)
            .await
            .is_err()
    );
    drop(access);
    cleanup.await.unwrap().unwrap();
    let exclusive = crate::audit::AUDIT_ACCESS.write().await;
    let writer = persistence.clone();
    let snapshot = detached_fixture.clone();
    let mut save = tokio::spawn(async move { writer.save_instance(&snapshot).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(50), &mut save)
            .await
            .is_err()
    );
    drop(exclusive);
    save.await.unwrap().unwrap();
    // The write above may switch format in a default build; restore remains exact.
    let updated = store
        .get(detached_fixture.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        persistence
            .restore_instance_record(&updated)
            .await
            .unwrap()
            .audit_log,
        detached_fixture.audit_log
    );

    let mut archived = serialized_instance.clone();
    archived.state = InstanceState::Completed;
    archived.completed_at = Some(chrono::Utc::now());
    persistence
        .save_completed_instance(&archived)
        .await
        .unwrap();

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

    let found_serialized = instances
        .iter()
        .find(|i| i.id == serialized_instance.id)
        .expect("serialized instance must be restored");
    assert_eq!(
        serde_json::to_value(found_serialized).unwrap(),
        serde_json::to_value(&serialized_instance).unwrap()
    );
    restored
        .delete_instance(&serialized_instance.id.to_string())
        .await
        .unwrap();

    let found_detached = instances
        .iter()
        .find(|i| i.id == detached_fixture.id)
        .expect("detached record must hydrate in every build");
    assert_eq!(
        serde_json::to_value(found_detached).unwrap(),
        serde_json::to_value(&detached_fixture).unwrap()
    );
    restored
        .kv("history_instances")
        .await
        .unwrap()
        .delete(detached_fixture.id.to_string())
        .await
        .unwrap();
    restored
        .delete_instance(&detached_fixture.id.to_string())
        .await
        .unwrap();
    assert!(
        restored
            .kv("instance_audit")
            .await
            .unwrap()
            .get(detached_key.as_ref().unwrap())
            .await
            .unwrap()
            .is_none()
    );

    let found_legacy = instances
        .iter()
        .find(|i| i.id == legacy_instance.id)
        .expect("legacy JSON must restore");
    assert_eq!(
        serde_json::to_value(found_legacy).unwrap(),
        serde_json::to_value(&legacy_instance).unwrap()
    );
    let found_archived = restored
        .get_completed_instance(&archived.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(found_archived).unwrap(),
        serde_json::to_value(&archived).unwrap()
    );
    let queried = restored
        .query_completed_instances(engine_core::persistence::CompletedInstanceQuery {
            definition_key: Some(def_key),
            state_filter: Some("completed".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(queried.iter().any(|i| i.id == archived.id));
    let archive_detail = restored
        .get_bucket_entry_detail("history_instances", &archived.id.to_string())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&archive_detail.data).unwrap(),
        serde_json::to_value(&archived).unwrap()
    );
    restored
        .delete_instance(&legacy_instance.id.to_string())
        .await
        .unwrap();
    restored
        .kv("history_instances")
        .await
        .unwrap()
        .delete(archived.id.to_string())
        .await
        .unwrap();

    // After removing the archive, a repeated active deletion removes audit versions too.
    restored
        .delete_instance(&serialized_instance.id.to_string())
        .await
        .unwrap();
    let audit_key = crate::codec::audit_key(
        serialized_instance.id,
        &serde_json::to_vec(&serialized_instance.audit_log).unwrap(),
    );
    assert!(
        restored
            .kv("instance_audit")
            .await
            .unwrap()
            .get(&audit_key)
            .await
            .unwrap()
            .is_none()
    );

    // Recognized unsupported records must fail restore rather than be skipped.
    let invalid_key = Uuid::new_v4().to_string();
    store
        .put(&invalid_key, b"\x89BPMNI\xff".to_vec().into())
        .await
        .unwrap();
    let invalid_restore = restored.list_instances().await;
    store.delete(&invalid_key).await.unwrap();
    assert!(
        invalid_restore
            .unwrap_err()
            .to_string()
            .contains("unsupported")
    );

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

/// Batched history appends are acknowledged, kept in order, and a retried
/// batch does not duplicate entries (`Nats-Msg-Id` = entry id).
#[tokio::test]
async fn test_history_batch_is_ordered_and_deduplicated() {
    let persistence = match setup_nats_test().await {
        Some(p) => p,
        None => return,
    };

    let instance_id = Uuid::new_v4();
    let entries: Vec<_> = (0..3)
        .map(|i| {
            engine_core::history::HistoryEntry::new(
                instance_id,
                engine_core::history::HistoryEventType::TokenAdvanced,
                format!("step {i}"),
                engine_core::history::ActorType::Engine,
                None,
            )
        })
        .collect();

    persistence
        .append_history_entries(&entries)
        .await
        .expect("batch must be acknowledged");
    // Simulated retry of the same batch.
    persistence
        .append_history_entries(&entries)
        .await
        .expect("retried batch must be acknowledged");

    let loaded = persistence
        .query_history(engine_core::persistence::HistoryQuery {
            instance_id,
            ..Default::default()
        })
        .await
        .unwrap();
    let ids: Vec<_> = loaded.iter().map(|e| e.id).collect();
    let expected: Vec<_> = entries.iter().map(|e| e.id).collect();
    assert_eq!(ids, expected, "entries must be stored once, in order");
}

/// Checkpoints/deltas are immutable, bounded, and readable without the write feature.
#[tokio::test]
async fn test_instance_journal_roundtrip_and_checkpointing() {
    use engine_core::{
        WorkflowEngine,
        domain::{BpmnElement, ProcessDefinitionBuilder},
        runtime::InstanceState,
    };
    let Some(persistence) = setup_nats_test().await else {
        return;
    };
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("journal_fixture")
        .node("start", BpmnElement::StartEvent)
        .node("wait", BpmnElement::UserTask("tester".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "wait")
        .flow("wait", "end")
        .build()
        .unwrap();
    let (definition, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(definition).await.unwrap();
    let mut instance = engine.get_instance_details(id).await.unwrap();
    instance
        .variables
        .insert("counter".into(), serde_json::json!(0));
    instance.variables.insert(
        "large_unchanged_value".into(),
        serde_json::json!("x".repeat(16 * 1024)),
    );
    let snapshot = serde_json::to_vec(&instance).unwrap();
    let (checkpoint, depth) = crate::journal::choose_node(&snapshot, None).unwrap();
    let head = crate::journal::Head {
        id,
        key: crate::journal::key(id, &checkpoint),
        depth,
    };
    let head_bytes = crate::journal::encode_head(&head).unwrap();
    let nodes = persistence.kv("instance_states").await.unwrap();
    let instances = persistence.kv("instances").await.unwrap();
    nodes
        .create(&head.key, checkpoint.clone().into())
        .await
        .unwrap();
    instances
        .put(id.to_string(), head_bytes.clone().into())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(
            persistence
                .restore_instance_record(&head_bytes)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&instance).unwrap()
    );
    let listed = persistence.list_instances().await.unwrap();
    assert!(
        listed
            .iter()
            .any(|found| found.id == id && found.variables == instance.variables)
    );
    let detail = persistence
        .get_bucket_entry_detail("instances", &id.to_string())
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&detail.data).unwrap(),
        serde_json::to_value(&instance).unwrap()
    );

    // A journal write interrupted before publishing the head leaves the old state readable.
    let mut pending = instance.clone();
    pending
        .variables
        .insert("counter".into(), serde_json::json!(99));
    let (pending_node, _) =
        crate::journal::choose_node(&serde_json::to_vec(&pending).unwrap(), None).unwrap();
    nodes
        .create(crate::journal::key(id, &pending_node), pending_node.into())
        .await
        .unwrap();
    assert_eq!(
        persistence
            .restore_instance_record(&head_bytes)
            .await
            .unwrap()
            .variables["counter"],
        serde_json::json!(0)
    );
    // Missing/corrupt nodes and inconsistent depth fail restore.
    nodes.purge(&head.key).await.unwrap();
    assert!(
        persistence
            .restore_instance_record(&head_bytes)
            .await
            .unwrap_err()
            .to_string()
            .contains("missing journal")
    );
    nodes
        .create(&head.key, b"BC1\0corrupt".to_vec().into())
        .await
        .unwrap();
    assert!(
        persistence
            .restore_instance_record(&head_bytes)
            .await
            .unwrap_err()
            .to_string()
            .contains("digest mismatch")
    );
    nodes.purge(&head.key).await.unwrap();
    nodes.create(&head.key, checkpoint.into()).await.unwrap();
    let invalid_depth = crate::journal::Head {
        depth: 1,
        ..head.clone()
    };
    assert!(
        persistence
            .resolve_snapshot(&crate::journal::encode_head(&invalid_depth).unwrap())
            .await
            .is_err()
    );

    persistence.save_instance(&instance).await.unwrap();
    #[cfg(feature = "delta-instances")]
    {
        for counter in 1..=u32::from(crate::journal::MAX_DEPTH) + 2 {
            instance
                .variables
                .insert("counter".into(), serde_json::json!(counter));
            persistence.save_instance(&instance).await.unwrap();
            let stored = instances.get(id.to_string()).await.unwrap().unwrap();
            let head = crate::journal::decode_head(&stored).unwrap();
            assert_eq!(
                u32::from(head.depth),
                counter % (u32::from(crate::journal::MAX_DEPTH) + 1)
            );
            let node = nodes.get(&head.key).await.unwrap().unwrap();
            if head.depth > 0 {
                assert!(node.len() < 4096, "small edit should store a small delta");
            }
            assert_eq!(
                serde_json::to_value(persistence.restore_instance_record(&stored).await.unwrap())
                    .unwrap(),
                serde_json::to_value(&instance).unwrap()
            );
        }
    }
    #[cfg(not(feature = "delta-instances"))]
    assert!(!crate::journal::is_head(
        &instances.get(id.to_string()).await.unwrap().unwrap()
    ));
    #[cfg(feature = "delta-instances")]
    {
        let prior = instances.get(id.to_string()).await.unwrap().unwrap();
        let prior_head = crate::journal::decode_head(&prior).unwrap();
        let base = persistence.resolve_snapshot(&prior).await.unwrap();
        let prior_counter = instance.variables["counter"].clone();
        instance.variables.insert(
            "counter".into(),
            serde_json::json!(u32::from(crate::journal::MAX_DEPTH) + 3),
        );
        let prepared = persistence
            .prepare_instance_payload(&instance, serde_json::to_vec(&instance).unwrap())
            .unwrap();
        let target = if crate::codec::is_prepared(&prepared) {
            crate::codec::split_prepared(&prepared).unwrap().0
        } else {
            prepared.as_slice()
        };
        let (pending_node, _) =
            crate::journal::choose_node(target, Some((&prior_head, &base))).unwrap();
        let pending_key = crate::journal::key(id, &pending_node);
        // A concurrent revision change makes the cached CAS fail after the node write.
        instances.put(id.to_string(), prior.clone()).await.unwrap();
        assert!(persistence.save_instance(&instance).await.is_err());
        assert_eq!(instances.get(id.to_string()).await.unwrap().unwrap(), prior);
        assert_eq!(
            persistence
                .restore_instance_record(&prior)
                .await
                .unwrap()
                .variables["counter"],
            prior_counter
        );
        let pending_revision = nodes.entry(&pending_key).await.unwrap().unwrap().revision;
        // Retry rereads the committed head and reuses the existing immutable node.
        persistence.save_instance(&instance).await.unwrap();
        assert_eq!(
            nodes.entry(&pending_key).await.unwrap().unwrap().revision,
            pending_revision
        );
        let committed = instances.get(id.to_string()).await.unwrap().unwrap();
        assert_eq!(
            crate::journal::decode_head(&committed).unwrap().key,
            pending_key
        );
        let detail = persistence
            .get_bucket_entry_detail("instance_states", &pending_key)
            .await
            .unwrap();
        assert_eq!(detail.encoding, "base64");
    }
    // Reconnect without any writer cache and reconstruct from persisted data.
    let fresh = NatsPersistence::connect(&nats_url(), &format!("TEST_JOURNAL_{}", Uuid::new_v4()))
        .await
        .unwrap();
    let restored = fresh
        .list_instances()
        .await
        .unwrap()
        .into_iter()
        .find(|found| found.id == id)
        .unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(&instance).unwrap()
    );
    let mut archived = instance.clone();
    archived.state = InstanceState::Completed;
    persistence
        .save_completed_instance(&archived)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(
            fresh
                .get_completed_instance(&id.to_string())
                .await
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(&archived).unwrap()
    );
    persistence.delete_instance(&id.to_string()).await.unwrap();
    assert!(
        nodes.get(&head.key).await.unwrap().is_some(),
        "archive must retain journal data"
    );
    persistence
        .kv("history_instances")
        .await
        .unwrap()
        .delete(id.to_string())
        .await
        .unwrap();
    persistence.delete_instance(&id.to_string()).await.unwrap();
    assert!(nodes.get(&head.key).await.unwrap().is_none());
    engine.shutdown().await;
}
