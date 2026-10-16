//! Signal events (X1): broadcast to catches, boundaries, event gateways and
//! signal starts; throw/end events deliver after the throwing batch.

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::ProcessDefinitionBuilder;
use crate::runtime::CatchKind;

fn waiting_for_signal(id: &str, signal: &str) -> ProcessDefinition {
    ProcessDefinitionBuilder::new(id)
        .node("start", BpmnElement::StartEvent)
        .node(
            "wait",
            BpmnElement::SignalCatchEvent {
                signal_name: signal.into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "wait")
        .flow("wait", "end")
        .build()
        .unwrap()
}

fn user_task_with_signal_boundary(cancel_activity: bool) -> ProcessDefinition {
    ProcessDefinitionBuilder::new("signal_boundary")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("clerk".into()))
        .node(
            "abort",
            BpmnElement::BoundarySignalEvent {
                attached_to: "task".into(),
                signal_name: "shutdown".into(),
                cancel_activity,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .node("aborted", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .flow("abort", "aborted")
        .build()
        .unwrap()
}

async fn state_of(engine: &WorkflowEngine, id: Uuid) -> InstanceState {
    engine.get_instance_details(id).await.unwrap().state
}

#[tokio::test]
async fn broadcast_resumes_every_waiting_instance() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(waiting_for_signal("catcher", "go"))
        .await;
    let a = engine.start_instance(def_key).await.unwrap();
    let b = engine.start_instance(def_key).await.unwrap();

    let affected = engine.broadcast_signal("go", HashMap::new()).await.unwrap();

    assert_eq!(affected.len(), 2);
    assert_eq!(state_of(&engine, a).await, InstanceState::Completed);
    assert_eq!(state_of(&engine, b).await, InstanceState::Completed);
    assert!(engine.pending_message_catches.is_empty());
}

#[tokio::test]
async fn signals_and_messages_do_not_mix() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(waiting_for_signal("catcher", "go"))
        .await;
    let id = engine.start_instance(def_key).await.unwrap();

    let by_message = engine
        .correlate_message("go".into(), None, HashMap::new())
        .await
        .unwrap();
    assert!(
        by_message.is_empty(),
        "a message must not trigger a signal catch"
    );
    let by_other_signal = engine
        .broadcast_signal("stop", HashMap::new())
        .await
        .unwrap();
    assert!(by_other_signal.is_empty());
    assert!(matches!(
        state_of(&engine, id).await,
        InstanceState::WaitingOnMessage { .. }
    ));

    engine
        .broadcast_signal("go", HashMap::from([("flag".into(), true.into())]))
        .await
        .unwrap();
    let inst = engine.get_instance_details(id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.variables.get("flag"), Some(&Value::from(true)));
}

#[tokio::test]
async fn interrupting_signal_boundary_cancels_task() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(user_task_with_signal_boundary(true))
        .await;
    let id = engine.start_instance(def_key).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);

    engine
        .broadcast_signal("shutdown", HashMap::new())
        .await
        .unwrap();

    let inst = engine.get_instance_details(id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.current_node, "aborted");
    assert!(engine.get_pending_user_tasks().is_empty());
}

#[tokio::test]
async fn non_interrupting_signal_boundary_keeps_task() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(user_task_with_signal_boundary(false))
        .await;
    let id = engine.start_instance(def_key).await.unwrap();

    let affected = engine
        .broadcast_signal("shutdown", HashMap::new())
        .await
        .unwrap();

    assert_eq!(affected, vec![id]);
    assert_eq!(
        engine.get_pending_user_tasks().len(),
        1,
        "task must survive a non-interrupting signal"
    );
    assert_ne!(state_of(&engine, id).await, InstanceState::Completed);
}

#[tokio::test]
async fn signal_start_starts_only_latest_version() {
    let engine = WorkflowEngine::new();
    let def = |marker: &str| {
        ProcessDefinitionBuilder::new("signal_started")
            .node(
                "start",
                BpmnElement::SignalStartEvent {
                    signal_name: "new_order".into(),
                },
            )
            .node(
                "mark",
                BpmnElement::ScriptTask {
                    script: format!("let version = \"{marker}\";"),
                    multi_instance: None,
                },
            )
            .node("end", BpmnElement::EndEvent)
            .flow("start", "mark")
            .flow("mark", "end")
            .build()
            .unwrap()
    };
    engine.deploy_definition(def("v1")).await;
    engine.deploy_definition(def("v2")).await;

    let started = engine
        .broadcast_signal("new_order", HashMap::from([("qty".into(), 3.into())]))
        .await
        .unwrap();

    assert_eq!(started.len(), 1);
    let inst = engine.get_instance_details(started[0]).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.variables.get("version"), Some(&Value::from("v2")));
    assert_eq!(inst.variables.get("qty"), Some(&Value::from(3)));
}

#[tokio::test]
async fn throw_and_end_events_broadcast_after_their_batch() {
    let engine = WorkflowEngine::new();
    let (catcher, _) = engine
        .deploy_definition(waiting_for_signal("catcher", "go"))
        .await;
    let (end_catcher, _) = engine
        .deploy_definition(waiting_for_signal("end_catcher", "done"))
        .await;
    let waiting = engine.start_instance(catcher).await.unwrap();
    let waiting_for_end = engine.start_instance(end_catcher).await.unwrap();

    let thrower = ProcessDefinitionBuilder::new("thrower")
        .node("start", BpmnElement::StartEvent)
        .node(
            "throw",
            BpmnElement::SignalThrowEvent {
                signal_name: "go".into(),
            },
        )
        .node(
            "end",
            BpmnElement::SignalEndEvent {
                signal_name: "done".into(),
            },
        )
        .flow("start", "throw")
        .flow("throw", "end")
        .build()
        .unwrap();
    let (thrower_key, _) = engine.deploy_definition(thrower).await;
    let thrower_id = engine.start_instance(thrower_key).await.unwrap();

    assert_eq!(
        state_of(&engine, thrower_id).await,
        InstanceState::Completed
    );
    assert_eq!(state_of(&engine, waiting).await, InstanceState::Completed);
    assert_eq!(
        state_of(&engine, waiting_for_end).await,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn event_gateway_signal_branch_wins() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("gw_signal")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::EventBasedGateway)
        .node(
            "sig",
            BpmnElement::SignalCatchEvent {
                signal_name: "alarm".into(),
            },
        )
        .node(
            "msg",
            BpmnElement::MessageCatchEvent {
                message_name: "reply".into(),
            },
        )
        .node("end_sig", BpmnElement::EndEvent)
        .node("end_msg", BpmnElement::EndEvent)
        .flow("start", "gw")
        .flow("gw", "sig")
        .flow("gw", "msg")
        .flow("sig", "end_sig")
        .flow("msg", "end_msg")
        .build()
        .unwrap();
    let (def_key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(def_key).await.unwrap();
    assert_eq!(engine.pending_message_catches.len(), 2);

    engine
        .broadcast_signal("alarm", HashMap::new())
        .await
        .unwrap();

    let inst = engine.get_instance_details(id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.current_node, "end_sig");
    assert!(
        engine.pending_message_catches.is_empty(),
        "the losing message catch must be cleared"
    );
}

#[tokio::test]
async fn signal_catch_survives_restart() {
    let pers = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(pers.clone());
    let def = waiting_for_signal("catcher", "go");
    let def_key = def.key;
    engine.deploy_definition(def).await;
    let id = engine.start_instance(def_key).await.unwrap();
    let persisted = pers.list_message_catches().await.unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].kind, CatchKind::Signal);
    drop(engine);

    let engine2 = WorkflowEngine::new().with_persistence(pers.clone());
    engine2
        .deploy_definition(
            ProcessDefinitionBuilder::new("catcher")
                .with_key(def_key)
                .node("start", BpmnElement::StartEvent)
                .node(
                    "wait",
                    BpmnElement::SignalCatchEvent {
                        signal_name: "go".into(),
                    },
                )
                .node("end", BpmnElement::EndEvent)
                .flow("start", "wait")
                .flow("wait", "end")
                .build()
                .unwrap(),
        )
        .await;
    for instance in pers.list_instances().await.unwrap() {
        engine2.restore_instance(instance).await;
    }
    for catch in pers.list_message_catches().await.unwrap() {
        engine2.restore_message_catch(catch);
    }

    let affected = engine2
        .broadcast_signal("go", HashMap::new())
        .await
        .unwrap();
    assert_eq!(affected, vec![id]);
    assert_eq!(state_of(&engine2, id).await, InstanceState::Completed);
}

#[test]
fn catches_persisted_before_signals_default_to_message() {
    let json = serde_json::json!({
        "id": Uuid::nil(),
        "instance_id": Uuid::nil(),
        "node_id": "wait",
        "message_name": "order",
        "token_id": Uuid::nil()
    });
    let catch: PendingMessageCatch = serde_json::from_value(json).unwrap();
    assert_eq!(catch.kind, CatchKind::Message);
}
