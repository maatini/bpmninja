//! Split from `unit_tests.rs` (M5).

use super::super::*;
use super::helpers::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn conditional_routing_on_service_task() {
    let engine = WorkflowEngine::new();

    let def = ProcessDefinitionBuilder::new("cond_svc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node("end_a", BpmnElement::EndEvent)
        .node("end_b", BpmnElement::EndEvent)
        .flow("start", "svc")
        .conditional_flow("svc", "end_a", "x == 1")
        .conditional_flow("svc", "end_b", "x == 2")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    let mut vars = HashMap::new();
    vars.insert("x".into(), Value::Number(2.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    let log = engine.get_audit_log(inst_id).await.unwrap();
    let end_entry = log
        .iter()
        .find(|l| l.contains("Process completed"))
        .unwrap();
    assert!(
        end_entry.contains("end_b"),
        "Expected end_b path: {end_entry}"
    );
}

#[tokio::test]
async fn start_instance_pauses_at_user_task() {
    let (engine, def_key) = setup_linear_engine().await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::WaitingOnUserTask {
            task_id: engine
                .pending_user_tasks
                .iter()
                .map(|r| r.value().clone())
                .next()
                .unwrap()
                .task_id
        }
    );
    assert_eq!(engine.pending_user_tasks.len(), 1);
}

#[tokio::test]
async fn complete_user_task_reaches_end() {
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

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert!(engine.pending_user_tasks.is_empty());
}

#[tokio::test]
async fn completing_wrong_task_gives_error() {
    let (engine, def_key) = setup_linear_engine().await;
    engine.start_instance(def_key).await.unwrap();
    complete_all_service_tasks(&engine, "worker", HashMap::new()).await;

    let wrong_id = Uuid::new_v4();
    let result = engine.complete_user_task(wrong_id, HashMap::new()).await;
    assert!(matches!(result, Err(EngineError::TaskNotPending { .. })));
}

#[tokio::test]
async fn service_handler_modifies_variables() {
    let (engine, def_key) = setup_linear_engine().await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    let mut vars = HashMap::new();
    vars.insert("validated".into(), Value::Bool(true));
    complete_all_service_tasks(&engine, "worker_1", vars).await;

    // The token stored centrally should have 'validated: true' from the service handler
    let pending = engine
        .pending_user_tasks
        .iter()
        .map(|r| r.value().clone())
        .next()
        .unwrap();
    let inst_arc = engine.instances.get(&inst_id).await.unwrap();
    let inst = inst_arc.read().await;
    let token = inst.tokens.get(&pending.token_id).unwrap();
    assert_eq!(token.variables.get("validated"), Some(&Value::Bool(true)));
}

// -----------------------------------------------------------------------
// Service Task specific operations
// -----------------------------------------------------------------------

#[tokio::test]
async fn service_task_fail_and_retries() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("retries")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "fail_test".into(),
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

    // 1. Fetch task
    let tasks = engine
        .fetch_and_lock_service_tasks("worker", 1, &["fail_test".into()], 60)
        .await;
    assert_eq!(tasks.len(), 1);
    let task_id = tasks[0].id;

    // 2. Fail task (default 3 retries, decrementing to 2)
    engine
        .fail_service_task(task_id, "worker", None, Some("Failed".into()), None)
        .await
        .unwrap();

    // 3. Task should be unlocked and retries should be 2
    let pending = engine.get_pending_service_tasks();
    let t = pending.iter().find(|t| t.id == task_id).unwrap();
    assert_eq!(t.retries, 2);
    assert!(t.worker_id.is_none());

    // 4. Fail directly to 0
    let _ = engine
        .fetch_and_lock_service_tasks("worker2", 1, &["fail_test".into()], 60)
        .await;
    engine
        .fail_service_task(task_id, "worker2", Some(0), Some("Fatal".into()), None)
        .await
        .unwrap();

    // Incident should be logged
    let inst_id = tasks[0].instance_id;
    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("INCIDENT")));
    assert!(log.iter().any(|l| l.contains("Fatal")));
}

#[tokio::test]
async fn service_task_extend_lock() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("extend")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "ext".into(),
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
        .fetch_and_lock_service_tasks("worker", 1, &["ext".into()], 60)
        .await;
    let task_id = tasks[0].id;
    let exp_before = engine
        .get_pending_service_tasks()
        .iter()
        .find(|t| t.id == task_id)
        .unwrap()
        .lock_expiration
        .unwrap();

    engine.extend_lock(task_id, "worker", 120).await.unwrap();

    let exp_after = engine
        .get_pending_service_tasks()
        .iter()
        .find(|t| t.id == task_id)
        .unwrap()
        .lock_expiration
        .unwrap();
    assert!(exp_after > exp_before);
}

#[tokio::test]
async fn service_task_handle_bpmn_error() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("err")
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
    engine.start_instance(def_key).await.unwrap();

    let tasks = engine
        .fetch_and_lock_service_tasks("worker", 1, &["err".into()], 60)
        .await;

    assert_eq!(engine.get_pending_service_tasks().len(), 1);

    engine
        .handle_bpmn_error(tasks[0].id, "worker", "ERR_CODE")
        .await
        .unwrap();

    // Task remains as incident (retries exhausted).
    let pending = engine.get_pending_service_tasks();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].retries <= 0);

    let log = engine.get_audit_log(tasks[0].instance_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("ERR_CODE")));
}

#[tokio::test]
async fn mutation_fetch_service_task_boundary() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("lock")
        .node("start", BpmnElement::StartEvent)
        .node(
            "t1",
            BpmnElement::ServiceTask {
                topic: "bound".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "t1")
        .flow("t1", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    engine.start_instance(def_key).await.unwrap();

    // Fetch once
    let tasks1 = engine
        .fetch_and_lock_service_tasks("worker1", 1, &["bound".into()], 1)
        .await;
    assert_eq!(tasks1.len(), 1);

    // Fetch immediately again, should return 0 since locked and not expired
    let tasks2 = engine
        .fetch_and_lock_service_tasks("worker2", 1, &["bound".into()], 1)
        .await;
    assert_eq!(tasks2.len(), 0);

    // Sleep 1.1 second so it exceeds. `cargo mutants` tests > vs == on the `expiration > now`.
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;

    // Fetch again, should return 1 since lock expired
    let tasks3 = engine
        .fetch_and_lock_service_tasks("worker3", 1, &["bound".into()], 1)
        .await;
    assert_eq!(tasks3.len(), 1);
}

/// Catches: verify_lock_ownership None-Fall (ServiceTaskNotLocked)
#[tokio::test]
async fn test_service_task_not_locked_error() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("no_lock")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "nl".into(),
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

    // Try to complete without fetching (no lock)
    let task_id = engine.get_pending_service_tasks()[0].id;
    let res = engine
        .complete_service_task(task_id, "any_worker", HashMap::new())
        .await;
    assert!(matches!(res, Err(EngineError::ServiceTaskNotLocked(_))));
}

/// Catches: fetch_and_lock > vs >= für max_tasks-Grenze
#[tokio::test]
async fn test_fetch_and_lock_respects_max_tasks() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("max_fetch")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node(
            "svc1",
            BpmnElement::ServiceTask {
                topic: "mf".into(),
                multi_instance: None,
            },
        )
        .node(
            "svc2",
            BpmnElement::ServiceTask {
                topic: "mf".into(),
                multi_instance: None,
            },
        )
        .node(
            "svc3",
            BpmnElement::ServiceTask {
                topic: "mf".into(),
                multi_instance: None,
            },
        )
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("fork", "svc1")
        .flow("fork", "svc2")
        .flow("fork", "svc3")
        .flow("svc1", "join")
        .flow("svc2", "join")
        .flow("svc3", "join")
        .flow("join", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    engine.start_instance(key).await.unwrap();

    // 3 service tasks available, but max_tasks=2
    let tasks = engine
        .fetch_and_lock_service_tasks("w", 2, &["mf".into()], 60)
        .await;
    assert_eq!(tasks.len(), 2, "Should respect max_tasks limit");
}

/// Tests that verify_lock_ownership returns correct errors for all three cases.
#[tokio::test]
async fn test_complete_service_task_lock_ownership_variants() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("lock_variants")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "lock_test".into(),
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

    let task_id = engine.get_pending_service_tasks()[0].id;

    // Case 1: Not locked — should return ServiceTaskNotLocked
    let res = engine
        .complete_service_task(task_id, "any", HashMap::new())
        .await;
    assert!(
        matches!(res, Err(EngineError::ServiceTaskNotLocked(_))),
        "Expected ServiceTaskNotLocked, got: {:?}",
        res
    );

    // Lock it with worker "alpha"
    engine
        .fetch_and_lock_service_tasks("alpha", 1, &["lock_test".into()], 60000)
        .await;

    // Case 2: Wrong worker — should return ServiceTaskLocked
    let res = engine
        .complete_service_task(task_id, "beta", HashMap::new())
        .await;
    assert!(
        matches!(res, Err(EngineError::ServiceTaskLocked { .. })),
        "Expected ServiceTaskLocked, got: {:?}",
        res
    );

    // Case 3: Correct worker — should succeed
    let res = engine
        .complete_service_task(task_id, "alpha", HashMap::new())
        .await;
    assert!(res.is_ok(), "Expected Ok, got: {:?}", res);

    // Case 4: Already removed — should return ServiceTaskNotFound
    let res = engine
        .complete_service_task(task_id, "alpha", HashMap::new())
        .await;
    assert!(
        matches!(res, Err(EngineError::ServiceTaskNotFound(_))),
        "Expected ServiceTaskNotFound, got: {:?}",
        res
    );
}
