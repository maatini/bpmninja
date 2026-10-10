//! Split from `unit_tests.rs` (M5).

use super::super::*;
use super::helpers::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn timer_start_succeeds() {
    let engine = WorkflowEngine::new();
    let dur = Duration::from_secs(60);

    let def = ProcessDefinitionBuilder::new("timer_proc")
        .node(
            "ts",
            BpmnElement::TimerStartEvent(crate::domain::TimerDefinition::Duration(dur)),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("ts", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.trigger_timer_start(def_key, dur).await.unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn timer_mismatch_gives_error() {
    let engine = WorkflowEngine::new();

    let def = ProcessDefinitionBuilder::new("timer_proc")
        .node(
            "ts",
            BpmnElement::TimerStartEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_secs(60),
            )),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("ts", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let result = engine
        .trigger_timer_start(def_key, Duration::from_secs(30))
        .await;
    assert!(matches!(result, Err(EngineError::TimerMismatch { .. })));
}

#[tokio::test]
async fn plain_start_rejects_timer_def() {
    let engine = WorkflowEngine::new();

    let def = ProcessDefinitionBuilder::new("timer_proc")
        .node(
            "ts",
            BpmnElement::TimerStartEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_secs(5),
            )),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("ts", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let result = engine.start_instance(def_key).await;
    assert!(matches!(
        result,
        Err(EngineError::InvalidDefinition(msg)) if msg.contains("timer")
    ));
}

#[tokio::test]
async fn unknown_definition_gives_error() {
    let engine = WorkflowEngine::new();
    let result = engine.start_instance(Uuid::new_v4()).await;
    assert!(matches!(result, Err(EngineError::NoSuchDefinition(_))));
}

#[tokio::test]
async fn message_start_event_succeeds() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("msg_start")
        .node(
            "start",
            BpmnElement::MessageStartEvent {
                message_name: "start_msg".to_string(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();

    let _ = engine.deploy_definition(def).await;

    // Normal start should fail or wait if not message? Actually, correlate_message starts it
    let mut vars = HashMap::new();
    vars.insert("k".into(), serde_json::Value::String("v".into()));

    let affected = engine
        .correlate_message("start_msg".into(), Some("bk1".into()), vars)
        .await
        .unwrap();
    assert_eq!(affected.len(), 1);

    let inst_id = affected[0];
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.business_key, "bk1");
}

#[tokio::test]
async fn timer_catch_event_succeeds() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("timer_catch")
        .node("start", BpmnElement::StartEvent)
        .node(
            "timer",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                std::time::Duration::from_millis(50),
            )),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "timer")
        .flow("timer", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::WaitingOnTimer {
            timer_id: engine
                .pending_timers
                .iter()
                .map(|r| r.value().clone())
                .next()
                .unwrap()
                .id
        }
    );

    // Won't trigger immediately
    let triggered = engine.process_timers().await.unwrap();
    assert_eq!(triggered, 0);

    tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;

    let triggered = engine.process_timers().await.unwrap();
    assert_eq!(triggered, 1);

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn boundary_timer_event_cancels_task() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("bound_timer")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("assignee".into()))
        .node(
            "bound_timer",
            BpmnElement::BoundaryTimerEvent {
                attached_to: "task".into(),
                timer: crate::domain::TimerDefinition::Duration(std::time::Duration::from_millis(
                    50,
                )),
                cancel_activity: true,
            },
        )
        .node("end1", BpmnElement::EndEvent)
        .node("end2", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end1")
        .flow("bound_timer", "end2")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.pending_user_tasks.len(), 1);
    assert_eq!(engine.pending_timers.len(), 1);

    tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;
    let triggered = engine.process_timers().await.unwrap();
    assert_eq!(triggered, 1);

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.current_node, "end2");
}

// ---------------------------------------------------------------------------
// Advanced Edge Case Testing with InMemoryPersistence
// ---------------------------------------------------------------------------

#[tokio::test]
async fn in_memory_simultaneous_timer_and_message_race() {
    let engine = WorkflowEngine::with_in_memory_persistence();

    let def = ProcessDefinitionBuilder::new("race")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node(
            "timer",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                std::time::Duration::from_millis(50),
            )),
        )
        .node(
            "msg",
            BpmnElement::MessageCatchEvent {
                message_name: "MSG_CANCEL".into(),
            },
        )
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("fork", "timer")
        .flow("fork", "msg")
        .flow("timer", "join")
        .flow("msg", "join")
        .flow("join", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.pending_timers.len(), 1);
    assert_eq!(engine.pending_message_catches.len(), 1);

    // Simulate time passing (50ms) BUT before processing timers, we send the message!
    tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;

    // Race: The message arrives precisely when the timer is due.
    let msg_name = engine
        .pending_message_catches
        .iter()
        .map(|r| r.value().clone())
        .next()
        .unwrap()
        .message_name
        .clone();
    engine
        .correlate_message(msg_name, None, std::collections::HashMap::new())
        .await
        .unwrap();

    // Since message was processed first, the instance was routed to join, and blocked on parallel gate
    let _inst = engine.get_instance_details(inst_id).await.unwrap();
    // (Note: correlate_message blindly resets state to Running visually, but it's still waiting on the other parallel branch inside active_tokens)

    // Now if we process timers, it should trigger the timer and join to finish
    let triggered = engine.process_timers().await.unwrap();
    assert_eq!(triggered, 1);

    let inst2 = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst2.state, InstanceState::Completed);
}

#[tokio::test]
async fn event_based_gateway_timer_wins() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("ebg_timer")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::EventBasedGateway)
        .node(
            "catch_timer",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_millis(50),
            )),
        )
        .node(
            "catch_msg",
            BpmnElement::MessageCatchEvent {
                message_name: "win_msg".into(),
            },
        )
        .node("end_timer", BpmnElement::EndEvent)
        .node("end_msg", BpmnElement::EndEvent)
        .flow("start", "gw")
        .flow("gw", "catch_timer")
        .flow("gw", "catch_msg")
        .flow("catch_timer", "end_timer")
        .flow("catch_msg", "end_msg")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let instance_id = engine.start_instance(key).await.unwrap();

    let state = engine.get_instance_state(instance_id).await.unwrap();
    assert_eq!(state, InstanceState::WaitingOnEventBasedGateway);

    // ensure pending timers and messages are registered
    assert_eq!(engine.pending_timers.len(), 1);
    assert_eq!(engine.pending_message_catches.len(), 1);

    // Wait for timer to expire
    tokio::time::sleep(Duration::from_millis(60)).await;

    let processed = engine.process_timers().await.unwrap();
    assert_eq!(processed, 1);

    // The message catch should have been CANCELLED and removed!
    assert_eq!(engine.pending_timers.len(), 0);
    assert_eq!(engine.pending_message_catches.len(), 0);

    let state = engine.get_instance_state(instance_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);

    let log = engine.get_audit_log(instance_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("cancelled")));
    assert!(log.iter().any(|l| l.contains("'end_timer'")));
}

#[tokio::test]
async fn event_based_gateway_message_wins() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("ebg_msg")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::EventBasedGateway)
        .node(
            "catch_timer",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_millis(5000),
            )),
        ) // Long timer
        .node(
            "catch_msg",
            BpmnElement::MessageCatchEvent {
                message_name: "win_msg".into(),
            },
        )
        .node("end_timer", BpmnElement::EndEvent)
        .node("end_msg", BpmnElement::EndEvent)
        .flow("start", "gw")
        .flow("gw", "catch_timer")
        .flow("gw", "catch_msg")
        .flow("catch_timer", "end_timer")
        .flow("catch_msg", "end_msg")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let instance_id = engine.start_instance(key).await.unwrap();

    // Correlate message
    let affected = engine
        .correlate_message("win_msg".into(), None, Default::default())
        .await
        .unwrap();
    assert_eq!(affected.len(), 1);

    // The timer should have been CANCELLED and removed!
    assert_eq!(engine.pending_timers.len(), 0);
    assert_eq!(engine.pending_message_catches.len(), 0);

    let state = engine.get_instance_state(instance_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);

    let log = engine.get_audit_log(instance_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("cancelled")));
    assert!(log.iter().any(|l| l.contains("'end_msg'")));
}

#[tokio::test]
async fn test_non_interrupting_timer_boundary() {
    // A process with a user task that has a non-interrupting timer boundary.
    // The timer fires: the process forks to a second user task, while the first ONE is STILL alive!
    let eng = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("test_bnd")
        .node("start", BpmnElement::StartEvent)
        .flow("start", "task")
        .node("task", BpmnElement::UserTask("User1".into()))
        .node(
            "timer_bnd",
            BpmnElement::BoundaryTimerEvent {
                attached_to: "task".into(),
                timer: crate::domain::TimerDefinition::Duration(std::time::Duration::from_secs(1)),
                cancel_activity: false, // NON-INTERRUPTING
            },
        )
        // From timer -> goes to task2
        .flow("timer_bnd", "task2")
        .node("task2", BpmnElement::UserTask("User1".into()))
        .flow("task2", "end2")
        .node("end2", BpmnElement::EndEvent)
        // From main task -> goes to end1
        .flow("task", "end1")
        .node("end1", BpmnElement::EndEvent)
        .build()
        .unwrap();

    let (def_key, _) = eng.deploy_definition(def).await;
    let inst_id = eng
        .start_instance_with_variables(def_key, Default::default())
        .await
        .unwrap();

    // The user task should be pending
    let tasks = eng
        .get_pending_user_tasks()
        .into_iter()
        .filter(|t| t.instance_id == inst_id)
        .collect::<Vec<_>>();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].node_id, "task");

    // The timer should be pending
    let timers = eng
        .pending_timers
        .iter()
        .map(|r| r.value().clone())
        .collect::<Vec<_>>();
    assert_eq!(timers.len(), 1);

    // Simulate timer firing
    eng.set_timer_expiry(
        timers[0].id,
        chrono::Utc::now() - chrono::Duration::hours(1),
    );

    eng.process_timers().await.unwrap();

    // Now, there should be TWO user tasks pending: 'task' and 'task2'
    let tasks = eng
        .get_pending_user_tasks()
        .into_iter()
        .filter(|t| t.instance_id == inst_id)
        .collect::<Vec<_>>();
    let node_ids: std::collections::HashSet<_> = tasks.iter().map(|t| t.node_id.clone()).collect();
    assert_eq!(node_ids.len(), 2, "Expected both tasks to be pending");
    assert!(node_ids.contains("task"));
    assert!(node_ids.contains("task2"));

    // Instance state should be parallel
    {
        let inst_lk = eng.instances.get(&inst_id).await.unwrap();
        let inst = inst_lk.read().await;
        assert!(matches!(
            inst.state,
            crate::runtime::InstanceState::ParallelExecution {
                active_token_count: 2
            }
        ));
    }

    // Complete first task
    let task1_id = tasks.iter().find(|t| t.node_id == "task").unwrap().task_id;
    eng.complete_user_task(task1_id, Default::default())
        .await
        .unwrap();

    // Process still running (waiting on task2)
    {
        let inst_lk = eng.instances.get(&inst_id).await.unwrap();
        let inst = inst_lk.read().await;
        assert!(!matches!(
            inst.state,
            crate::runtime::InstanceState::Completed
        ));
    }

    // Complete second task
    let task2_id = tasks.iter().find(|t| t.node_id == "task2").unwrap().task_id;
    eng.complete_user_task(task2_id, Default::default())
        .await
        .unwrap();

    // Now completed
    {
        let inst_lk = eng.instances.get(&inst_id).await.unwrap();
        let inst = inst_lk.read().await;
        assert!(matches!(
            inst.state,
            crate::runtime::InstanceState::Completed
        ));
    }
}

#[tokio::test]
async fn test_interrupting_timer_boundary_cleanup() {
    let eng = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("test_bnd_2")
        .node("start", BpmnElement::StartEvent)
        .flow("start", "task")
        .node(
            "task",
            BpmnElement::ServiceTask {
                topic: "test".into(),
                multi_instance: None,
            },
        )
        .node(
            "timer_bnd",
            BpmnElement::BoundaryTimerEvent {
                attached_to: "task".into(),
                timer: crate::domain::TimerDefinition::Duration(std::time::Duration::from_secs(1)),
                cancel_activity: true, // INTERRUPTING
            },
        )
        .flow("timer_bnd", "end")
        .flow("task", "end")
        .node("end", BpmnElement::EndEvent)
        .build()
        .unwrap();

    let (def_key, _) = eng.deploy_definition(def).await;
    let inst_id = eng
        .start_instance_with_variables(def_key, Default::default())
        .await
        .unwrap();

    let service_tasks = eng
        .get_pending_service_tasks()
        .into_iter()
        .filter(|t| t.instance_id == inst_id)
        .collect::<Vec<_>>();
    assert_eq!(service_tasks.len(), 1);

    let timers = eng
        .pending_timers
        .iter()
        .map(|r| r.value().clone())
        .collect::<Vec<_>>();
    let tid = timers[0].id;
    eng.set_timer_expiry(tid, chrono::Utc::now() - chrono::Duration::hours(1));

    eng.process_timers().await.unwrap();

    // The service task should be DELETED, not just orphaned token
    let service_tasks_after = eng
        .get_pending_service_tasks()
        .into_iter()
        .filter(|t| t.instance_id == inst_id)
        .collect::<Vec<_>>();
    assert_eq!(
        service_tasks_after.len(),
        0,
        "Interrupting boundary event should delete the pending service task"
    );

    let inst_lk = eng.instances.get(&inst_id).await.unwrap();
    let inst = inst_lk.read().await;
    assert!(matches!(
        inst.state,
        crate::runtime::InstanceState::Completed
    ));
}

#[tokio::test]
async fn test_non_interrupting_message_boundary() {
    let eng = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("test_bnd_3")
        .node("start", BpmnElement::StartEvent)
        .flow("start", "task")
        .node("task", BpmnElement::UserTask("User1".into()))
        .node(
            "msg_bnd",
            BpmnElement::BoundaryMessageEvent {
                attached_to: "task".into(),
                message_name: "async_signal".into(),
                cancel_activity: false, // NON-INTERRUPTING
            },
        )
        .flow("msg_bnd", "end2")
        .node("end2", BpmnElement::EndEvent)
        .flow("task", "end1")
        .node("end1", BpmnElement::EndEvent)
        .build()
        .unwrap();

    let (def_key, _) = eng.deploy_definition(def).await;
    let inst_id = eng
        .start_instance_with_variables(def_key, Default::default())
        .await
        .unwrap();

    // Trigger message
    eng.correlate_message("async_signal".into(), None, Default::default())
        .await
        .unwrap();

    // The user task should STILL be pending!
    let tasks = eng
        .get_pending_user_tasks()
        .into_iter()
        .filter(|t| t.instance_id == inst_id)
        .collect::<Vec<_>>();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].node_id, "task");

    // Instance state should be parallel, but one branch (end2) just finished.
    // Wait, the message correlates, starts parallel branch, immediately hits EndEvent.
    // So the state parallel count decreased by 1 immediately.
    // Let's just check the instance is not fully completed.
    let inst_lk = eng.instances.get(&inst_id).await.unwrap();
    assert!(!matches!(
        inst_lk.read().await.state,
        crate::runtime::InstanceState::Completed
    ));

    eng.complete_user_task(tasks[0].task_id, Default::default())
        .await
        .unwrap();
    assert!(matches!(
        inst_lk.read().await.state,
        crate::runtime::InstanceState::Completed
    ));
}

/// Catches: correlate_message == vs != für business_key-Filter
#[tokio::test]
async fn test_correlate_message_with_business_key_filter() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("msg_bk")
        .node("start", BpmnElement::StartEvent)
        .node(
            "msg_catch",
            BpmnElement::MessageCatchEvent {
                message_name: "ORDER".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "msg_catch")
        .flow("msg_catch", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // Start instance with business_key = "BK-1"
    let inst_id = engine.start_instance(key).await.unwrap();
    {
        let inst_arc = engine.instances.get(&inst_id).await.unwrap();
        let mut inst = inst_arc.write().await;
        inst.business_key = "BK-1".into();
    }

    // Correlate with wrong business_key → should NOT match the catch
    let affected = engine
        .correlate_message("ORDER".into(), Some("BK-WRONG".into()), HashMap::new())
        .await
        .unwrap();
    assert!(affected.is_empty(), "Wrong business_key should not match");

    // Correlate with correct business_key → should match
    let affected = engine
        .correlate_message("ORDER".into(), Some("BK-1".into()), HashMap::new())
        .await
        .unwrap();
    assert_eq!(affected.len(), 1);
    assert_eq!(affected[0], inst_id);

    let state = engine.get_instance_state(inst_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);
}

/// Catches: process_timers > vs >= vs < für timer expiry check;
/// suspended instances should be skipped.
#[tokio::test]
async fn test_process_timers_skips_suspended() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("timer_susp")
        .node("start", BpmnElement::StartEvent)
        .node(
            "timer",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                std::time::Duration::from_millis(10),
            )),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "timer")
        .flow("timer", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Suspend instance before timer fires
    engine.suspend_instance(inst_id).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

    // Timer is expired but instance is suspended → should NOT fire
    let triggered = engine.process_timers().await.unwrap();
    assert_eq!(triggered, 0);

    // Resume → timer should now fire
    engine.resume_instance(inst_id).await.unwrap();
    let triggered = engine.process_timers().await.unwrap();
    assert_eq!(triggered, 1);

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

/// Catches: boundary.rs setup_boundary_events attached_to == vs !=
#[tokio::test]
async fn test_boundary_events_only_attach_to_correct_node() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("bnd_iso")
        .node("start", BpmnElement::StartEvent)
        .node("task_a", BpmnElement::UserTask("a".into()))
        .node("task_b", BpmnElement::UserTask("b".into()))
        .node(
            "timer_on_a",
            BpmnElement::BoundaryTimerEvent {
                attached_to: "task_a".into(),
                timer: crate::domain::TimerDefinition::Duration(Duration::from_secs(60)),
                cancel_activity: true,
            },
        )
        .node("end1", BpmnElement::EndEvent)
        .node("end2", BpmnElement::EndEvent)
        .node("end3", BpmnElement::EndEvent)
        .flow("start", "task_a")
        .flow("task_a", "task_b")
        .flow("task_b", "end1")
        .flow("timer_on_a", "end2")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Timer boundary on task_a → 1 timer pending
    assert_eq!(engine.pending_timers.len(), 1);

    // Complete task_a → boundary timer should be cancelled
    let task_a = engine
        .get_pending_user_tasks()
        .into_iter()
        .find(|t| t.node_id == "task_a")
        .unwrap();
    engine
        .complete_user_task(task_a.task_id, HashMap::new())
        .await
        .unwrap();

    // Timer should be gone (cancelled by cancel_boundary_timers)
    assert_eq!(engine.pending_timers.len(), 0);

    // Instance should now be at task_b
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.current_node, "task_b");
}

/// Catches: delete_instance mit allen Queue-Typen (user, service, timer, message)
#[tokio::test]
async fn test_delete_instance_cleans_timers_and_messages() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("del_all_q")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node("ut", BpmnElement::UserTask("a".into()))
        .node(
            "timer",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_secs(3600),
            )),
        )
        .node(
            "msg",
            BpmnElement::MessageCatchEvent {
                message_name: "DEL_MSG".into(),
            },
        )
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("fork", "ut")
        .flow("fork", "timer")
        .flow("fork", "msg")
        .flow("ut", "join")
        .flow("timer", "join")
        .flow("msg", "join")
        .flow("join", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    assert!(!engine.pending_user_tasks.is_empty());
    assert!(!engine.pending_timers.is_empty());
    assert!(!engine.pending_message_catches.is_empty());

    engine.delete_instance(inst_id).await.unwrap();

    // All queues for this instance should be empty
    assert_eq!(
        engine
            .pending_user_tasks
            .iter()
            .filter(|t| t.instance_id == inst_id)
            .count(),
        0
    );
    assert_eq!(
        engine
            .pending_timers
            .iter()
            .filter(|t| t.instance_id == inst_id)
            .count(),
        0
    );
    assert_eq!(
        engine
            .pending_message_catches
            .iter()
            .filter(|t| t.instance_id == inst_id)
            .count(),
        0
    );
}

// -----------------------------------------------------------------------
// Tests targeting specific MISSED mutations
// -----------------------------------------------------------------------

/// Catches: replace cancel_boundary_timers with ()
/// Catches: == vs != and && vs || in cancel_boundary_timers filter/retain
#[tokio::test]
async fn test_cancel_boundary_timers_removes_timers_on_task_complete() {
    let engine = WorkflowEngine::with_in_memory_persistence();
    let def = ProcessDefinitionBuilder::new("bt_cancel")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node(
            "bt",
            BpmnElement::BoundaryTimerEvent {
                attached_to: "ut".into(),
                timer: crate::domain::TimerDefinition::Duration(Duration::from_secs(3600)),
                cancel_activity: true,
            },
        )
        .node("timeout_end", BpmnElement::EndEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .flow("bt", "timeout_end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Boundary timer should be registered
    let timer_count_before = engine
        .pending_timers
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    assert_eq!(timer_count_before, 1, "Boundary timer should be pending");

    // Complete the user task → boundary timer must be cancelled
    let task_id = engine
        .pending_user_tasks
        .iter()
        .find(|r| r.instance_id == inst_id)
        .map(|r| r.task_id)
        .unwrap();
    engine
        .complete_user_task(task_id, HashMap::new())
        .await
        .unwrap();

    let timer_count_after = engine
        .pending_timers
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    assert_eq!(
        timer_count_after, 0,
        "Boundary timer should be cancelled after task completion"
    );
}

/// Catches: replace cancel_boundary_message_catches with ()
/// Catches: == vs != and && vs || in cancel_boundary_message_catches filter/retain
/// Catches: delete ! in cancel_boundary_message_catches retain predicate
#[tokio::test]
async fn test_cancel_boundary_message_catches_on_task_complete() {
    let engine = WorkflowEngine::with_in_memory_persistence();
    let def = ProcessDefinitionBuilder::new("bm_cancel")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node(
            "bm",
            BpmnElement::BoundaryMessageEvent {
                attached_to: "ut".into(),
                message_name: "cancel_msg".into(),
                cancel_activity: true,
            },
        )
        .node("msg_end", BpmnElement::EndEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .flow("bm", "msg_end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Boundary message catch should be registered
    let msg_count_before = engine
        .pending_message_catches
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    assert_eq!(
        msg_count_before, 1,
        "Boundary message catch should be pending"
    );

    // Complete the user task → boundary message must be cancelled
    let task_id = engine
        .pending_user_tasks
        .iter()
        .find(|r| r.instance_id == inst_id)
        .map(|r| r.task_id)
        .unwrap();
    engine
        .complete_user_task(task_id, HashMap::new())
        .await
        .unwrap();

    let msg_count_after = engine
        .pending_message_catches
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    assert_eq!(
        msg_count_after, 0,
        "Boundary message catch should be cancelled after task completion"
    );
}

/// Catches: && vs || in clear_wait_states_for_token filter
/// Catches: replace clear_wait_states_for_token with ()
#[tokio::test]
async fn test_event_based_gateway_cancels_alternatives() {
    let engine = WorkflowEngine::with_in_memory_persistence();

    let def = ProcessDefinitionBuilder::new("ebg_cancel")
        .node("start", BpmnElement::StartEvent)
        .node("ebg", BpmnElement::EventBasedGateway)
        .node(
            "timer_catch",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_secs(3600),
            )),
        )
        .node(
            "msg_catch",
            BpmnElement::MessageCatchEvent {
                message_name: "go".into(),
            },
        )
        .node("timer_end", BpmnElement::EndEvent)
        .node("msg_end", BpmnElement::EndEvent)
        .flow("start", "ebg")
        .flow("ebg", "timer_catch")
        .flow("ebg", "msg_catch")
        .flow("timer_catch", "timer_end")
        .flow("msg_catch", "msg_end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Both a timer and a message catch should be pending
    let timer_count = engine
        .pending_timers
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    let msg_count = engine
        .pending_message_catches
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    assert_eq!(timer_count, 1, "Timer catch should be pending");
    assert_eq!(msg_count, 1, "Message catch should be pending");

    // Correlate the message → timer alternative should be cancelled
    engine
        .correlate_message("go".to_string(), None, HashMap::new())
        .await
        .unwrap();

    let timer_after = engine
        .pending_timers
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    assert_eq!(
        timer_after, 0,
        "Timer catch should be cancelled when message fires"
    );
}

/// Catches: cancel_boundary_timers isolates instance — only timers for the
/// specific instance+node are removed, not unrelated timers.
#[tokio::test]
async fn test_cancel_boundary_timers_isolates_instances() {
    let engine = WorkflowEngine::with_in_memory_persistence();
    let def = ProcessDefinitionBuilder::new("bt_iso")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node(
            "bt",
            BpmnElement::BoundaryTimerEvent {
                attached_to: "ut".into(),
                timer: crate::domain::TimerDefinition::Duration(Duration::from_secs(3600)),
                cancel_activity: true,
            },
        )
        .node("timeout_end", BpmnElement::EndEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .flow("bt", "timeout_end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    let inst_a = engine.start_instance(key).await.unwrap();
    let inst_b = engine.start_instance(key).await.unwrap();

    // Both instances should have boundary timers
    assert_eq!(
        engine
            .pending_timers
            .iter()
            .filter(|r| r.instance_id == inst_a)
            .count(),
        1
    );
    assert_eq!(
        engine
            .pending_timers
            .iter()
            .filter(|r| r.instance_id == inst_b)
            .count(),
        1
    );

    // Complete inst_a user task → only inst_a boundary timer should be cancelled
    let task_a = engine
        .pending_user_tasks
        .iter()
        .find(|r| r.instance_id == inst_a)
        .map(|r| r.task_id)
        .unwrap();
    engine
        .complete_user_task(task_a, HashMap::new())
        .await
        .unwrap();

    assert_eq!(
        engine
            .pending_timers
            .iter()
            .filter(|r| r.instance_id == inst_a)
            .count(),
        0,
        "inst_a timers should be cancelled"
    );
    assert_eq!(
        engine
            .pending_timers
            .iter()
            .filter(|r| r.instance_id == inst_b)
            .count(),
        1,
        "inst_b timers should remain"
    );
}

/// Tests that correlate_message with a MessageStartEvent starts a new instance.
#[tokio::test]
async fn test_correlate_message_starts_new_instance() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("msg_start_corr")
        .node(
            "start",
            BpmnElement::MessageStartEvent {
                message_name: "order_created".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();

    let _ = engine.deploy_definition(def).await;

    // No instances before message
    assert!(engine.list_instances().await.is_empty());

    let mut vars = HashMap::new();
    vars.insert("orderId".into(), serde_json::Value::from(42));

    let affected = engine
        .correlate_message("order_created".into(), None, vars)
        .await
        .unwrap();
    assert_eq!(affected.len(), 1);

    // Instance should have been created and completed (start → end)
    let inst = engine.get_instance_details(affected[0]).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(
        inst.variables.get("orderId"),
        Some(&serde_json::Value::from(42))
    );
}
