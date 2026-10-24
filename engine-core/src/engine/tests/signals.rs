//! Signal-Events: Broadcast an wartende Catches, Signal-Start, Boundary und Persistenz.

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::ProcessDefinitionBuilder;

fn signal_catch_definition(name: &str, signal: &str, key: Option<Uuid>) -> ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new(name);
    if let Some(key) = key {
        builder = builder.with_key(key);
    }
    builder
        .node("start", BpmnElement::StartEvent)
        .node(
            "catch",
            BpmnElement::SignalCatchEvent {
                signal_name: signal.to_string(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "catch")
        .flow("catch", "end")
        .build()
        .unwrap()
}

#[tokio::test]
async fn broadcast_reaches_all_matching_catches() {
    let engine = WorkflowEngine::new();

    let (matching_key, _) = engine
        .deploy_definition(signal_catch_definition("sig_match", "PING", None))
        .await;
    let (other_key, _) = engine
        .deploy_definition(signal_catch_definition("sig_other", "OTHER", None))
        .await;

    let inst_a = engine.start_instance(matching_key).await.unwrap();
    let inst_b = engine.start_instance(matching_key).await.unwrap();
    let inst_other = engine.start_instance(other_key).await.unwrap();

    assert_eq!(engine.pending_signal_catches.len(), 3);

    let affected = engine
        .correlate_signal("PING".into(), HashMap::new())
        .await
        .unwrap();

    assert_eq!(affected.len(), 2);
    assert!(affected.contains(&inst_a));
    assert!(affected.contains(&inst_b));

    assert_eq!(
        engine.get_instance_state(inst_a).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(
        engine.get_instance_state(inst_b).await.unwrap(),
        InstanceState::Completed
    );
    assert!(matches!(
        engine.get_instance_state(inst_other).await.unwrap(),
        InstanceState::WaitingOnSignal { .. }
    ));
    assert_eq!(engine.pending_signal_catches.len(), 1);
}

#[tokio::test]
async fn signal_start_starts_instance_and_plain_start_is_rejected() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("sig_start")
        .node(
            "start",
            BpmnElement::SignalStartEvent {
                signal_name: "START_ME".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    let rejected = engine.start_instance(def_key).await;
    assert!(matches!(
        rejected,
        Err(EngineError::InvalidDefinition(msg)) if msg.contains("correlate_signal")
    ));

    let mut vars = HashMap::new();
    vars.insert(
        "origin".to_string(),
        serde_json::Value::String("signal".into()),
    );

    let affected = engine
        .correlate_signal("START_ME".into(), vars.clone())
        .await
        .unwrap();

    assert_eq!(affected.len(), 1);
    let inst = engine.get_instance_details(affected[0]).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(
        inst.variables.get("origin"),
        Some(&serde_json::Value::String("signal".into()))
    );
}

#[tokio::test]
async fn throw_wakes_catch_in_same_instance() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("sig_throw")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node(
            "catch",
            BpmnElement::SignalCatchEvent {
                signal_name: "WAKE".into(),
            },
        )
        .node("end_catch", BpmnElement::EndEvent)
        .node("task", BpmnElement::UserTask("worker".into()))
        .node(
            "throw",
            BpmnElement::SignalThrowEvent {
                signal_name: "WAKE".into(),
            },
        )
        .node("end_throw", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("fork", "catch")
        .flow("fork", "task")
        .flow("catch", "end_catch")
        .flow("task", "throw")
        .flow("throw", "end_throw")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.pending_signal_catches.len(), 1);
    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 1);

    engine
        .complete_user_task(tasks[0].task_id, HashMap::new())
        .await
        .unwrap();

    assert_eq!(engine.pending_signal_catches.len(), 0);
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("Signal 'WAKE' caught")));
    assert!(log.iter().any(|l| l.contains("Signal 'WAKE' broadcast")));
}

fn user_task_with_boundary_signal(cancel_activity: bool) -> ProcessDefinition {
    ProcessDefinitionBuilder::new("sig_boundary")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("worker".into()))
        .node(
            "bound_signal",
            BpmnElement::BoundarySignalEvent {
                attached_to: "task".into(),
                signal_name: "ABORT".into(),
                cancel_activity,
            },
        )
        .node("boundary_end", BpmnElement::EndEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .flow("bound_signal", "boundary_end")
        .build()
        .unwrap()
}

#[tokio::test]
async fn interrupting_boundary_signal_cancels_user_task() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(user_task_with_boundary_signal(true))
        .await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.get_pending_user_tasks().len(), 1);

    let affected = engine
        .correlate_signal("ABORT".into(), HashMap::new())
        .await
        .unwrap();

    assert_eq!(affected, vec![inst_id]);
    assert!(
        engine.get_pending_user_tasks().is_empty(),
        "interrupting boundary signal must remove the pending user task"
    );
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("'boundary_end'")));
}

#[tokio::test]
async fn non_interrupting_boundary_signal_keeps_user_task() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(user_task_with_boundary_signal(false))
        .await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    engine
        .correlate_signal("ABORT".into(), HashMap::new())
        .await
        .unwrap();

    let tasks = engine.get_pending_user_tasks();
    assert_eq!(
        tasks.len(),
        1,
        "non-interrupting boundary signal must keep the user task"
    );
    assert_ne!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    engine
        .complete_user_task(tasks[0].task_id, HashMap::new())
        .await
        .unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn signal_end_event_broadcasts_and_completes() {
    let engine = WorkflowEngine::new();

    let (catch_key, _) = engine
        .deploy_definition(signal_catch_definition("sig_wait", "FINISHED", None))
        .await;
    let waiting = engine.start_instance(catch_key).await.unwrap();
    assert_eq!(engine.pending_signal_catches.len(), 1);

    let thrower_def = ProcessDefinitionBuilder::new("sig_end_thrower")
        .node("start", BpmnElement::StartEvent)
        .node(
            "end_signal",
            BpmnElement::SignalEndEvent {
                signal_name: "FINISHED".into(),
            },
        )
        .flow("start", "end_signal")
        .build()
        .unwrap();
    let (thrower_key, _) = engine.deploy_definition(thrower_def).await;

    let thrower = engine.start_instance(thrower_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(thrower).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(
        engine.get_instance_state(waiting).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(engine.pending_signal_catches.len(), 0);
}

#[tokio::test]
async fn signal_catch_persists_and_restores() {
    let pers = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(pers.clone());

    let def = signal_catch_definition("sig_restore", "RELOAD", None);
    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    let persisted = pers.list_signal_catches().await.unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].instance_id, inst_id);
    assert_eq!(persisted[0].signal_name, "RELOAD");

    let engine2 = WorkflowEngine::new().with_persistence(pers.clone());
    engine2
        .deploy_definition(signal_catch_definition(
            "sig_restore",
            "RELOAD",
            Some(def_key),
        ))
        .await;
    for instance in pers.list_instances().await.unwrap() {
        engine2.restore_instance(instance).await;
    }
    for catch in pers.list_signal_catches().await.unwrap() {
        engine2.restore_signal_catch(catch);
    }

    assert_eq!(engine2.pending_signal_catches.len(), 1);

    let affected = engine2
        .correlate_signal("RELOAD".into(), HashMap::new())
        .await
        .unwrap();
    assert_eq!(affected, vec![inst_id]);
    assert_eq!(
        engine2.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert!(pers.list_signal_catches().await.unwrap().is_empty());
}

#[tokio::test]
async fn service_task_boundary_signal_is_cancelled() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("sig_boundary_service")
        .node("start", BpmnElement::StartEvent)
        .node(
            "task",
            BpmnElement::ServiceTask {
                topic: "long_running".into(),
                multi_instance: None,
            },
        )
        .node(
            "bound_signal",
            BpmnElement::BoundarySignalEvent {
                attached_to: "task".into(),
                signal_name: "ABORT".into(),
                cancel_activity: true,
            },
        )
        .node("boundary_end", BpmnElement::EndEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .flow("bound_signal", "boundary_end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();
    assert_eq!(engine.get_pending_service_tasks().len(), 1);

    engine
        .correlate_signal("ABORT".into(), HashMap::new())
        .await
        .unwrap();

    assert!(
        engine.get_pending_service_tasks().is_empty(),
        "interrupting boundary signal must remove the pending service task"
    );
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn event_based_gateway_signal_cancels_sibling_timer() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("sig_ebg")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::EventBasedGateway)
        .node(
            "catch_timer",
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_millis(5000),
            )),
        )
        .node(
            "catch_signal",
            BpmnElement::SignalCatchEvent {
                signal_name: "WIN".into(),
            },
        )
        .node("end_timer", BpmnElement::EndEvent)
        .node("end_signal", BpmnElement::EndEvent)
        .flow("start", "gw")
        .flow("gw", "catch_timer")
        .flow("gw", "catch_signal")
        .flow("catch_timer", "end_timer")
        .flow("catch_signal", "end_signal")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::WaitingOnEventBasedGateway
    );
    assert_eq!(engine.pending_timers.len(), 1);
    assert_eq!(engine.pending_signal_catches.len(), 1);

    let affected = engine
        .correlate_signal("WIN".into(), HashMap::new())
        .await
        .unwrap();
    assert_eq!(affected, vec![inst_id]);

    assert_eq!(engine.pending_timers.len(), 0);
    assert_eq!(engine.pending_signal_catches.len(), 0);
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("cancelled")));
    assert!(log.iter().any(|l| l.contains("'end_signal'")));
}
