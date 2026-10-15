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
    let mut newer = engine.get_definition(&key).await.unwrap().as_ref().clone();
    newer
        .nodes
        .insert("work".into(), BpmnElement::UserTask("newer".into()));
    let (new_key, _) = engine.deploy_definition(newer).await;
    assert_ne!(new_key, key);
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

fn parallel_loop(count: usize) -> crate::domain::LoopCharacteristics {
    crate::domain::LoopCharacteristics::MultiInstance(crate::domain::MultiInstanceDef {
        is_sequential: false,
        loop_cardinality: Some(count.to_string()),
        collection: None,
        element_variable: None,
        completion_condition: None,
        output_collection: None,
        output_element: None,
    })
}

#[tokio::test]
async fn repeated_handler_multi_instance_activations_complete_independently() {
    let engine = WorkflowEngine::new();
    let mut def = definition(Trigger::Message("alert".into()), false);
    def.loops.insert("work".into(), parallel_loop(2));
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
    for _ in 0..3 {
        finish_user(&engine, "work", HashMap::new()).await;
        assert_ne!(
            engine.get_instance_details(id).await.unwrap().state,
            InstanceState::Completed
        );
    }
    finish_user(&engine, "work", HashMap::new()).await;
    let result = engine.get_instance_details(id).await.unwrap();
    assert_eq!(result.state, InstanceState::Completed);
    assert!(result.multi_instance_state.is_empty());
}

#[tokio::test]
async fn interrupting_handler_cancels_main_loops_and_other_handler_activations() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let mut def = definition(Trigger::Message("alert".into()), false);
    def.loops.insert("main".into(), parallel_loop(3));
    def.loops.insert("work".into(), parallel_loop(2));
    let body = ProcessDefinitionBuilder::new("stop-handler")
        .node("stop_start", BpmnElement::StartEvent)
        .node("stop_work", BpmnElement::UserTask("stop".into()))
        .node("stop_end", BpmnElement::EndEvent)
        .flow("stop_start", "stop_work")
        .flow("stop_work", "stop_end")
        .build()
        .unwrap();
    def.event_subprocesses.push(EventSubProcess {
        id: "stop_handler".into(),
        start_node: "stop_start".into(),
        node_ids: body.nodes.keys().cloned().collect(),
        trigger: Trigger::Signal("stop".into()),
        is_interrupting: true,
    });
    def.nodes.extend(body.nodes);
    def.flows.extend(body.flows);
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    engine
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    let cancelled = engine.get_pending_user_tasks();
    assert_eq!(cancelled.len(), 5);
    engine
        .broadcast_signal("stop", HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert!(
        engine
            .get_instance_details(id)
            .await
            .unwrap()
            .multi_instance_state
            .is_empty()
    );
    for task in cancelled {
        assert!(
            engine
                .complete_user_task(task.task_id, HashMap::new())
                .await
                .is_err()
        );
    }
    finish_user(&engine, "stop_work", HashMap::new()).await;
    let history = handler_history(&persistence, id).await;
    let cancelled: Vec<_> = history
        .iter()
        .filter(|entry| entry.event_type == crate::HistoryEventType::EventSubprocessInterrupted)
        .collect();
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].node_id.as_deref(), Some("handler"));
    assert!(
        !history
            .iter()
            .any(|entry| entry.node_id.as_deref() == Some("handler")
                && entry.event_type == crate::HistoryEventType::EventSubprocessCompleted)
    );
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn terminate_in_handler_cleans_process_work_and_subscriptions() {
    let engine = WorkflowEngine::new();
    let mut def = definition(Trigger::Message("alert".into()), false);
    def.nodes
        .insert("work".into(), BpmnElement::TerminateEndEvent);
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    engine
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    let instance = engine.get_instance_details(id).await.unwrap();
    assert_eq!(instance.state, InstanceState::Completed);
    assert!(instance.tokens.is_empty());
    assert!(instance.event_state.unwrap().subscriptions.is_empty());
    assert!(engine.get_pending_user_tasks().is_empty());
    assert!(engine.get_pending_message_catches().is_empty());
}

#[tokio::test]
async fn concurrent_interrupting_signals_claim_one_handler() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Signal("stop".into()), true))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    let (first, second) = tokio::join!(
        engine.broadcast_signal("stop", HashMap::new()),
        engine.broadcast_signal("stop", HashMap::new())
    );
    first.unwrap();
    second.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(
        engine
            .get_instance_details(id)
            .await
            .unwrap()
            .event_state
            .unwrap()
            .activations
            .len(),
        1
    );
    finish_user(&engine, "work", HashMap::new()).await;
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn exact_error_handler_precedes_wildcard_handler() {
    for (code, expected) in [("FAIL", "exact_work"), ("OTHER", "work")] {
        let engine = WorkflowEngine::new();
        let mut def = definition(Trigger::Error(None), true);
        def.nodes.insert(
            "main".into(),
            BpmnElement::ServiceTask {
                topic: "main".into(),
                multi_instance: None,
            },
        );
        let body = ProcessDefinitionBuilder::new("exact")
            .node("exact_start", BpmnElement::StartEvent)
            .node("exact_work", BpmnElement::UserTask("exact".into()))
            .node("exact_end", BpmnElement::EndEvent)
            .flow("exact_start", "exact_work")
            .flow("exact_work", "exact_end")
            .build()
            .unwrap();
        def.event_subprocesses.push(EventSubProcess {
            id: "exact".into(),
            start_node: "exact_start".into(),
            node_ids: body.nodes.keys().cloned().collect(),
            trigger: Trigger::Error(Some("FAIL".into())),
            is_interrupting: true,
        });
        def.nodes.extend(body.nodes);
        def.flows.extend(body.flows);
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine.start_instance(key).await.unwrap();
        let task = engine
            .fetch_and_lock_service_tasks("worker", 1, &["main".into()], 60)
            .await
            .remove(0);
        engine
            .handle_bpmn_error(task.id, "worker", code)
            .await
            .unwrap();
        assert_eq!(engine.get_pending_user_tasks().len(), 1);
        assert_eq!(engine.get_pending_user_tasks()[0].node_id, expected);
        finish_user(&engine, expected, HashMap::new()).await;
        assert_eq!(
            engine.get_instance_details(id).await.unwrap().state,
            InstanceState::Completed
        );
    }
}

#[tokio::test]
async fn activity_error_boundary_precedes_process_handler() {
    let engine = WorkflowEngine::new();
    let mut def = definition(Trigger::Error(None), true);
    def.nodes.insert(
        "main".into(),
        BpmnElement::ServiceTask {
            topic: "main".into(),
            multi_instance: None,
        },
    );
    def.nodes.insert(
        "boundary".into(),
        BpmnElement::BoundaryErrorEvent {
            attached_to: "main".into(),
            error_code: Some("FAIL".into()),
        },
    );
    def.nodes
        .insert("caught".into(), BpmnElement::UserTask("caught".into()));
    def.flows.insert(
        "boundary".into(),
        vec![crate::domain::SequenceFlow::simple("caught")],
    );
    def.flows.insert(
        "caught".into(),
        vec![crate::domain::SequenceFlow::simple("end")],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = engine
        .fetch_and_lock_service_tasks("worker", 1, &["main".into()], 60)
        .await
        .remove(0);
    engine
        .handle_bpmn_error(task.id, "worker", "FAIL")
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "caught");
    assert!(
        engine
            .get_instance_details(id)
            .await
            .unwrap()
            .event_state
            .unwrap()
            .activations
            .is_empty()
    );
    finish_user(&engine, "caught", HashMap::new()).await;
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn snapshots_without_event_metadata_remain_readable() {
    let engine = WorkflowEngine::new();
    let mut old_definition =
        serde_json::to_value(definition(Trigger::Message("alert".into()), false)).unwrap();
    old_definition
        .as_object_mut()
        .unwrap()
        .remove("event_subprocesses");
    let decoded: ProcessDefinition = serde_json::from_value(old_definition).unwrap();
    assert!(decoded.event_subprocesses.is_empty());
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Message("alert".into()), false))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    let mut old_instance =
        serde_json::to_value(engine.get_instance_details(id).await.unwrap()).unwrap();
    old_instance.as_object_mut().unwrap().remove("event_state");
    for token in old_instance["tokens"].as_object_mut().unwrap().values_mut() {
        token.as_object_mut().unwrap().remove("event_context");
    }
    for active in old_instance["active_tokens"].as_array_mut().unwrap() {
        active["token"]
            .as_object_mut()
            .unwrap()
            .remove("event_context");
    }
    let decoded: ProcessInstance = serde_json::from_value(old_instance).unwrap();
    assert!(decoded.event_state.is_none());
    assert!(
        decoded
            .tokens
            .values()
            .all(|token| token.event_context.is_none())
    );
}

#[tokio::test]
async fn invalid_programmatic_handler_is_rejected_before_instance_creation() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Error(None), false))
        .await;
    assert!(matches!(
        engine.start_instance(key).await,
        Err(EngineError::InvalidDefinition(_))
    ));
    assert!(engine.instances.all().await.is_empty());
}

async fn handler_history(persistence: &InMemoryPersistence, id: Uuid) -> Vec<crate::HistoryEntry> {
    use crate::HistoryEventType::*;
    persistence
        .query_history(crate::persistence::HistoryQuery {
            instance_id: id,
            event_types: Some(vec![
                EventSubprocessActivated,
                EventSubprocessInterrupted,
                EventSubprocessScopeInterrupted,
                EventSubprocessCompleted,
            ]),
            ..Default::default()
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn repeated_handlers_record_distinct_lifecycle_history_and_notifications() {
    use crate::HistoryEventType::*;
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Message("alert".into()), false))
        .await;
    let mut events = engine.subscribe_events();
    let id = engine.start_instance(key).await.unwrap();
    for _ in 0..2 {
        engine
            .correlate_message("alert".into(), None, HashMap::new())
            .await
            .unwrap();
    }
    finish_user(&engine, "work", HashMap::new()).await;
    assert_ne!(
        engine.get_instance_details(id).await.unwrap().state,
        InstanceState::Completed
    );
    finish_user(&engine, "work", HashMap::new()).await;
    finish_user(&engine, "main", HashMap::new()).await;
    let history = handler_history(&persistence, id).await;
    assert_eq!(history.len(), 4);
    let started: Vec<_> = history
        .iter()
        .filter(|entry| entry.event_type == EventSubprocessActivated)
        .collect();
    assert_eq!(started.len(), 2);
    assert_ne!(
        started[0].context["activation_id"],
        started[1].context["activation_id"]
    );
    for start in started {
        assert_eq!(
            history
                .iter()
                .filter(
                    |entry| entry.context["activation_id"] == start.context["activation_id"]
                        && entry.event_type == EventSubprocessCompleted
                )
                .count(),
            1
        );
    }
    let mut notifications = Vec::new();
    while let Ok(event) = events.try_recv() {
        if let EngineEvent::EventSubprocessChanged {
            event_id,
            instance_id,
            definition_key,
            subprocess_id,
            activation_id,
            phase,
            ..
        } = event
        {
            assert_eq!(instance_id, id);
            assert_eq!(definition_key, key);
            assert_eq!(subprocess_id, "handler");
            let entry = history.iter().find(|entry| entry.id == event_id).unwrap();
            assert_eq!(entry.context["activation_id"], activation_id.to_string());
            assert_eq!(entry.definition_version, Some(1));
            notifications.push(phase);
        }
    }
    assert_eq!(
        notifications,
        ["activated", "activated", "completed", "completed"]
    );
    assert!(
        engine
            .get_instance_details(id)
            .await
            .unwrap()
            .event_state
            .unwrap()
            .lifecycle_outbox
            .is_empty()
    );
}

#[tokio::test]
async fn interruption_history_distinguishes_scope_interruption_from_activation() {
    use crate::HistoryEventType::*;
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Signal("stop".into()), true))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    engine
        .broadcast_signal("stop", HashMap::new())
        .await
        .unwrap();
    finish_user(&engine, "work", HashMap::new()).await;
    let history = handler_history(&persistence, id).await;
    assert_eq!(history.len(), 3);
    for kind in [
        EventSubprocessScopeInterrupted,
        EventSubprocessActivated,
        EventSubprocessCompleted,
    ] {
        assert_eq!(
            history
                .iter()
                .filter(|entry| entry.event_type == kind)
                .count(),
            1
        );
    }
    assert!(history.iter().all(|entry| entry.context["generation"] == 1));
}

#[tokio::test]
async fn recovery_replays_unacknowledged_history_without_duplicate_entries() {
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
    let entry = handler_history(&persistence, id).await.remove(0);
    // Crash after history append but before acknowledging the outbox snapshot.
    let mut snapshot = engine.get_instance_details(id).await.unwrap();
    snapshot
        .event_state
        .as_mut()
        .unwrap()
        .lifecycle_outbox
        .push(entry.clone());
    persistence.save_instance(&snapshot).await.unwrap();
    drop(engine);
    let restored = WorkflowEngine::new().with_persistence(persistence.clone());
    for definition in persistence.list_definitions().await.unwrap() {
        restored.deploy_definition(definition).await;
    }
    for snapshot in persistence.list_instances().await.unwrap() {
        restored.restore_instance(snapshot).await;
    }
    for task in persistence.list_user_tasks().await.unwrap() {
        restored.restore_user_task(task);
    }
    for catch in persistence.list_message_catches().await.unwrap() {
        restored.restore_message_catch(catch);
    }
    restored.reconcile_event_subprocesses().await.unwrap();
    restored.reconcile_event_subprocesses().await.unwrap();
    assert_eq!(
        handler_history(&persistence, id)
            .await
            .iter()
            .filter(|candidate| candidate.id == entry.id)
            .count(),
        1
    );
    assert!(
        restored
            .get_instance_details(id)
            .await
            .unwrap()
            .event_state
            .unwrap()
            .lifecycle_outbox
            .is_empty()
    );
    finish_user(&restored, "work", HashMap::new()).await;
    finish_user(&restored, "main", HashMap::new()).await;
    assert_eq!(handler_history(&persistence, id).await.len(), 2);
}

#[tokio::test]
async fn failed_history_write_keeps_lifecycle_outbox_for_retry() {
    use super::helpers::InstrumentedPersistence;
    use std::sync::atomic::Ordering;
    let persistence = Arc::new(InstrumentedPersistence::new(Duration::ZERO, Duration::ZERO));
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine
        .deploy_definition(definition(Trigger::Message("alert".into()), false))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    persistence.history_failures.store(100, Ordering::SeqCst);
    engine
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    let pending = persistence
        .inner
        .list_instances()
        .await
        .unwrap()
        .into_iter()
        .find(|snapshot| snapshot.id == id)
        .unwrap();
    assert_eq!(pending.event_state.unwrap().lifecycle_outbox.len(), 1);
    persistence.history_failures.store(0, Ordering::SeqCst);
    engine.flush_event_lifecycle(id).await;
    assert_eq!(handler_history(&persistence.inner, id).await.len(), 1);
    assert!(
        engine
            .get_instance_details(id)
            .await
            .unwrap()
            .event_state
            .unwrap()
            .lifecycle_outbox
            .is_empty()
    );
}
