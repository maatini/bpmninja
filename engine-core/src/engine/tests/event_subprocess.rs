use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::{
    EventSubProcess, EventSubProcessTrigger as Trigger, ProcessDefinitionBuilder, TimerDefinition,
};
use crate::persistence::WorkflowPersistence;
use serde_json::json;

fn definition(trigger: Trigger, interrupting: bool) -> ProcessDefinition {
    let mut def = ProcessDefinitionBuilder::new("events")
        .node("start", BpmnElement::StartEvent)
        .node("main", BpmnElement::UserTask("main".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "main")
        .flow("main", "end")
        .build()
        .unwrap();
    let body = ProcessDefinitionBuilder::new("handler")
        .node("hs", BpmnElement::StartEvent)
        .node("work", BpmnElement::UserTask("handler".into()))
        .node("he", BpmnElement::EndEvent)
        .flow("hs", "work")
        .flow("work", "he")
        .build()
        .unwrap();
    def.event_subprocesses.push(EventSubProcess {
        id: "handler".into(),
        start_node: "hs".into(),
        node_ids: body.nodes.keys().cloned().collect(),
        trigger,
        is_interrupting: interrupting,
    });
    def.nodes.extend(body.nodes);
    def.flows.extend(body.flows);
    def
}
async fn finish_user(engine: &WorkflowEngine, node: &str, variables: HashMap<String, Value>) {
    let task = engine
        .get_pending_user_tasks()
        .into_iter()
        .find(|t| t.node_id == node)
        .unwrap();
    engine
        .complete_user_task(task.task_id, variables)
        .await
        .unwrap();
}

#[tokio::test]
async fn repeated_non_interrupting_message_handlers_wait_and_preserve_shared_writes() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Message("alert".into()), false))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    for value in [1, 2] {
        engine
            .correlate_message(
                "alert".into(),
                None,
                HashMap::from([("payload".into(), json!(value))]),
            )
            .await
            .unwrap();
    }
    assert_eq!(engine.get_pending_user_tasks().len(), 3);
    finish_user(
        &engine,
        "main",
        HashMap::from([("mainResult".into(), json!(10))]),
    )
    .await;
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    finish_user(
        &engine,
        "work",
        HashMap::from([("handlerResult".into(), json!(42))]),
    )
    .await;
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    finish_user(&engine, "work", HashMap::new()).await;
    let instance = engine.get_instance_details(id).await.unwrap();
    assert_eq!(instance.state, InstanceState::Completed);
    assert_eq!(instance.variables["mainResult"], json!(10));
    assert_eq!(instance.variables["handlerResult"], json!(42));
    assert_eq!(instance.variables["payload"], json!(2));
    assert!(engine.get_pending_message_catches().is_empty());
}

#[tokio::test]
async fn interrupting_signal_cancels_worker_and_fences_late_completion() {
    let engine = WorkflowEngine::new();
    let mut def = definition(Trigger::Signal("stop".into()), true);
    def.nodes.insert(
        "main".into(),
        BpmnElement::ServiceTask {
            topic: "main".into(),
            multi_instance: None,
        },
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = engine
        .fetch_and_lock_service_tasks("worker", 1, &["main".into()], 60)
        .await
        .remove(0);
    engine
        .broadcast_signal("stop", HashMap::new())
        .await
        .unwrap();
    assert!(engine.get_pending_service_tasks().is_empty());
    assert!(
        engine
            .complete_service_task(task.id, "worker", HashMap::new())
            .await
            .is_err()
    );
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    finish_user(&engine, "work", HashMap::new()).await;
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn non_interrupting_timer_repeats_twice_then_unsubscribes() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(
            Trigger::Timer(TimerDefinition::RepeatingInterval {
                repetitions: Some(2),
                interval: Duration::from_millis(1),
            }),
            false,
        ))
        .await;
    engine.start_instance(key).await.unwrap();
    for _ in 0..2 {
        tokio::time::sleep(Duration::from_millis(3)).await;
        engine.process_timers().await.unwrap();
    }
    assert_eq!(engine.get_pending_user_tasks().len(), 3);
    assert!(engine.get_pending_timers().is_empty());
}

#[tokio::test]
async fn suspended_scope_ignores_signal_until_resumed() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Signal("alert".into()), false))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    engine.suspend_instance(id).await.unwrap();
    engine
        .broadcast_signal("alert", HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    engine.resume_instance(id).await.unwrap();
    engine
        .broadcast_signal("alert", HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 2);
}

#[tokio::test]
async fn worker_error_activates_interrupting_process_handler() {
    let engine = WorkflowEngine::new();
    let mut def = definition(Trigger::Error(Some("FAIL".into())), true);
    def.nodes.insert(
        "main".into(),
        BpmnElement::ServiceTask {
            topic: "main".into(),
            multi_instance: None,
        },
    );
    let (key, _) = engine.deploy_definition(def).await;
    engine.start_instance(key).await.unwrap();
    let task = engine
        .fetch_and_lock_service_tasks("worker", 1, &["main".into()], 60)
        .await
        .remove(0);
    engine
        .handle_bpmn_error(task.id, "worker", "FAIL")
        .await
        .unwrap();
    assert!(engine.get_pending_service_tasks().is_empty());
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "work");
}

#[tokio::test]
async fn escalation_throw_keeps_main_execution_for_non_interrupting_handler() {
    let engine = WorkflowEngine::new();
    let mut def = definition(Trigger::Escalation(Some("WARN".into())), false);
    def.nodes.insert(
        "main".into(),
        BpmnElement::EscalationThrowEvent {
            escalation_code: "WARN".into(),
        },
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    finish_user(&engine, "work", HashMap::new()).await;
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn cancellation_cascades_to_call_activity_child() {
    let engine = WorkflowEngine::new();
    let child = ProcessDefinitionBuilder::new("child")
        .node("cs", BpmnElement::StartEvent)
        .node("cw", BpmnElement::UserTask("child".into()))
        .node("ce", BpmnElement::EndEvent)
        .flow("cs", "cw")
        .flow("cw", "ce")
        .build()
        .unwrap();
    engine.deploy_definition(child).await;
    let mut def = definition(Trigger::Signal("stop".into()), true);
    def.nodes.insert(
        "main".into(),
        BpmnElement::CallActivity {
            called_element: "child".into(),
        },
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let child_task = engine.get_pending_user_tasks()[0].clone();
    engine
        .broadcast_signal("stop", HashMap::new())
        .await
        .unwrap();
    assert!(
        engine
            .complete_user_task(child_task.task_id, HashMap::new())
            .await
            .is_err()
    );
    assert!(matches!(
        engine
            .get_instance_details(child_task.instance_id)
            .await
            .unwrap()
            .state,
        InstanceState::CompletedWithError { .. }
    ));
    finish_user(&engine, "work", HashMap::new()).await;
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn restart_keeps_original_definition_and_pending_handler() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Message("alert".into()), false))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    engine
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    let mut newer = definition(Trigger::Message("alert".into()), false);
    newer
        .nodes
        .insert("work".into(), BpmnElement::UserTask("newer".into()));
    engine.deploy_definition(newer).await;
    drop(engine);
    let restored = WorkflowEngine::new().with_persistence(persistence.clone());
    for def in persistence.list_definitions().await.unwrap() {
        restored.deploy_definition(def).await;
    }
    for instance in persistence.list_instances().await.unwrap() {
        restored
            .restore_instance(
                serde_json::from_value(serde_json::to_value(instance).unwrap()).unwrap(),
            )
            .await;
    }
    for task in persistence.list_user_tasks().await.unwrap() {
        restored.restore_user_task(task);
    }
    for catch in persistence.list_message_catches().await.unwrap() {
        restored.restore_message_catch(catch);
    }
    restored.reconcile_event_subprocesses().await.unwrap();
    restored
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    assert_eq!(restored.get_pending_user_tasks().len(), 3);
    assert!(
        restored
            .get_pending_user_tasks()
            .iter()
            .filter(|t| t.node_id == "work")
            .all(|t| t.assignee == "handler")
    );
    finish_user(&restored, "main", HashMap::new()).await;
    finish_user(&restored, "work", HashMap::new()).await;
    finish_user(&restored, "work", HashMap::new()).await;
    assert_eq!(
        restored.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn recovery_recreates_subscription_jobs_and_resumes_claimed_activation() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Message("alert".into()), true))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    let stale_tasks = persistence.list_user_tasks().await.unwrap();
    let subscription = *engine
        .get_instance_details(id)
        .await
        .unwrap()
        .event_state
        .unwrap()
        .subscriptions
        .keys()
        .next()
        .unwrap();
    // Simulate recovery from a snapshot whose subscription job was never saved.
    engine.pending_message_catches.remove(&subscription);
    engine.reconcile_event_subprocesses().await.unwrap();
    assert!(engine.pending_message_catches.contains_key(&subscription));
    // Persist the interrupting claim, but crash before running the handler.
    engine
        .prepare_event_activation(id, subscription, &HashMap::new())
        .await
        .unwrap()
        .unwrap();
    drop(engine);
    let restored = WorkflowEngine::new().with_persistence(persistence.clone());
    for def in persistence.list_definitions().await.unwrap() {
        restored.deploy_definition(def).await;
    }
    for instance in persistence.list_instances().await.unwrap() {
        restored.restore_instance(instance).await;
    }
    // A separate task bucket can still contain work from before interruption.
    for task in stale_tasks {
        restored.restore_user_task(task);
    }
    restored.reconcile_event_subprocesses().await.unwrap();
    let tasks = restored.get_pending_user_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].node_id, "work");
    finish_user(&restored, "work", HashMap::new()).await;
    assert_eq!(
        restored.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn repeated_handlers_keep_parallel_joins_separate() {
    let engine = WorkflowEngine::new();
    let mut def = definition(Trigger::Message("alert".into()), false);
    let body = ProcessDefinitionBuilder::new("parallel-handler")
        .node("hs", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node("a", BpmnElement::UserTask("a".into()))
        .node("b", BpmnElement::UserTask("b".into()))
        .node("join", BpmnElement::ParallelGateway)
        .node("he", BpmnElement::EndEvent)
        .flow("hs", "fork")
        .flow("fork", "a")
        .flow("fork", "b")
        .flow("a", "join")
        .flow("b", "join")
        .flow("join", "he")
        .build()
        .unwrap();
    def.nodes
        .retain(|id, _| !def.event_subprocesses[0].node_ids.contains(id));
    def.flows
        .retain(|source, _| !def.event_subprocesses[0].node_ids.contains(source));
    def.event_subprocesses[0].node_ids = body.nodes.keys().cloned().collect();
    def.nodes.extend(body.nodes);
    def.flows.extend(body.flows);
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    for _ in 0..2 {
        engine
            .correlate_message("alert".into(), None, HashMap::new())
            .await
            .unwrap();
    }
    assert_eq!(engine.get_pending_user_tasks().len(), 5);
    finish_user(&engine, "main", HashMap::new()).await;
    for _ in 0..2 {
        finish_user(&engine, "a", HashMap::new()).await;
    }
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    for _ in 0..2 {
        finish_user(&engine, "b", HashMap::new()).await;
    }
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}
