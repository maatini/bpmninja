use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::{
    EventSubProcess, EventSubProcessTrigger, ProcessDefinitionBuilder, TimerDefinition,
};
use std::sync::Arc;
use std::time::Duration;

fn definition(trigger: EventSubProcessTrigger, interrupting: bool) -> ProcessDefinition {
    let mut def = ProcessDefinitionBuilder::new("events")
        .node("start", BpmnElement::StartEvent)
        .node("main", BpmnElement::UserTask("main".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "main")
        .flow("main", "end")
        .build()
        .unwrap();
    attach_handler(&mut def, trigger, interrupting);
    def
}

fn attach_handler(
    def: &mut ProcessDefinition,
    trigger: EventSubProcessTrigger,
    interrupting: bool,
) {
    let body = ProcessDefinitionBuilder::new("handler")
        .node("hs", BpmnElement::StartEvent)
        .node("work", BpmnElement::UserTask("handler".into()))
        .node("he", BpmnElement::EndEvent)
        .flow("hs", "work")
        .flow("work", "he")
        .build()
        .unwrap();
    let mut node_ids: Vec<String> = body.nodes.keys().cloned().collect();
    node_ids.sort();
    def.nodes.extend(body.nodes);
    for (source, targets) in body.flows {
        def.flows.entry(source).or_default().extend(targets);
    }
    def.event_subprocesses.push(EventSubProcess {
        id: "handler".into(),
        start_node: "hs".into(),
        node_ids,
        trigger,
        is_interrupting: interrupting,
    });
}

async fn finish_user(engine: &WorkflowEngine, node: &str) {
    let task = engine
        .get_pending_user_tasks()
        .into_iter()
        .find(|task| task.node_id == node)
        .unwrap_or_else(|| panic!("missing user task {node}"));
    engine
        .complete_user_task(task.task_id, HashMap::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn interrupting_message_cancels_scope_and_cleans_up() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(
            EventSubProcessTrigger::Message("alert".into()),
            true,
        ))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_message_catches().len(), 1);

    engine
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].node_id, "work");
    assert!(engine.get_pending_message_catches().is_empty());

    finish_user(&engine, "work").await;
    let instance = engine.get_instance_details(id).await.unwrap();
    assert_eq!(instance.state, InstanceState::Completed);
    assert!(instance.event_subscriptions.is_empty());
    assert!(engine.get_pending_message_catches().is_empty());
}

#[tokio::test]
async fn non_interrupting_message_runs_in_parallel_until_scope_ends() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(
            EventSubProcessTrigger::Message("alert".into()),
            false,
        ))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    engine
        .correlate_message(
            "alert".into(),
            None,
            HashMap::from([("payload".into(), serde_json::json!(7))]),
        )
        .await
        .unwrap();
    engine
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 3);
    assert_eq!(engine.get_pending_message_catches().len(), 1);

    finish_user(&engine, "main").await;
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    finish_user(&engine, "work").await;
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    finish_user(&engine, "work").await;
    let instance = engine.get_instance_details(id).await.unwrap();
    assert_eq!(instance.state, InstanceState::Completed);
    assert!(engine.get_pending_message_catches().is_empty());
    assert!(instance.event_subscriptions.is_empty());
}

#[tokio::test]
async fn interrupting_timer_fires_without_sleep() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(
            EventSubProcessTrigger::Timer(TimerDefinition::Duration(Duration::ZERO)),
            true,
        ))
        .await;
    engine.start_instance(key).await.unwrap();
    assert_eq!(engine.process_timers().await.unwrap(), 1);
    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].node_id, "work");
    assert!(engine.get_pending_timers().is_empty());
}

#[tokio::test]
async fn non_interrupting_timer_repeats_then_unsubscribes() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(
            EventSubProcessTrigger::Timer(TimerDefinition::RepeatingInterval {
                repetitions: Some(2),
                interval: Duration::ZERO,
            }),
            false,
        ))
        .await;
    engine.start_instance(key).await.unwrap();
    assert_eq!(engine.process_timers().await.unwrap(), 1);
    assert_eq!(engine.process_timers().await.unwrap(), 1);
    assert_eq!(engine.get_pending_user_tasks().len(), 3);
    assert!(engine.get_pending_timers().is_empty());
    let tasks = engine.get_pending_user_tasks();
    assert_eq!(
        tasks.iter().filter(|task| task.node_id == "work").count(),
        2
    );
    assert_eq!(
        tasks.iter().filter(|task| task.node_id == "main").count(),
        1
    );
}

#[tokio::test]
async fn service_task_error_activates_interrupting_handler() {
    let engine = WorkflowEngine::new();
    let mut def = definition(EventSubProcessTrigger::Error(Some("FAIL".into())), true);
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
    assert!(
        engine
            .complete_service_task(task.id, "worker", HashMap::new())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn non_interrupting_escalation_keeps_main_execution() {
    let engine = WorkflowEngine::new();
    let mut def = ProcessDefinitionBuilder::new("esc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "throw",
            BpmnElement::EscalationThrowEvent {
                escalation_code: "ESC".into(),
            },
        )
        .node("main", BpmnElement::UserTask("main".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "throw")
        .flow("throw", "main")
        .flow("main", "end")
        .build()
        .unwrap();
    attach_handler(
        &mut def,
        EventSubProcessTrigger::Escalation(Some("ESC".into())),
        false,
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let nodes: Vec<String> = engine
        .get_pending_user_tasks()
        .into_iter()
        .map(|task| task.node_id)
        .collect();
    assert!(nodes.contains(&"main".to_string()));
    assert!(nodes.contains(&"work".to_string()));
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn completing_the_main_scope_drops_unused_subscriptions() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(
            EventSubProcessTrigger::Signal("alert".into()),
            false,
        ))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    assert_eq!(engine.pending_signal_catches.len(), 1);
    finish_user(&engine, "main").await;
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    assert!(engine.pending_signal_catches.is_empty());
}

#[tokio::test]
async fn suspended_scope_ignores_signal_until_resumed() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(
            EventSubProcessTrigger::Signal("alert".into()),
            false,
        ))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    engine.suspend_instance(id).await.unwrap();
    engine
        .correlate_signal("alert".into(), HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    engine.resume_instance(id).await.unwrap();
    engine
        .correlate_signal("alert".into(), HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 2);
}

#[tokio::test]
async fn message_subscription_is_restored_after_restart() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine
        .deploy_definition(definition(
            EventSubProcessTrigger::Message("alert".into()),
            true,
        ))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    for catch in persistence.list_message_catches().await.unwrap() {
        persistence.delete_message_catch(catch.id).await.unwrap();
    }

    let restored = WorkflowEngine::new().with_persistence(persistence.clone());
    restored.restore_definitions(persistence.list_definitions().await.unwrap());
    for instance in persistence.list_instances().await.unwrap() {
        restored.restore_instance(instance).await;
    }
    assert!(restored.get_pending_message_catches().is_empty());
    restored.reconcile_event_subprocesses().await.unwrap();
    assert_eq!(restored.get_pending_message_catches().len(), 1);

    restored
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    assert_eq!(restored.get_pending_user_tasks()[0].node_id, "work");
    assert_eq!(
        restored
            .get_instance_details(id)
            .await
            .unwrap()
            .tokens
            .len(),
        1
    );
}
