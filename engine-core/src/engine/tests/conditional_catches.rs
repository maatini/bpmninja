use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use serde_json::json;

fn definition(gateway: bool) -> ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new("conditional_catches")
        .node("start", BpmnElement::StartEvent)
        .node(
            "catch",
            BpmnElement::ConditionalCatchEvent {
                condition: "flag".into(),
            },
        )
        .node("work", BpmnElement::UserTask("review".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("catch", "work")
        .flow("work", "end");
    if gateway {
        builder = builder
            .node("gateway", BpmnElement::EventBasedGateway)
            .node(
                "message",
                BpmnElement::MessageCatchEvent {
                    message_name: "release".into(),
                },
            )
            .node("other", BpmnElement::UserTask("message".into()))
            .flow("start", "gateway")
            .flow("gateway", "catch")
            .flow("gateway", "message")
            .flow("message", "other")
            .flow("other", "end");
    } else {
        builder = builder.flow("start", "catch");
    }
    builder.build().unwrap()
}

#[tokio::test]
async fn catch_waits_then_consumes_once_and_initial_true_passes() {
    for initial in [false, true] {
        let engine = WorkflowEngine::new();
        let (key, _) = engine.deploy_definition(definition(false)).await;
        let id = engine
            .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(initial))]))
            .await
            .unwrap();
        assert_eq!(engine.get_pending_user_tasks().len(), usize::from(initial));
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .unwrap();
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .unwrap();
        assert_eq!(engine.get_pending_user_tasks().len(), 1);
        let task = engine.get_pending_user_tasks().remove(0);
        engine
            .complete_user_task(task.task_id, HashMap::new())
            .await
            .unwrap();
        assert_eq!(
            engine.get_instance_state(id).await.unwrap(),
            InstanceState::Completed
        );
    }
}

#[tokio::test]
async fn gateway_conditional_or_message_wins_once() {
    for message in [false, true] {
        let engine = WorkflowEngine::new();
        let (key, _) = engine.deploy_definition(definition(true)).await;
        let id = engine.start_instance(key).await.unwrap();
        if message {
            engine
                .correlate_message("release".into(), None, HashMap::new())
                .await
                .unwrap();
        }
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .unwrap();
        engine
            .correlate_message("release".into(), None, HashMap::new())
            .await
            .unwrap();
        assert_eq!(engine.get_pending_user_tasks().len(), 1);
        assert_eq!(
            engine.get_pending_user_tasks()[0].node_id,
            if message { "other" } else { "work" }
        );
        assert!(engine.get_pending_message_catches().is_empty());
    }
}

async fn restored(persistence: Arc<crate::adapter::InMemoryPersistence>) -> WorkflowEngine {
    use crate::persistence::WorkflowPersistence;
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    for definition in persistence.list_definitions().await.unwrap() {
        engine.deploy_definition(definition).await;
    }
    for instance in persistence.list_instances().await.unwrap() {
        engine.restore_instance(instance).await;
    }
    for task in persistence.list_user_tasks().await.unwrap() {
        engine.restore_user_task(task);
    }
    for job in persistence.list_message_catches().await.unwrap() {
        engine.restore_message_catch(job);
    }
    for job in persistence.list_timers().await.unwrap() {
        engine.restore_timer(job);
    }
    engine
}

#[tokio::test]
async fn waiting_and_claimed_catches_recover_without_repeating_listeners() {
    use crate::persistence::WorkflowPersistence;
    for claimed in [false, true] {
        let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
        let engine = WorkflowEngine::new().with_persistence(persistence.clone());
        let mut def = definition(false);
        def.variable_defaults.insert("starts".into(), json!(0));
        def.variable_defaults.insert("ends".into(), json!(0));
        def.listeners.insert(
            "catch".into(),
            vec![
                crate::domain::ExecutionListener {
                    event: crate::domain::ListenerEvent::Start,
                    script: "starts += 1;".into(),
                },
                crate::domain::ExecutionListener {
                    event: crate::domain::ListenerEvent::End,
                    script: "ends += 1;".into(),
                },
            ],
        );
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine.start_instance(key).await.unwrap();
        if claimed {
            let instance = engine.instances.get(&id).await.unwrap();
            let mut instance = instance.write().await;
            instance.variables.insert("flag".into(), json!(true));
            WorkflowEngine::evaluate_conditionals(
                &mut instance,
                &engine.get_definition(&key).await.unwrap(),
            )
            .unwrap();
            instance.variables.insert("flag".into(), json!(false));
            instance
                .event_state
                .as_mut()
                .unwrap()
                .ready_conditional_tokens
                .clear();
            persistence.save_instance(&instance).await.unwrap();
        }
        drop(engine);
        let engine = restored(persistence).await;
        engine.reconcile_event_subprocesses().await.unwrap();
        engine.reconcile_event_subprocesses().await.unwrap();
        assert_eq!(engine.get_pending_user_tasks().len(), usize::from(claimed));
        if !claimed {
            engine
                .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
                .await
                .unwrap();
        }
        let instance = engine.get_instance_details(id).await.unwrap();
        assert_eq!(instance.variables["starts"], 1);
        assert_eq!(instance.variables["ends"], 1);
    }
}

#[tokio::test]
async fn suspended_catch_recovery_evaluates_on_resume() {
    let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine.deploy_definition(definition(false)).await;
    let id = engine.start_instance(key).await.unwrap();
    engine.suspend_instance(id).await.unwrap();
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    drop(engine);
    let engine = restored(persistence).await;
    engine.reconcile_event_subprocesses().await.unwrap();
    assert!(engine.get_pending_user_tasks().is_empty());
    engine.resume_instance(id).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
}

#[tokio::test]
async fn gateway_jobs_recover_original_ids_and_only_one_racing_winner() {
    use crate::persistence::WorkflowPersistence;
    let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine.deploy_definition(definition(true)).await;
    let id = engine.start_instance(key).await.unwrap();
    let job = engine.get_pending_message_catches().remove(0);
    persistence.delete_message_catch(job.id).await.unwrap();
    drop(engine);
    let engine = restored(persistence).await;
    engine.reconcile_event_subprocesses().await.unwrap();
    assert_eq!(engine.get_pending_message_catches()[0].id, job.id);
    let (condition, message) = tokio::join!(
        engine.update_instance_variables(id, HashMap::from([("flag".into(), json!(true))])),
        engine.correlate_message("release".into(), None, HashMap::new())
    );
    condition.unwrap();
    message.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
}

#[tokio::test]
async fn loop_revisit_while_true_creates_a_fresh_wait() {
    let engine = WorkflowEngine::new();
    let mut def = definition(false);
    def.nodes.insert(
        "route".into(),
        BpmnElement::ExclusiveGateway {
            default: Some("catch".into()),
        },
    );
    def.flows.insert(
        "work".into(),
        vec![crate::domain::SequenceFlow::simple("route")],
    );
    def.flows.insert(
        "route".into(),
        vec![
            crate::domain::SequenceFlow::conditional("end", "done"),
            crate::domain::SequenceFlow::simple("catch"),
        ],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    let first = engine.get_pending_user_tasks().remove(0);
    engine
        .complete_user_task(first.task_id, HashMap::new())
        .await
        .unwrap();
    let second = engine.get_pending_user_tasks().remove(0);
    assert_ne!(first.task_id, second.task_id);
    engine
        .complete_user_task(
            second.task_id,
            HashMap::from([("done".into(), json!(true))]),
        )
        .await
        .unwrap();
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn parallel_and_repeated_handler_catches_have_distinct_occurrences() {
    use crate::domain::EventSubProcessTrigger;
    let engine = WorkflowEngine::new();
    let mut def =
        super::event_subprocess::definition(EventSubProcessTrigger::Message("alert".into()), false);
    def.nodes.insert(
        "work".into(),
        BpmnElement::ConditionalCatchEvent {
            condition: "flag".into(),
        },
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    for _ in 0..2 {
        engine
            .correlate_message("alert".into(), None, HashMap::new())
            .await
            .unwrap();
    }
    assert_eq!(engine.get_pending_conditional_catches().await.len(), 2);
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert!(engine.get_pending_conditional_catches().await.is_empty());
    let main = engine.get_pending_user_tasks().remove(0);
    engine
        .complete_user_task(main.task_id, HashMap::new())
        .await
        .unwrap();
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn conditional_end_listener_failure_is_atomic_and_cancels_waits() {
    let engine = WorkflowEngine::new();
    let mut def = definition(false);
    def.variable_defaults.insert("ends".into(), json!(0));
    def.listeners.insert(
        "catch".into(),
        vec![crate::domain::ExecutionListener {
            event: crate::domain::ListenerEvent::End,
            script: "ends += 1; throw \"failure\";".into(),
        }],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    assert!(
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .is_err()
    );
    let instance = engine.get_instance_details(id).await.unwrap();
    assert!(matches!(
        instance.state,
        InstanceState::CompletedWithError { .. }
    ));
    assert_eq!(instance.variables["ends"], 0);
    assert!(engine.get_pending_conditional_catches().await.is_empty());
}

#[tokio::test]
async fn timer_and_condition_share_one_gateway_claim() {
    for initial in [false, true] {
        let engine = WorkflowEngine::new();
        let mut def = definition(true);
        def.nodes.insert(
            "message".into(),
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(Duration::ZERO)),
        );
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine
            .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(initial))]))
            .await
            .unwrap();
        engine.process_timers().await.unwrap();
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .unwrap();
        assert_eq!(engine.get_pending_user_tasks().len(), 1);
        assert_eq!(
            engine.get_pending_user_tasks()[0].node_id,
            if initial { "work" } else { "other" }
        );
    }
}

#[tokio::test]
async fn legacy_gateway_waits_upgrade_or_reject_ambiguous_restore() {
    use crate::persistence::WorkflowPersistence;
    for missing in [false, true] {
        let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
        let engine = WorkflowEngine::new().with_persistence(persistence.clone());
        let mut def = definition(true);
        def.nodes.insert(
            "catch".into(),
            BpmnElement::TimerCatchEvent(crate::domain::TimerDefinition::Duration(
                Duration::from_secs(60),
            )),
        );
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine.start_instance(key).await.unwrap();
        let mut snapshot = engine.get_instance_details(id).await.unwrap();
        snapshot.event_state.as_mut().unwrap().gateway_waits.clear();
        persistence.save_instance(&snapshot).await.unwrap();
        if missing {
            for job in persistence.list_timers().await.unwrap() {
                persistence.delete_timer(job.id).await.unwrap();
            }
            for job in persistence.list_message_catches().await.unwrap() {
                persistence.delete_message_catch(job.id).await.unwrap();
            }
        }
        drop(engine);
        let engine = restored(persistence).await;
        let result = engine.reconcile_event_subprocesses().await;
        if missing {
            assert!(result.is_err());
        } else {
            result.unwrap();
            engine
                .correlate_message("release".into(), None, HashMap::new())
                .await
                .unwrap();
            assert_eq!(engine.get_pending_user_tasks().len(), 1);
            assert_eq!(engine.get_pending_user_tasks()[0].node_id, "other");
        }
    }
}

#[tokio::test]
async fn signal_gateway_claim_and_handler_termination_remove_other_waits() {
    let engine = WorkflowEngine::new();
    let mut def = definition(true);
    def.nodes.insert(
        "message".into(),
        BpmnElement::SignalCatchEvent {
            signal_name: "release".into(),
        },
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    engine
        .broadcast_signal("release", HashMap::new())
        .await
        .unwrap();
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "other");
    let mut def = super::event_subprocess::definition(
        crate::domain::EventSubProcessTrigger::Message("alert".into()),
        false,
    );
    def.nodes.insert(
        "work".into(),
        BpmnElement::ConditionalCatchEvent {
            condition: "flag".into(),
        },
    );
    def.nodes
        .insert("end".into(), BpmnElement::TerminateEndEvent);
    let (key, _) = engine.deploy_definition(def).await;
    let handler_id = engine.start_instance(key).await.unwrap();
    engine
        .correlate_message("alert".into(), None, HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_conditional_catches().await.len(), 1);
    let main = engine
        .get_pending_user_tasks()
        .into_iter()
        .find(|task| task.instance_id == handler_id)
        .unwrap();
    engine
        .complete_user_task(main.task_id, HashMap::new())
        .await
        .unwrap();
    assert!(engine.get_pending_conditional_catches().await.is_empty());
    assert_eq!(
        engine.get_instance_state(handler_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn replayed_old_gateway_claim_cannot_repeat_callbacks_or_replace_continuation() {
    let engine = WorkflowEngine::new();
    let mut def = definition(true);
    def.variable_defaults.insert("starts".into(), json!(0));
    def.variable_defaults.insert("ends".into(), json!(0));
    def.listeners.insert(
        "catch".into(),
        vec![
            crate::domain::ExecutionListener {
                event: crate::domain::ListenerEvent::Start,
                script: "starts += 1;".into(),
            },
            crate::domain::ExecutionListener {
                event: crate::domain::ListenerEvent::End,
                script: "ends += 1;".into(),
            },
        ],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let instance = engine.instances.get(&id).await.unwrap();
    let stale = {
        let mut instance = instance.write().await;
        instance.variables.insert("flag".into(), json!(true));
        WorkflowEngine::evaluate_conditionals(
            &mut instance,
            &engine.get_definition(&key).await.unwrap(),
        )
        .unwrap();
        instance.tokens.values().next().unwrap().clone()
    };
    engine.run_conditional_work(id).await.unwrap();
    engine.run_instance_batch(id, stale.clone()).await.unwrap();
    let instance = engine.get_instance_details(id).await.unwrap();
    assert_eq!(instance.variables["starts"], 1);
    assert_eq!(instance.variables["ends"], 1);
    assert_eq!(instance.tokens[&stale.id].current_node, "work");
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
}

#[tokio::test]
async fn competing_conditional_alternatives_cancel_losers_before_claim_recovery() {
    use crate::persistence::WorkflowPersistence;
    let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let mut def = definition(true);
    def.nodes.insert(
        "message".into(),
        BpmnElement::ConditionalCatchEvent {
            condition: "flag == true".into(),
        },
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    {
        let instance = engine.instances.get(&id).await.unwrap();
        let mut instance = instance.write().await;
        instance.variables.insert("flag".into(), json!(true));
        WorkflowEngine::evaluate_conditionals(
            &mut instance,
            &engine.get_definition(&key).await.unwrap(),
        )
        .unwrap();
        let state = instance.event_state.as_mut().unwrap();
        assert_eq!(state.conditional_waits.len(), 1);
        state.ready_conditional_tokens.clear();
        persistence.save_instance(&instance).await.unwrap();
    }
    drop(engine);
    let engine = restored(persistence).await;
    engine.reconcile_event_subprocesses().await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "work");
}

#[tokio::test]
async fn suspended_recovery_resumes_a_consumed_catch_with_missing_continuation_job() {
    use crate::persistence::WorkflowPersistence;
    let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine.deploy_definition(definition(false)).await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    let task = engine.get_pending_user_tasks().remove(0);
    persistence.delete_user_task(task.task_id).await.unwrap();
    let mut snapshot = engine.get_instance_details(id).await.unwrap();
    snapshot.state = InstanceState::Suspended {
        previous_state: Box::new(InstanceState::Running),
    };
    persistence.save_instance(&snapshot).await.unwrap();
    drop(engine);
    let engine = restored(persistence).await;
    engine.reconcile_event_subprocesses().await.unwrap();
    assert!(engine.get_pending_user_tasks().is_empty());
    engine.resume_instance(id).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "work");
}

#[tokio::test]
async fn stale_gateway_job_cannot_claim_or_cancel_a_later_loop_occurrence() {
    let engine = WorkflowEngine::new();
    let mut def = definition(true);
    def.nodes.insert(
        "route".into(),
        BpmnElement::ExclusiveGateway {
            default: Some("gateway".into()),
        },
    );
    def.flows.insert(
        "work".into(),
        vec![crate::domain::SequenceFlow::simple("route")],
    );
    def.flows.insert(
        "route".into(),
        vec![
            crate::domain::SequenceFlow::conditional("end", "done"),
            crate::domain::SequenceFlow::simple("gateway"),
        ],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let stale = engine.get_pending_message_catches().remove(0);
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    let task = engine.get_pending_user_tasks().remove(0);
    engine
        .complete_user_task(task.task_id, HashMap::from([("flag".into(), json!(false))]))
        .await
        .unwrap();
    let current = engine.get_pending_message_catches().remove(0);
    assert_ne!(stale.id, current.id);
    engine.restore_message_catch(stale.clone());
    engine
        .resume_catch(stale.id, &HashMap::new(), "stale message")
        .await
        .unwrap();
    assert!(engine.get_pending_user_tasks().is_empty());
    assert_eq!(engine.get_pending_message_catches()[0].id, current.id);
    engine
        .correlate_message("release".into(), None, HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "other");
}

#[tokio::test]
async fn parallel_conditional_catches_preserve_one_join_continuation() {
    let engine = WorkflowEngine::new();
    let mut def = definition(false);
    def.nodes
        .insert("fork".into(), BpmnElement::ParallelGateway);
    def.nodes
        .insert("join".into(), BpmnElement::ParallelGateway);
    def.nodes.insert(
        "second".into(),
        BpmnElement::ConditionalCatchEvent {
            condition: "flag == true".into(),
        },
    );
    def.flows.insert(
        "start".into(),
        vec![crate::domain::SequenceFlow::simple("fork")],
    );
    def.flows.insert(
        "fork".into(),
        vec![
            crate::domain::SequenceFlow::simple("catch"),
            crate::domain::SequenceFlow::simple("second"),
        ],
    );
    for node in ["catch", "second"] {
        def.flows.insert(
            node.into(),
            vec![crate::domain::SequenceFlow::simple("join")],
        );
    }
    def.flows.insert(
        "join".into(),
        vec![crate::domain::SequenceFlow::simple("work")],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    assert_eq!(engine.get_pending_conditional_catches().await.len(), 2);
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    let task = engine.get_pending_user_tasks().remove(0);
    engine
        .complete_user_task(task.task_id, HashMap::new())
        .await
        .unwrap();
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}
