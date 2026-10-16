use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use serde_json::json;

fn definition(interrupting: bool) -> ProcessDefinition {
    ProcessDefinitionBuilder::new("boundary_conditions")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("main".into()))
        .node(
            "boundary",
            BpmnElement::BoundaryConditionalEvent {
                attached_to: "task".into(),
                condition: "flag".into(),
                cancel_activity: interrupting,
                order: 0,
            },
        )
        .node("branch", BpmnElement::UserTask("branch".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .flow("boundary", "branch")
        .flow("branch", "end")
        .build()
        .unwrap()
}

#[tokio::test]
async fn boundary_initial_and_later_true_interrupt_once() {
    for initial in [false, true] {
        let engine = WorkflowEngine::new();
        let (key, _) = engine.deploy_definition(definition(true)).await;
        let id = engine
            .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(initial))]))
            .await
            .unwrap();
        let original = engine
            .get_pending_user_tasks()
            .into_iter()
            .find(|task| task.node_id == "task");
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .unwrap();
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .unwrap();
        let tasks = engine.get_pending_user_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].node_id, "branch");
        if let Some(original) = original {
            assert!(
                engine
                    .complete_user_task(original.task_id, HashMap::new())
                    .await
                    .is_err()
            );
        }
        engine
            .complete_user_task(tasks[0].task_id, HashMap::new())
            .await
            .unwrap();
        assert_eq!(
            engine.get_instance_state(id).await.unwrap(),
            InstanceState::Completed
        );
    }
}

#[tokio::test]
async fn boundary_non_interrupting_rearms_and_survives_activity_completion() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(definition(false)).await;
    let id = engine.start_instance(key).await.unwrap();
    for value in [true, true, false, true] {
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(value))]))
            .await
            .unwrap();
    }
    assert_eq!(engine.get_pending_user_tasks().len(), 3);
    let main = engine
        .get_pending_user_tasks()
        .into_iter()
        .find(|task| task.node_id == "task")
        .unwrap();
    engine
        .complete_user_task(main.task_id, HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 2);
    for value in [false, true] {
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(value))]))
            .await
            .unwrap();
    }
    assert_eq!(engine.get_pending_user_tasks().len(), 2);
    for task in engine.get_pending_user_tasks() {
        engine
            .complete_user_task(task.task_id, HashMap::new())
            .await
            .unwrap();
    }
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn boundary_completion_outputs_do_not_interrupt_completed_activity() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(definition(true)).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = engine.get_pending_user_tasks()[0].clone();
    engine
        .complete_user_task(task.task_id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert!(engine.get_pending_user_tasks().is_empty());
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}

async fn restore(persistence: Arc<crate::adapter::InMemoryPersistence>) -> WorkflowEngine {
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
    for timer in persistence.list_timers().await.unwrap() {
        engine.restore_timer(timer);
    }
    for message in persistence.list_message_catches().await.unwrap() {
        engine.restore_message_catch(message);
    }
    engine
}

#[tokio::test]
async fn boundary_wait_claim_and_consumed_continuation_recover() {
    use crate::persistence::WorkflowPersistence;
    for point in 0..3 {
        let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
        let engine = WorkflowEngine::new().with_persistence(persistence.clone());
        let (key, _) = engine.deploy_definition(definition(true)).await;
        let id = engine.start_instance(key).await.unwrap();
        if point > 0 {
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
        if point == 2 {
            let instance = engine.instances.get(&id).await.unwrap();
            let mut snapshot = instance.read().await.clone();
            let mut token = snapshot.tokens.values().next().unwrap().clone();
            token.current_node = "branch".into();
            token.conditional_wait = None;
            snapshot
                .event_state
                .as_mut()
                .unwrap()
                .conditional_waits
                .clear();
            snapshot.tokens.insert(token.id, token);
            persistence.save_instance(&snapshot).await.unwrap();
        }
        drop(engine);
        let engine = restore(persistence).await;
        engine.reconcile_event_subprocesses().await.unwrap();
        engine.reconcile_event_subprocesses().await.unwrap();
        if point == 0 {
            engine
                .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
                .await
                .unwrap();
        }
        let tasks = engine.get_pending_user_tasks();
        assert_eq!(tasks.len(), 1, "crash point {point}");
        assert_eq!(tasks[0].node_id, "branch");
    }
}

#[tokio::test]
async fn boundary_suspend_preserves_wait_and_claim() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(definition(true)).await;
    let id = engine.start_instance(key).await.unwrap();
    engine.suspend_instance(id).await.unwrap();
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "task");
    engine.resume_instance(id).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "branch");
}

#[tokio::test]
async fn boundary_claim_fences_an_inflight_removed_task_token() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(definition(true)).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = engine.get_pending_user_tasks()[0].clone();
    let instance = engine.instances.get(&id).await.unwrap();
    let mut stale = instance
        .write()
        .await
        .tokens
        .remove(&task.token_id)
        .unwrap();
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert!(matches!(
        engine
            .run_end_scripts(
                id,
                &mut stale,
                &engine.get_definition(&key).await.unwrap(),
                "task"
            )
            .await,
        Err(EngineError::AlreadyCompleted)
    ));
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "branch");
}

#[tokio::test]
async fn boundary_declaration_order_stops_after_interrupting_winner() {
    let engine = WorkflowEngine::new();
    let mut def = definition(true);
    def.nodes.insert(
        "earlier".into(),
        BpmnElement::BoundaryConditionalEvent {
            attached_to: "task".into(),
            condition: "flag".into(),
            cancel_activity: false,
            order: 0,
        },
    );
    if let BpmnElement::BoundaryConditionalEvent { order, .. } =
        def.nodes.get_mut("boundary").unwrap()
    {
        *order = 1;
    }
    def.flows
        .insert("earlier".into(), def.flows["boundary"].clone());
    let (key, _) = engine.deploy_definition(def).await;
    engine
        .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 2);
    assert!(
        engine
            .get_pending_user_tasks()
            .iter()
            .all(|task| task.node_id == "branch")
    );
}

#[tokio::test]
async fn boundary_non_interrupting_latch_survives_restore() {
    let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine.deploy_definition(definition(false)).await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 2);
    drop(engine);
    let engine = restore(persistence).await;
    engine.reconcile_event_subprocesses().await.unwrap();
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 2);
    for value in [false, true] {
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(value))]))
            .await
            .unwrap();
    }
    assert_eq!(engine.get_pending_user_tasks().len(), 3);
}

#[tokio::test]
async fn boundary_task_revisit_has_fresh_activation_and_fences_old_callbacks() {
    let engine = WorkflowEngine::new();
    let mut def = definition(false);
    def.nodes.insert(
        "gate".into(),
        BpmnElement::ExclusiveGateway {
            default: Some("end".into()),
        },
    );
    def.flows.insert(
        "task".into(),
        vec![crate::domain::SequenceFlow::simple("gate")],
    );
    def.flows.insert(
        "gate".into(),
        vec![
            crate::domain::SequenceFlow::conditional("task", "again"),
            crate::domain::SequenceFlow::simple("end"),
        ],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine
        .start_instance_with_variables(
            key,
            HashMap::from([("flag".into(), json!(true)), ("again".into(), json!(true))]),
        )
        .await
        .unwrap();
    let task = engine
        .get_pending_user_tasks()
        .into_iter()
        .find(|task| task.node_id == "task")
        .unwrap();
    let mut stale = engine.get_instance_details(id).await.unwrap().tokens[&task.token_id].clone();
    engine
        .complete_user_task(task.task_id, HashMap::new())
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 3);
    let current = engine.get_instance_details(id).await.unwrap();
    assert_ne!(
        current.tokens[&task.token_id].boundary_activation,
        stale.boundary_activation
    );
    assert!(matches!(
        engine
            .run_end_scripts(
                id,
                &mut stale,
                &engine.get_definition(&key).await.unwrap(),
                "task"
            )
            .await,
        Err(EngineError::AlreadyCompleted)
    ));
}

#[tokio::test]
async fn boundary_parallel_handler_activations_are_isolated() {
    let engine = WorkflowEngine::new();
    let mut def = super::event_subprocess::definition(
        crate::domain::EventSubProcessTrigger::Message("alert".into()),
        false,
    );
    def.nodes.insert(
        "boundary".into(),
        BpmnElement::BoundaryConditionalEvent {
            attached_to: "work".into(),
            condition: "flag".into(),
            cancel_activity: true,
            order: 0,
        },
    );
    def.flows.insert(
        "boundary".into(),
        vec![crate::domain::SequenceFlow::simple("he")],
    );
    def.event_subprocesses[0].node_ids.push("boundary".into());
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
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    let main = engine.get_pending_user_tasks()[0].clone();
    assert_eq!(main.node_id, "main");
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
async fn boundary_failed_end_listener_keeps_subscription_and_discards_writes() {
    let engine = WorkflowEngine::new();
    let mut def = definition(true);
    def.listeners.insert(
        "task".into(),
        vec![crate::domain::ExecutionListener {
            event: crate::domain::ListenerEvent::End,
            script: "flag = true; throw \"failure\";".into(),
        }],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(false))]))
        .await
        .unwrap();
    let task = engine.get_pending_user_tasks()[0].clone();
    assert!(
        engine
            .complete_user_task(task.task_id, HashMap::new())
            .await
            .is_err()
    );
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().variables["flag"],
        false
    );
    assert_eq!(engine.get_pending_conditional_catches().await.len(), 1);
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "branch");
}

#[tokio::test]
async fn boundary_receive_completion_and_interrupt_are_exclusive() {
    for interrupt in [false, true] {
        let engine = WorkflowEngine::new();
        let mut def = definition(true);
        def.nodes.insert(
            "task".into(),
            BpmnElement::MessageCatchEvent {
                message_name: "received".into(),
            },
        );
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine.start_instance(key).await.unwrap();
        assert_eq!(engine.get_pending_conditional_catches().await.len(), 1);
        if interrupt {
            engine
                .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
                .await
                .unwrap();
        }
        engine
            .correlate_message(
                "received".into(),
                None,
                HashMap::from([("flag".into(), json!(true))]),
            )
            .await
            .unwrap();
        assert!(engine.get_pending_conditional_catches().await.is_empty());
        assert_eq!(
            engine.get_pending_user_tasks().len(),
            usize::from(interrupt)
        );
        assert!(engine.get_pending_message_catches().is_empty());
        if !interrupt {
            assert_eq!(
                engine.get_instance_state(id).await.unwrap(),
                InstanceState::Completed
            );
        }
    }
}

#[tokio::test]
async fn boundary_sibling_signal_or_timer_winner_clears_conditional_activation() {
    for timer in [false, true] {
        let engine = WorkflowEngine::new();
        let mut def = definition(true);
        def.nodes.insert(
            "sibling".into(),
            if timer {
                BpmnElement::BoundaryTimerEvent {
                    attached_to: "task".into(),
                    timer: crate::domain::TimerDefinition::Duration(std::time::Duration::ZERO),
                    cancel_activity: true,
                }
            } else {
                BpmnElement::BoundarySignalEvent {
                    attached_to: "task".into(),
                    signal_name: "cancel".into(),
                    cancel_activity: true,
                }
            },
        );
        def.flows
            .insert("sibling".into(), def.flows["boundary"].clone());
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine.start_instance(key).await.unwrap();
        if timer {
            engine.process_timers().await.unwrap();
        } else {
            engine
                .broadcast_signal("cancel", HashMap::new())
                .await
                .unwrap();
        }
        engine
            .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
            .await
            .unwrap();
        assert!(engine.get_pending_conditional_catches().await.is_empty());
        assert_eq!(engine.get_pending_user_tasks().len(), 1);
        assert_eq!(engine.get_pending_user_tasks()[0].node_id, "branch");
    }
}

#[tokio::test]
async fn boundary_initial_claim_skips_script_body_and_script_outputs_close_activation() {
    for initial in [false, true] {
        let engine = WorkflowEngine::new();
        let mut def = definition(true);
        def.nodes.insert(
            "task".into(),
            BpmnElement::ScriptTask {
                script: "runs += 1; flag = true;".into(),
                multi_instance: None,
            },
        );
        def.variable_defaults.insert("runs".into(), json!(0));
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine
            .start_instance_with_variables(key, HashMap::from([("flag".into(), json!(initial))]))
            .await
            .unwrap();
        assert_eq!(
            engine.get_instance_details(id).await.unwrap().variables["runs"],
            usize::from(!initial)
        );
        assert_eq!(engine.get_pending_user_tasks().len(), usize::from(initial));
        assert!(engine.get_pending_conditional_catches().await.is_empty());
    }
}

#[tokio::test]
async fn boundary_claim_and_native_completion_race_has_one_continuation() {
    for _ in 0..12 {
        let engine = WorkflowEngine::new();
        let (key, _) = engine.deploy_definition(definition(true)).await;
        let id = engine.start_instance(key).await.unwrap();
        let task = engine.get_pending_user_tasks()[0].clone();
        let (_, _) = tokio::join!(
            engine.complete_user_task(task.task_id, HashMap::new()),
            engine.update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        );
        let tasks = engine.get_pending_user_tasks();
        assert!(tasks.len() <= 1);
        assert!(tasks.iter().all(|task| task.node_id == "branch"));
        assert!(engine.get_pending_conditional_catches().await.is_empty());
        if tasks.is_empty() {
            assert_eq!(
                engine.get_instance_state(id).await.unwrap(),
                InstanceState::Completed
            );
        }
    }
}

#[tokio::test]
async fn boundary_legacy_wait_reconstructs_without_markers() {
    use crate::persistence::WorkflowPersistence;
    let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let (key, _) = engine.deploy_definition(definition(true)).await;
    let id = engine.start_instance(key).await.unwrap();
    let mut snapshot = engine.get_instance_details(id).await.unwrap();
    snapshot
        .event_state
        .as_mut()
        .unwrap()
        .boundary_activations
        .clear();
    for token in snapshot.tokens.values_mut() {
        token.boundary_activation = None;
    }
    persistence.save_instance(&snapshot).await.unwrap();
    drop(engine);
    let engine = restore(persistence).await;
    engine.reconcile_event_subprocesses().await.unwrap();
    assert_eq!(engine.get_pending_conditional_catches().await.len(), 1);
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "branch");
}

#[tokio::test]
async fn boundary_interrupting_branch_preserves_parallel_join() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("boundary_join")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node("task", BpmnElement::UserTask("main".into()))
        .node("other", BpmnElement::UserTask("other".into()))
        .node(
            "boundary",
            BpmnElement::BoundaryConditionalEvent {
                attached_to: "task".into(),
                condition: "flag".into(),
                cancel_activity: true,
                order: 0,
            },
        )
        .node("branch", BpmnElement::UserTask("branch".into()))
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("fork", "task")
        .flow("fork", "other")
        .flow("task", "join")
        .flow("other", "join")
        .flow("boundary", "branch")
        .flow("branch", "join")
        .flow("join", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 2);
    for task in tasks {
        engine
            .complete_user_task(task.task_id, HashMap::new())
            .await
            .unwrap();
    }
    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn boundary_old_message_job_cannot_cancel_same_node_revisit() {
    let engine = WorkflowEngine::new();
    let mut def = definition(true);
    def.nodes.insert(
        "sibling".into(),
        BpmnElement::BoundaryMessageEvent {
            attached_to: "task".into(),
            message_name: "cancel".into(),
            cancel_activity: true,
        },
    );
    def.flows
        .insert("sibling".into(), def.flows["boundary"].clone());
    def.nodes.insert(
        "gate".into(),
        BpmnElement::ExclusiveGateway {
            default: Some("end".into()),
        },
    );
    def.flows.insert(
        "task".into(),
        vec![crate::domain::SequenceFlow::simple("gate")],
    );
    def.flows.insert(
        "gate".into(),
        vec![
            crate::domain::SequenceFlow::conditional("task", "again"),
            crate::domain::SequenceFlow::simple("end"),
        ],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("again".into(), json!(true))]))
        .await
        .unwrap();
    let original = engine.get_pending_message_catches()[0].clone();
    let task = engine.get_pending_user_tasks()[0].clone();
    engine
        .complete_user_task(task.task_id, HashMap::new())
        .await
        .unwrap();
    let current = engine.get_pending_message_catches()[0].clone();
    assert_ne!(original.id, current.id);
    engine.restore_message_catch(original.clone());
    engine
        .resume_catch(original.id, &HashMap::new(), "stale delivery")
        .await
        .unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.get_pending_user_tasks()[0].node_id, "task");
    assert_eq!(engine.get_pending_message_catches().len(), 1);
    assert_eq!(engine.get_pending_message_catches()[0].id, current.id);
    assert_eq!(engine.get_pending_conditional_catches().await.len(), 1);
    assert!(
        engine
            .get_instance_details(id)
            .await
            .unwrap()
            .event_state
            .unwrap()
            .boundary_activations
            .values()
            .all(|activation| activation.jobs.contains(&current.id))
    );
}

#[tokio::test]
async fn boundary_recovery_restores_original_sibling_deadline_before_task_publication() {
    for suspended in [false, true] {
        let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
        let engine = WorkflowEngine::new().with_persistence(persistence.clone());
        let mut def = definition(true);
        def.nodes.insert(
            "sibling".into(),
            BpmnElement::BoundaryTimerEvent {
                attached_to: "task".into(),
                timer: crate::domain::TimerDefinition::Duration(std::time::Duration::from_secs(
                    3600,
                )),
                cancel_activity: true,
            },
        );
        def.flows
            .insert("sibling".into(), def.flows["boundary"].clone());
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine.start_instance(key).await.unwrap();
        let timer = engine.pending_timers.iter().next().unwrap().clone();
        let task = engine.get_pending_user_tasks()[0].clone();
        engine.remove_persisted_user_task(task.task_id).await;
        engine.remove_persisted_timer(timer.id).await;
        if suspended {
            engine.suspend_instance(id).await.unwrap();
        }
        drop(engine);
        let engine = restore(persistence).await;
        engine.reconcile_event_subprocesses().await.unwrap();
        if suspended {
            engine.resume_instance(id).await.unwrap();
        }
        assert_eq!(engine.get_pending_user_tasks().len(), 1);
        assert_eq!(engine.pending_timers.len(), 1);
        let restored = engine.pending_timers.get(&timer.id).unwrap();
        assert_eq!(restored.expires_at, timer.expires_at);
        assert_eq!(restored.token_id, timer.token_id);
        assert_eq!(engine.get_pending_conditional_catches().await.len(), 1);
        assert_eq!(
            engine
                .get_instance_details(id)
                .await
                .unwrap()
                .definition_key,
            key
        );
    }
}

#[tokio::test]
async fn boundary_cancelled_activation_cannot_register_again() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(definition(true)).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = engine.get_pending_user_tasks()[0].clone();
    let mut stale = engine.get_instance_details(id).await.unwrap().tokens[&task.token_id].clone();
    engine
        .update_instance_variables(id, HashMap::from([("flag".into(), json!(true))]))
        .await
        .unwrap();
    let instance = engine.instances.get(&id).await.unwrap();
    let mut instance = instance.write().await;
    WorkflowEngine::register_conditional_boundaries(
        &mut instance,
        &engine.get_definition(&key).await.unwrap(),
        &mut stale,
    );
    assert!(
        instance
            .event_state
            .as_ref()
            .unwrap()
            .boundary_activations
            .is_empty()
    );
    assert!(!super::super::event_subprocess::token_valid(
        &instance, &stale
    ));
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
}

#[tokio::test]
async fn boundary_recurring_timer_keeps_repetitions_with_conditional_definitions() {
    for owned in [false, true] {
        let engine = WorkflowEngine::new();
        let mut def = definition(true);
        if !owned {
            def.nodes
                .insert("unused".into(), BpmnElement::UserTask("unused".into()));
            def.flows.insert(
                "unused".into(),
                vec![crate::domain::SequenceFlow::simple("end")],
            );
            if let BpmnElement::BoundaryConditionalEvent { attached_to, .. } =
                def.nodes.get_mut("boundary").unwrap()
            {
                *attached_to = "unused".into();
            }
        }
        def.nodes.insert(
            "timer".into(),
            BpmnElement::BoundaryTimerEvent {
                attached_to: "task".into(),
                timer: crate::domain::TimerDefinition::RepeatingInterval {
                    repetitions: Some(2),
                    interval: std::time::Duration::ZERO,
                },
                cancel_activity: false,
            },
        );
        def.flows
            .insert("timer".into(), def.flows["boundary"].clone());
        let (key, _) = engine.deploy_definition(def).await;
        engine.start_instance(key).await.unwrap();
        engine.process_timers().await.unwrap();
        assert_eq!(engine.pending_timers.len(), 1, "owned={owned}");
        engine.process_timers().await.unwrap();
        assert_eq!(engine.pending_timers.len(), 0, "owned={owned}");
        assert_eq!(engine.get_pending_user_tasks().len(), 3, "owned={owned}");
    }
}

#[tokio::test]
async fn boundary_self_loop_completion_crash_removes_old_task_before_new_activation() {
    let persistence = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let mut def = definition(true);
    def.flows.insert(
        "task".into(),
        vec![crate::domain::SequenceFlow::simple("task")],
    );
    def.variable_defaults.insert("starts".into(), json!(0));
    def.listeners.insert(
        "task".into(),
        vec![crate::domain::ExecutionListener {
            event: crate::domain::ListenerEvent::Start,
            script: "starts += 1;".into(),
        }],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let old = engine.get_pending_user_tasks()[0].clone();
    let mut token = engine.get_instance_details(id).await.unwrap().tokens[&old.token_id].clone();
    engine
        .run_end_scripts(
            id,
            &mut token,
            &engine.get_definition(&key).await.unwrap(),
            "task",
        )
        .await
        .unwrap();
    engine.persist_instance(id).await;
    // Old task storage survives; its cleanup intent and next token are saved.
    drop(engine);
    let engine = restore(persistence).await;
    engine.reconcile_event_subprocesses().await.unwrap();
    engine.reconcile_event_subprocesses().await.unwrap();
    let current = engine.get_pending_user_tasks();
    assert_eq!(current.len(), 1);
    assert_ne!(current[0].task_id, old.task_id);
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().variables["starts"],
        2
    );
    assert!(
        engine
            .complete_user_task(old.task_id, HashMap::new())
            .await
            .is_err()
    );
    assert_eq!(engine.get_pending_conditional_catches().await.len(), 1);
}
