//! Split from `unit_tests.rs` (M5).

use super::super::*;
use super::helpers::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn audit_log_captures_all_steps() {
    let (engine, def_key) = setup_linear_engine().await;
    let inst_id = engine.start_instance(def_key).await.unwrap();
    complete_all_service_tasks(&engine, "worker", HashMap::new()).await;

    let task_id = engine
        .pending_user_tasks
        .iter()
        .map(|r| r.value().clone())
        .next()
        .unwrap()
        .task_id;
    engine
        .complete_user_task(task_id, HashMap::new())
        .await
        .unwrap();

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(log.len() >= 4);
    assert!(log[0].contains("started"));
    assert!(log.last().unwrap().contains("completed"));
}

#[tokio::test]
async fn test_delete_instance() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("test")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let instance_id = engine.start_instance(key).await.unwrap();
    assert_eq!(engine.instances.len().await, 1);

    engine.delete_instance(instance_id).await.unwrap();

    assert_eq!(engine.instances.len().await, 0);
    assert!(engine.get_instance_details(instance_id).await.is_err());
}

#[tokio::test]
async fn test_delete_definition_cascade() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("test")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let _id1 = engine.start_instance(key).await.unwrap();
    let _id2 = engine.start_instance(key).await.unwrap();

    let err = engine.delete_definition(key, false).await.unwrap_err();
    assert!(matches!(
        err,
        crate::domain::EngineError::DefinitionHasInstances(2)
    ));

    engine.delete_definition(key, true).await.unwrap();

    let stats = engine.get_stats().await;
    assert_eq!(stats.definitions_count, 0);
    assert_eq!(engine.instances.len().await, 0);
}

#[tokio::test]
async fn restore_instance_loads_from_persistence() {
    let engine = WorkflowEngine::new();

    // Deploy a definition so it exists
    let def = ProcessDefinitionBuilder::new("restore")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (def_key, _) = engine.deploy_definition(def).await;

    // Create a dummy instance
    let inst = ProcessInstance {
        id: Uuid::new_v4(),
        definition_key: def_key,
        business_key: "BK1".into(),
        parent_instance_id: None,
        state: InstanceState::Completed,
        current_node: "end".into(),
        audit_log: vec![],
        variables: std::collections::HashMap::new(),
        tokens: std::collections::HashMap::new(),
        active_tokens: vec![],
        join_barriers: std::collections::HashMap::new(),
        multi_instance_state: std::collections::HashMap::new(),
        compensation_log: Vec::new(),
        outstanding_calls: HashMap::new(),
        started_at: None,
        completed_at: None,
    };

    engine.restore_instance(inst.clone()).await;

    let loaded = engine.get_instance_details(inst.id).await.unwrap();
    assert_eq!(loaded.id, inst.id);
    assert_eq!(loaded.business_key, "BK1");
}

#[tokio::test]
async fn mutation_delete_instance_and_variables() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("del")
        .node("start", BpmnElement::StartEvent)
        .node("t1", BpmnElement::UserTask("a".into()))
        .node("t2", BpmnElement::UserTask("b".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "t1")
        .flow("t1", "t2")
        .flow("t2", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    // Check list definitions formatting
    let defs = engine.list_definitions().await;
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].1, "del");

    let inst_id = engine.start_instance(def_key).await.unwrap();

    // Variable Math check (verify += vs -= mutant in update_instance_variables)
    let mut vars = HashMap::new();
    vars.insert("val".into(), serde_json::Value::Number(10.into()));
    engine
        .update_instance_variables(inst_id, vars)
        .await
        .unwrap();

    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(
        details.variables.get("val").unwrap(),
        &serde_json::Value::Number(10.into())
    );
    assert_eq!(details.variables.len(), 1); // Test mutant missing logic

    // Test delete instance == vs != loop
    let mut vars2 = HashMap::new();
    vars2.insert("other".into(), serde_json::Value::Bool(true));
    engine
        .update_instance_variables(inst_id, vars2)
        .await
        .unwrap();
    let details2 = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(details2.variables.len(), 2);

    // Create side tasks
    let pending = engine.get_pending_user_tasks();
    assert_eq!(pending.len(), 1);

    engine.delete_instance(inst_id).await.unwrap();
    // After delete, list should be 0.
    assert!(engine.get_instance_state(inst_id).await.is_err());
}

#[tokio::test]
async fn in_memory_large_file_variables() {
    let engine = WorkflowEngine::with_in_memory_persistence();

    let def = ProcessDefinitionBuilder::new("large_file")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("worker".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    // Create a very large dummy payload (10 MB of zeros to simulate memory stress)
    // NOTE: In the real engine-server, the file goes to the persistence layer.
    // In engine-core tests, we can just insert the reference into variables and
    // also persist it explicitly to in-memory persistence.
    let large_payload = vec![0u8; 10 * 1024 * 1024];

    if let Some(p) = &engine.persistence {
        p.save_file("file:big_data", &large_payload).await.unwrap();
    }

    let file_ref = crate::domain::FileReference {
        object_key: "file:big_data".into(),
        filename: "big_data.bin".into(),
        mime_type: "application/octet-stream".into(),
        size_bytes: large_payload.len() as u64,
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    };

    // Inject it into task
    let tasks = engine.get_pending_user_tasks();
    let mut vars = std::collections::HashMap::new();
    vars.insert("my_file".into(), serde_json::to_value(&file_ref).unwrap());

    engine
        .complete_user_task(tasks[0].task_id, vars)
        .await
        .unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);

    // Validate we can download it back
    let v = inst.variables.get("my_file").unwrap();
    let f_ref: crate::domain::FileReference = serde_json::from_value(v.clone()).unwrap();
    if let Some(p) = &engine.persistence {
        let downloaded = p.load_file(&f_ref.object_key).await.unwrap();
        assert_eq!(downloaded.len(), 10 * 1024 * 1024);
    }
}

#[tokio::test]
async fn test_definition_versioning_and_migration() {
    let engine = WorkflowEngine::with_in_memory_persistence();

    // V1 Definition
    let def_v1 = ProcessDefinitionBuilder::new("my_process")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("worker".into()))
        .node("end1", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end1")
        .build()
        .unwrap();

    let (key_v1, _) = engine.deploy_definition(def_v1).await;
    let def_v1_deployed = engine.definitions.get(&key_v1).unwrap();
    assert_eq!(def_v1_deployed.version, 1);

    // Start instance on V1
    let inst_v1 = engine.start_instance(key_v1).await.unwrap();

    // V2 Definition (Same ID, changed structure)
    let def_v2 = ProcessDefinitionBuilder::new("my_process")
        .node("start", BpmnElement::StartEvent)
        .node("task2", BpmnElement::UserTask("worker2".into())) // Changed ID
        .node("end2", BpmnElement::EndEvent)
        .flow("start", "task2")
        .flow("task2", "end2")
        .build()
        .unwrap();

    let (key_v2, _) = engine.deploy_definition(def_v2).await;
    let def_v2_deployed = engine.definitions.get(&key_v2).unwrap();

    // Key should be different, version should be bumped
    assert_ne!(key_v1, key_v2);
    assert_eq!(def_v2_deployed.version, 2);

    // Instance v1 should still be on 'task' safely.
    let inst_v1_data = engine.get_instance_details(inst_v1).await.unwrap();
    assert_eq!(inst_v1_data.current_node, "task");
    assert_eq!(inst_v1_data.definition_key, key_v1);

    // Start instance on V2
    let inst_v2 = engine.start_instance(key_v2).await.unwrap();
    let inst_v2_data = engine.get_instance_details(inst_v2).await.unwrap();
    assert_eq!(inst_v2_data.current_node, "task2");
    assert_eq!(inst_v2_data.definition_key, key_v2);
}

#[tokio::test]
async fn restore_timer_and_message_catch() {
    let engine = WorkflowEngine::new();

    let timer = PendingTimer {
        id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        node_id: "timer_1".into(),
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
        token_id: Uuid::new_v4(),
        timer_def: None,
        remaining_repetitions: None,
    };
    engine.restore_timer(timer.clone());
    assert_eq!(engine.pending_timers.len(), 1);
    assert_eq!(
        engine
            .pending_timers
            .iter()
            .map(|r| r.value().clone())
            .next()
            .unwrap()
            .id,
        timer.id
    );

    let catch = PendingMessageCatch {
        id: Uuid::new_v4(),
        instance_id: Uuid::new_v4(),
        node_id: "msg_1".into(),
        message_name: "ORDER_RECEIVED".into(),
        token_id: Uuid::new_v4(),
    };
    engine.restore_message_catch(catch.clone());
    assert_eq!(engine.pending_message_catches.len(), 1);
    assert_eq!(
        engine
            .pending_message_catches
            .iter()
            .map(|r| r.value().clone())
            .next()
            .unwrap()
            .id,
        catch.id
    );
}

#[test]
fn engine_is_send_and_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<super::super::WorkflowEngine>();
    assert_sync::<super::super::WorkflowEngine>();
}

// ============================================================================
// Mutation-Score Improvement Tests
// ============================================================================

#[tokio::test]
async fn test_get_stats_counts_correctly() {
    // Catches: replace += with -=, *= in get_stats; delete match arms
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("stats")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // Start 3 instances — all should be waiting on user task
    let _id1 = engine.start_instance(key).await.unwrap();
    let _id2 = engine.start_instance(key).await.unwrap();
    let _id3 = engine.start_instance(key).await.unwrap();

    let stats = engine.get_stats().await;
    assert_eq!(stats.definitions_count, 1);
    assert_eq!(stats.instances_waiting_user, 3);
    assert_eq!(stats.instances_running, 0);
    assert_eq!(stats.instances_completed, 0);
    assert_eq!(stats.instances_total, 3);
}

#[tokio::test]
async fn test_suspend_and_resume_instance() {
    // Catches: delete match arms in suspend/resume_instance
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("susp")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Suspend
    let result = engine.suspend_instance(inst_id).await;
    assert!(result.is_ok());
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Suspended { .. }));

    // Double suspend should fail
    let result = engine.suspend_instance(inst_id).await;
    assert!(result.is_err());

    // Resume
    let result = engine.resume_instance(inst_id).await;
    assert!(result.is_ok());
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(
        inst.state,
        InstanceState::WaitingOnUserTask { .. }
    ));

    // Double resume should fail
    let result = engine.resume_instance(inst_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_suspend_completed_instance_fails() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("done")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Instance is completed — suspend should fail
    let result = engine.suspend_instance(inst_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_instances_returns_all() {
    // Catches: replace list_instances -> Vec<ProcessInstance> with vec![]
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("list")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    engine.start_instance(key).await.unwrap();
    engine.start_instance(key).await.unwrap();

    let instances = engine.list_instances().await;
    assert_eq!(instances.len(), 2);
}

#[tokio::test]
async fn test_update_instance_variables() {
    // Catches: replace += with -=, *= in update_instance_variables
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("vars")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let mut new_vars = HashMap::new();
    new_vars.insert("x".to_string(), serde_json::json!(42));
    new_vars.insert("name".to_string(), serde_json::json!("test"));
    let result = engine.update_instance_variables(inst_id, new_vars).await;
    assert!(result.is_ok());

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.variables.get("x"), Some(&serde_json::json!(42)));
    assert_eq!(inst.variables.get("name"), Some(&serde_json::json!("test")));
}

#[tokio::test]
async fn test_registry_find_by_bpmn_id() {
    // Catches: replace find_by_bpmn_id -> Option with None; replace == with !=
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("myproc")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // find_by_bpmn_id should find it
    let found = engine.definitions.find_by_bpmn_id("myproc");
    assert!(found.is_some());
    assert_eq!(found.unwrap().0, key);

    // Non-existent should return None
    let not_found = engine.definitions.find_by_bpmn_id("nope");
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_registry_find_latest_and_versions() {
    // Catches: replace find_latest_by_bpmn_id -> Option with None;
    //          replace all_versions_of -> Vec with vec![]
    let engine = WorkflowEngine::new();
    let def1 = ProcessDefinitionBuilder::new("versioned")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key1, _) = engine.deploy_definition(def1).await;

    let def2 = ProcessDefinitionBuilder::new("versioned")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key2, _) = engine.deploy_definition(def2).await;

    // Latest should be v2
    let latest = engine.definitions.find_latest_by_bpmn_id("versioned");
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().0, key2);
    assert_ne!(key1, key2);

    // All versions
    let versions = engine.definitions.all_versions_of("versioned");
    assert_eq!(versions.len(), 2);

    // Registry stats
    assert!(!engine.definitions.is_empty());
    assert_eq!(engine.definitions.len(), 2);
    assert!(engine.definitions.contains_key(&key1));
}

#[tokio::test]
async fn test_move_token_to_valid_node() {
    // Catches: delete ! in move_token; replace == with != for node existence checks
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("move")
        .node("start", BpmnElement::StartEvent)
        .node("ut1", BpmnElement::UserTask("alice".into()))
        .node("ut2", BpmnElement::UserTask("bob".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut1")
        .flow("ut1", "ut2")
        .flow("ut2", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Should be at ut1
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.current_node, "ut1");

    // Move to ut2
    let result = engine
        .move_token(inst_id, "ut2", HashMap::new(), false)
        .await;
    assert!(result.is_ok());

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.current_node, "ut2");
}

#[tokio::test]
async fn test_move_token_invalid_node_fails() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("move_bad")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Move to non-existent node
    let result = engine
        .move_token(inst_id, "nonexistent", HashMap::new(), false)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_instance_cleans_all_queues() {
    // Catches: replace == with != in delete_instance queue cleanup
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("del_q")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "test_topic".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Service task should be pending
    assert!(!engine.pending_service_tasks.is_empty());

    // Delete the instance
    let result = engine.delete_instance(inst_id).await;
    assert!(result.is_ok());

    // Instance should be gone
    assert!(engine.get_instance_details(inst_id).await.is_err());
    // Pending service tasks for this instance should be cleaned
    let remaining: Vec<_> = engine
        .pending_service_tasks
        .iter()
        .filter(|t| t.instance_id == inst_id)
        .collect();
    assert!(remaining.is_empty());
}

// ============================================================================
// Gezielte Mutation-Score Tests
// ============================================================================

/// Catches: get_stats delete match arms, replace += with -=/×=
/// Erzeugt Instanzen in ALLEN Zuständen und prüft jeden Zähler exakt.
#[tokio::test]
async fn test_get_stats_all_state_categories() {
    let engine = WorkflowEngine::new();

    // User-Task Def (waiting_user)
    let user_def = ProcessDefinitionBuilder::new("s_user")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("a".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (uk, _) = engine.deploy_definition(user_def).await;

    // Service-Task Def (waiting_service)
    let svc_def = ProcessDefinitionBuilder::new("s_svc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "stats_topic".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .build()
        .unwrap();
    let (sk, _) = engine.deploy_definition(svc_def).await;

    // Completed Def
    let done_def = ProcessDefinitionBuilder::new("s_done")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (dk, _) = engine.deploy_definition(done_def).await;

    // CompletedWithError Def
    let err_def = ProcessDefinitionBuilder::new("s_err")
        .node("start", BpmnElement::StartEvent)
        .node(
            "err_end",
            BpmnElement::ErrorEndEvent {
                error_code: "E1".into(),
            },
        )
        .flow("start", "err_end")
        .build()
        .unwrap();
    let (ek, _) = engine.deploy_definition(err_def).await;

    // 2 waiting_user
    engine.start_instance(uk).await.unwrap();
    engine.start_instance(uk).await.unwrap();

    // 1 waiting_service
    engine.start_instance(sk).await.unwrap();

    // 1 completed
    engine.start_instance(dk).await.unwrap();

    // 1 completed_with_error
    engine.start_instance(ek).await.unwrap();

    let stats = engine.get_stats().await;
    assert_eq!(stats.definitions_count, 4);
    assert_eq!(stats.instances_total, 5);
    assert_eq!(stats.instances_waiting_user, 2);
    assert_eq!(stats.instances_waiting_service, 1);
    assert_eq!(stats.instances_completed, 2); // 1 Completed + 1 CompletedWithError
    assert_eq!(stats.instances_running, 0);
    assert_eq!(stats.pending_user_tasks, 2);
    assert_eq!(stats.pending_service_tasks, 1);
}

/// Catches: update_instance_variables += counters (added, modified, deleted)
/// Prüft Audit-Log-Text für korrekte Zählung.
#[tokio::test]
async fn test_update_instance_variables_counts_added_modified_deleted() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("var_count")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("a".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Step 1: Add 2 variables
    let mut vars = HashMap::new();
    vars.insert("a".into(), serde_json::json!(1));
    vars.insert("b".into(), serde_json::json!(2));
    engine
        .update_instance_variables(inst_id, vars)
        .await
        .unwrap();

    let log1 = engine.get_audit_log(inst_id).await.unwrap();
    assert!(
        log1.iter()
            .any(|l| l.contains("+2") && l.contains("~0") && l.contains("-0")),
        "Expected +2 ~0 -0 but got: {log1:?}"
    );

    // Step 2: Modify 1, add 1
    let mut vars2 = HashMap::new();
    vars2.insert("a".into(), serde_json::json!(99)); // modify
    vars2.insert("c".into(), serde_json::json!(3)); // add
    engine
        .update_instance_variables(inst_id, vars2)
        .await
        .unwrap();

    let log2 = engine.get_audit_log(inst_id).await.unwrap();
    assert!(
        log2.iter()
            .any(|l| l.contains("+1") && l.contains("~1") && l.contains("-0")),
        "Expected +1 ~1 -0 but got: {log2:?}"
    );

    // Step 3: Delete 1
    let mut vars3 = HashMap::new();
    vars3.insert("b".into(), Value::Null); // delete
    engine
        .update_instance_variables(inst_id, vars3)
        .await
        .unwrap();

    let log3 = engine.get_audit_log(inst_id).await.unwrap();
    assert!(
        log3.iter()
            .any(|l| l.contains("+0") && l.contains("~0") && l.contains("-1")),
        "Expected +0 ~0 -1 but got: {log3:?}"
    );

    // Verify final state
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.variables.get("a"), Some(&serde_json::json!(99)));
    assert!(!inst.variables.contains_key("b"));
    assert_eq!(inst.variables.get("c"), Some(&serde_json::json!(3)));
}

/// Catches: delete_instance == vs != in retain-Prädikaten
/// Zwei Instanzen mit pending tasks — nur die gelöschte wird bereinigt.
#[tokio::test]
async fn test_delete_instance_only_affects_target() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("del_iso")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("a".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    let inst_a = engine.start_instance(key).await.unwrap();
    let inst_b = engine.start_instance(key).await.unwrap();

    assert_eq!(engine.pending_user_tasks.len(), 2);

    // Delete only inst_a
    engine.delete_instance(inst_a).await.unwrap();

    // inst_b tasks should remain, inst_a tasks gone
    assert_eq!(engine.pending_user_tasks.len(), 1);
    let remaining: Vec<_> = engine
        .pending_user_tasks
        .iter()
        .map(|r| r.value().instance_id)
        .collect();
    assert_eq!(remaining, vec![inst_b]);
    assert!(engine.get_instance_details(inst_b).await.is_ok());
}

/// Catches: move_token on completed/suspended instances, cancel_current == vs != in filter
#[tokio::test]
async fn test_move_token_rejected_for_completed_and_suspended() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("mv_rej")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("a".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // Completed instance → move_token should fail
    let done_def = ProcessDefinitionBuilder::new("mv_done")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (dk, _) = engine.deploy_definition(done_def).await;
    let done_id = engine.start_instance(dk).await.unwrap();
    let res = engine
        .move_token(done_id, "end", HashMap::new(), false)
        .await;
    assert!(matches!(res, Err(EngineError::AlreadyCompleted)));

    // Suspended instance → move_token should fail
    let susp_id = engine.start_instance(key).await.unwrap();
    engine.suspend_instance(susp_id).await.unwrap();
    let res = engine
        .move_token(susp_id, "ut", HashMap::new(), false)
        .await;
    assert!(matches!(res, Err(EngineError::InstanceSuspended(_))));
}

/// Catches: move_token cancel_current=true cleans all queue types
#[tokio::test]
async fn test_move_token_cancel_current_cleans_queues() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("mv_cancel")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "mv_topic".into(),
                multi_instance: None,
            },
        )
        .node("ut", BpmnElement::UserTask("a".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Should have 1 pending service task
    assert_eq!(engine.pending_service_tasks.len(), 1);

    // Move with cancel_current=true to "ut"
    engine
        .move_token(inst_id, "ut", HashMap::new(), true)
        .await
        .unwrap();

    // Service task should be cleaned
    let remaining_svc: Vec<_> = engine
        .pending_service_tasks
        .iter()
        .filter(|t| t.instance_id == inst_id)
        .collect();
    assert!(
        remaining_svc.is_empty(),
        "cancel_current should clean pending service tasks"
    );

    // Should now be waiting on user task
    assert!(matches!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::WaitingOnUserTask { .. }
    ));
}

/// Catches: get_definition -> None; list_definition_versions -> vec![]
#[tokio::test]
async fn test_get_definition_and_list_versions() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("def_ops")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // get_definition should return the definition
    let got = engine.get_definition(&key).await;
    assert!(got.is_some());
    assert_eq!(got.unwrap().id, "def_ops");

    // Non-existent key
    assert!(engine.get_definition(&Uuid::new_v4()).await.is_none());

    // Deploy v2
    let def2 = ProcessDefinitionBuilder::new("def_ops")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("a".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .build()
        .unwrap();
    let (key2, _) = engine.deploy_definition(def2).await;

    // list_definition_versions should return both
    let versions = engine.list_definition_versions("def_ops").await;
    assert_eq!(versions.len(), 2);
    // Sorted ascending, v1 first
    assert_eq!(versions[0].0, key);
    assert_eq!(versions[0].1, 1); // version
    assert_eq!(versions[1].0, key2);
    assert_eq!(versions[1].1, 2);
    // Node count must be correct
    assert_eq!(versions[0].2, 2); // start + end
    assert_eq!(versions[1].2, 3); // start + task + end

    // Non-existent BPMN ID
    let empty = engine.list_definition_versions("nope").await;
    assert!(empty.is_empty());
}

/// Catches: retry_incident > vs >= und resolve_incident > vs >=
#[tokio::test]
async fn test_retry_incident_and_resolve_incident() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("incident")
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
    let (key, _) = engine.deploy_definition(def).await;
    engine.start_instance(key).await.unwrap();

    let tasks = engine
        .fetch_and_lock_service_tasks("w", 1, &["inc_topic".into()], 60)
        .await;
    let task_id = tasks[0].id;

    // Fail to 0 retries → incident
    engine
        .fail_service_task(task_id, "w", Some(0), Some("Boom".into()), None)
        .await
        .unwrap();

    // retry_incident on non-incident should fail
    // First, check retries > 0 guard — retry when already retries == 0 should succeed
    engine.retry_incident(task_id, Some(2)).await.unwrap();

    let t = engine
        .get_pending_service_tasks()
        .iter()
        .find(|t| t.id == task_id)
        .cloned()
        .unwrap();
    assert_eq!(t.retries, 2);
    assert!(t.error_message.is_none());
    assert!(t.worker_id.is_none());

    // retry_incident on task with retries > 0 should fail
    let res = engine.retry_incident(task_id, None).await;
    assert!(res.is_err());

    // Fail to 0 again for resolve test
    let _ = engine
        .fetch_and_lock_service_tasks("w2", 1, &["inc_topic".into()], 60)
        .await;
    engine
        .fail_service_task(task_id, "w2", Some(0), Some("Again".into()), None)
        .await
        .unwrap();

    // resolve_incident on task with retries > 0 should fail
    // (Already at 0, so this should succeed)
    let mut resolve_vars = HashMap::new();
    resolve_vars.insert("resolved".into(), serde_json::json!(true));
    engine
        .resolve_incident(task_id, resolve_vars)
        .await
        .unwrap();

    // Instance should complete (resolve advances token)
    let inst_id = tasks[0].instance_id;
    let state = engine.get_instance_state(inst_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);
}

/// Catches: InstanceStore is_empty, clear
#[tokio::test]
async fn test_instance_store_is_empty_and_clear() {
    let store = crate::engine::instance_store::InstanceStore::new();
    assert!(store.is_empty().await);

    let id = Uuid::new_v4();
    let inst = ProcessInstance {
        id,
        definition_key: Uuid::new_v4(),
        business_key: String::new(),
        parent_instance_id: None,
        state: InstanceState::Running,
        current_node: "start".into(),
        audit_log: vec![],
        variables: HashMap::new(),
        tokens: HashMap::new(),
        active_tokens: vec![],
        join_barriers: HashMap::new(),
        multi_instance_state: HashMap::new(),
        compensation_log: Vec::new(),
        outstanding_calls: HashMap::new(),
        started_at: None,
        completed_at: None,
    };
    store.insert(id, inst).await;
    assert!(!store.is_empty().await);
    assert_eq!(store.len().await, 1);

    store.clear().await;
    assert!(store.is_empty().await);
    assert_eq!(store.len().await, 0);
}

/// Catches: DefinitionRegistry contains_key -> true, is_empty -> false
#[tokio::test]
async fn test_registry_is_empty_and_contains_key() {
    let reg = crate::engine::registry::DefinitionRegistry::new();
    assert!(reg.is_empty());
    assert!(!reg.contains_key(&Uuid::new_v4()));

    let key = Uuid::new_v4();
    let def = ProcessDefinitionBuilder::new("reg_test")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    reg.insert(key, std::sync::Arc::new(def));

    assert!(!reg.is_empty());
    assert!(reg.contains_key(&key));
    assert!(!reg.contains_key(&Uuid::new_v4()));
}

/// Catches: replace restore_user_task with ()
/// Catches: replace restore_service_task with ()
#[tokio::test]
async fn test_restore_user_and_service_tasks() {
    let engine = WorkflowEngine::new();
    let task_id = uuid::Uuid::new_v4();
    let inst_id = uuid::Uuid::new_v4();

    // Restore a user task
    let pending_user = crate::runtime::PendingUserTask {
        task_id,
        instance_id: inst_id,
        node_id: "ut".into(),
        assignee: "alice".into(),
        token_id: uuid::Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        business_key: None,
    };
    engine.restore_user_task(pending_user);
    assert_eq!(engine.pending_user_tasks.len(), 1);
    assert!(engine.pending_user_tasks.contains_key(&task_id));

    // Restore a service task
    let svc_id = uuid::Uuid::new_v4();
    let pending_svc = crate::runtime::PendingServiceTask {
        id: svc_id,
        instance_id: inst_id,
        definition_key: uuid::Uuid::new_v4(),
        node_id: "svc".into(),
        topic: "validate".into(),
        token_id: uuid::Uuid::new_v4(),
        variables_snapshot: HashMap::new(),
        created_at: chrono::Utc::now(),
        worker_id: None,
        lock_expiration: None,
        retries: 3,
        error_message: None,
        error_details: None,
        business_key: None,
    };
    engine.restore_service_task(pending_svc);
    assert_eq!(engine.pending_service_tasks.len(), 1);
    assert!(engine.pending_service_tasks.contains_key(&svc_id));
}

/// Catches: replace restore_timer with ()
/// Catches: replace restore_message_catch with ()
#[tokio::test]
async fn test_restore_timer_and_message_catch() {
    let engine = WorkflowEngine::new();
    let inst_id = uuid::Uuid::new_v4();

    let timer_id = uuid::Uuid::new_v4();
    let pending_timer = crate::runtime::PendingTimer {
        id: timer_id,
        instance_id: inst_id,
        node_id: "timer".into(),
        token_id: uuid::Uuid::new_v4(),
        expires_at: chrono::Utc::now(),
        timer_def: None,
        remaining_repetitions: None,
    };
    engine.restore_timer(pending_timer);
    assert_eq!(engine.pending_timers.len(), 1);
    assert!(engine.pending_timers.contains_key(&timer_id));

    let msg_id = uuid::Uuid::new_v4();
    let pending_msg = crate::runtime::PendingMessageCatch {
        id: msg_id,
        instance_id: inst_id,
        node_id: "msg".into(),
        message_name: "order".into(),
        token_id: uuid::Uuid::new_v4(),
    };
    engine.restore_message_catch(pending_msg);
    assert_eq!(engine.pending_message_catches.len(), 1);
    assert!(engine.pending_message_catches.contains_key(&msg_id));
}

/// Catches: replace shutdown with ()
#[tokio::test]
async fn test_shutdown_completes_without_panic() {
    let engine = WorkflowEngine::with_in_memory_persistence();
    // Deploy and start something so the retry worker is active
    let def = ProcessDefinitionBuilder::new("shutdown_test")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let _ = engine.start_instance(key).await.unwrap();

    // Shutdown should signal retry worker and wait
    engine.shutdown().await;

    // After shutdown, engine should still be usable (no panics)
    assert!(
        engine
            .get_instance_details(uuid::Uuid::new_v4())
            .await
            .is_err()
    );
}

/// Catches: with_persistence activates retry_tx
#[tokio::test]
async fn test_set_persistence_activates_retry_tx() {
    let engine = WorkflowEngine::new();
    assert!(
        engine.retry_tx.is_none(),
        "No retry_tx before with_persistence"
    );

    let persistence = std::sync::Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = engine.with_persistence(persistence);

    assert!(
        engine.retry_tx.is_some(),
        "retry_tx should be set after with_persistence"
    );
}

/// Catches: replace restore_instance with () (the instance must be accessible)
#[tokio::test]
async fn test_restore_instance_makes_it_accessible() {
    let engine = WorkflowEngine::new();
    let inst_id = uuid::Uuid::new_v4();
    let def_key = uuid::Uuid::new_v4();

    let instance = crate::runtime::ProcessInstance {
        id: inst_id,
        definition_key: def_key,
        business_key: "restored".into(),
        parent_instance_id: None,
        state: InstanceState::Running,
        current_node: "start".into(),
        audit_log: vec![],
        variables: HashMap::new(),
        tokens: HashMap::new(),
        active_tokens: vec![],
        join_barriers: HashMap::new(),
        multi_instance_state: HashMap::new(),
        compensation_log: Vec::new(),
        outstanding_calls: HashMap::new(),
        started_at: None,
        completed_at: None,
    };
    engine.restore_instance(instance).await;

    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(details.business_key, "restored");
}
