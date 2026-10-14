//! Multi-instance (X4) and standard loop (X5) activities.

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::{
    LoopCharacteristics, MultiInstanceDef, ProcessDefinitionBuilder, StandardLoopDef,
};
use serde_json::json;

fn mi(is_sequential: bool) -> MultiInstanceDef {
    MultiInstanceDef {
        is_sequential,
        loop_cardinality: None,
        collection: None,
        element_variable: None,
        completion_condition: None,
        output_collection: None,
        output_element: None,
    }
}

fn single_activity(id: &str, activity: BpmnElement, lc: LoopCharacteristics) -> ProcessDefinition {
    ProcessDefinitionBuilder::new(id)
        .node("start", BpmnElement::StartEvent)
        .node("act", activity)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "act")
        .flow("act", "end")
        .loop_characteristics("act", lc)
        .build()
        .unwrap()
}

fn script(source: &str) -> BpmnElement {
    BpmnElement::ScriptTask {
        script: source.into(),
        multi_instance: None,
    }
}

async fn details(engine: &WorkflowEngine, id: Uuid) -> ProcessInstance {
    engine.get_instance_details(id).await.unwrap()
}

#[tokio::test]
async fn parallel_collection_collects_outputs_in_order() {
    let engine = WorkflowEngine::new();
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        collection: Some("${items}".into()),
        element_variable: Some("item".into()),
        output_collection: Some("results".into()),
        output_element: Some("doubled".into()),
        ..mi(false)
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "mi_script",
            script("let doubled = item * 2;"),
            lc,
        ))
        .await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("items".into(), json!([1, 2, 3]))]))
        .await
        .unwrap();

    let inst = details(&engine, id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.variables.get("results"), Some(&json!([2, 4, 6])));
    for local in [
        "item",
        "loopCounter",
        "nrOfInstances",
        "nrOfCompletedInstances",
    ] {
        assert!(
            !inst.variables.contains_key(local),
            "{local} must stay local"
        );
    }
    assert!(inst.multi_instance_state.is_empty());
}

#[tokio::test]
async fn parallel_user_tasks_complete_after_all_instances() {
    let engine = WorkflowEngine::new();
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        loop_cardinality: Some("3".into()),
        ..mi(false)
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "mi_users",
            BpmnElement::UserTask("clerk".into()),
            lc,
        ))
        .await;
    let id = engine.start_instance(key).await.unwrap();

    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 3);
    for (n, task) in tasks.iter().enumerate() {
        assert_ne!(details(&engine, id).await.state, InstanceState::Completed);
        engine
            .complete_user_task(task.task_id, HashMap::from([(format!("v{n}"), json!(n))]))
            .await
            .unwrap();
    }

    let inst = details(&engine, id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    for n in 0..3 {
        assert_eq!(inst.variables.get(&format!("v{n}")), Some(&json!(n)));
    }
}

#[tokio::test]
async fn completion_condition_cancels_remaining_instances() {
    let engine = WorkflowEngine::new();
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        loop_cardinality: Some("${reviewers}".into()),
        completion_condition: Some("${nrOfCompletedInstances >= 2}".into()),
        ..mi(false)
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "mi_quorum",
            BpmnElement::UserTask("reviewer".into()),
            lc,
        ))
        .await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("reviewers".into(), json!(3))]))
        .await
        .unwrap();
    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 3);

    engine
        .complete_user_task(tasks[0].task_id, HashMap::new())
        .await
        .unwrap();
    engine
        .complete_user_task(tasks[1].task_id, HashMap::new())
        .await
        .unwrap();

    assert_eq!(details(&engine, id).await.state, InstanceState::Completed);
    assert!(
        engine.get_pending_user_tasks().is_empty(),
        "the third review must be cancelled"
    );
}

#[tokio::test]
async fn sequential_service_tasks_run_one_after_another() {
    let engine = WorkflowEngine::new();
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        collection: Some("orders".into()),
        element_variable: Some("order".into()),
        output_collection: Some("receipts".into()),
        output_element: Some("receipt".into()),
        ..mi(true)
    });
    let service = BpmnElement::ServiceTask {
        topic: "charge".into(),
        multi_instance: None,
    };
    let (key, _) = engine
        .deploy_definition(single_activity("mi_seq", service, lc))
        .await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("orders".into(), json!(["a", "b"]))]))
        .await
        .unwrap();

    for expected in ["a", "b"] {
        let tasks = engine
            .fetch_and_lock_service_tasks("w", 10, &["charge".into()], 60)
            .await;
        assert_eq!(tasks.len(), 1, "sequential: one instance at a time");
        assert_eq!(
            tasks[0].variables_snapshot.get("order"),
            Some(&json!(expected))
        );
        engine
            .complete_service_task(
                tasks[0].id,
                "w",
                HashMap::from([("receipt".into(), json!(format!("paid-{expected}")))]),
            )
            .await
            .unwrap();
    }

    let inst = details(&engine, id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(
        inst.variables.get("receipts"),
        Some(&json!(["paid-a", "paid-b"]))
    );
    assert!(!inst.variables.contains_key("order"));
}

#[tokio::test]
async fn zero_instances_skip_the_activity() {
    let engine = WorkflowEngine::new();
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        collection: Some("items".into()),
        ..mi(false)
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "mi_empty",
            BpmnElement::UserTask("x".into()),
            lc,
        ))
        .await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("items".into(), json!([]))]))
        .await
        .unwrap();
    assert_eq!(details(&engine, id).await.state, InstanceState::Completed);
    assert!(engine.get_pending_user_tasks().is_empty());
}

#[tokio::test]
async fn invalid_collection_fails_the_step() {
    let engine = WorkflowEngine::new();
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        collection: Some("items".into()),
        ..mi(false)
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "mi_bad",
            BpmnElement::UserTask("x".into()),
            lc,
        ))
        .await;
    let err = engine
        .start_instance_with_variables(key, HashMap::from([("items".into(), json!("nope"))]))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("is not an array"), "{err}");
}

#[tokio::test]
async fn standard_loop_repeats_while_condition_holds() {
    let engine = WorkflowEngine::new();
    let lc = LoopCharacteristics::Standard(StandardLoopDef {
        loop_condition: Some("counter < 3".into()),
        test_before: false,
        loop_maximum: None,
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "loop",
            script("counter = counter + 1;"),
            lc,
        ))
        .await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("counter".into(), json!(0))]))
        .await
        .unwrap();

    let inst = details(&engine, id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.variables.get("counter"), Some(&json!(3)));
    assert!(!inst.variables.contains_key("loopCounter"));
}

#[tokio::test]
async fn standard_loop_test_before_can_skip_and_maximum_caps() {
    let engine = WorkflowEngine::new();
    let skip = LoopCharacteristics::Standard(StandardLoopDef {
        loop_condition: Some("counter < 3".into()),
        test_before: true,
        loop_maximum: None,
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "loop_skip",
            script("counter = counter + 1;"),
            skip,
        ))
        .await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("counter".into(), json!(5))]))
        .await
        .unwrap();
    assert_eq!(
        details(&engine, id).await.variables.get("counter"),
        Some(&json!(5))
    );

    let capped = LoopCharacteristics::Standard(StandardLoopDef {
        loop_condition: Some("always".into()),
        test_before: false,
        loop_maximum: Some(4),
    });
    let (key, _) = engine
        .deploy_definition(single_activity(
            "loop_cap",
            script("counter = counter + 1;"),
            capped,
        ))
        .await;
    let id = engine
        .start_instance_with_variables(
            key,
            HashMap::from([("counter".into(), json!(0)), ("always".into(), json!(true))]),
        )
        .await
        .unwrap();
    assert_eq!(
        details(&engine, id).await.variables.get("counter"),
        Some(&json!(4))
    );
}

#[tokio::test]
async fn interrupting_boundary_cancels_all_instances() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("mi_boundary")
        .node("start", BpmnElement::StartEvent)
        .node("act", BpmnElement::UserTask("clerk".into()))
        .node(
            "abort",
            BpmnElement::BoundaryMessageEvent {
                attached_to: "act".into(),
                message_name: "abort".into(),
                cancel_activity: true,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .node("aborted", BpmnElement::EndEvent)
        .flow("start", "act")
        .flow("act", "end")
        .flow("abort", "aborted")
        .loop_characteristics(
            "act",
            LoopCharacteristics::MultiInstance(MultiInstanceDef {
                loop_cardinality: Some("3".into()),
                ..mi(false)
            }),
        )
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 3);

    engine
        .correlate_message("abort".into(), None, HashMap::new())
        .await
        .unwrap();

    let inst = details(&engine, id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.current_node, "aborted");
    assert!(engine.get_pending_user_tasks().is_empty());
    assert!(engine.pending_message_catches.is_empty());
}

#[tokio::test]
async fn parallel_call_activities_complete_parent() {
    let engine = WorkflowEngine::new();
    let child = ProcessDefinitionBuilder::new("mi_child")
        .node("start", BpmnElement::StartEvent)
        .node("work", BpmnElement::UserTask("child".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "work")
        .flow("work", "end")
        .build()
        .unwrap();
    engine.deploy_definition(child).await;
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        loop_cardinality: Some("2".into()),
        ..mi(false)
    });
    let call = BpmnElement::CallActivity {
        called_element: "mi_child".into(),
    };
    let (key, _) = engine
        .deploy_definition(single_activity("mi_parent", call, lc))
        .await;
    let parent = engine.start_instance(key).await.unwrap();

    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 2, "one child process per instance");
    for task in tasks {
        engine
            .complete_user_task(task.task_id, HashMap::new())
            .await
            .unwrap();
    }
    assert_eq!(
        details(&engine, parent).await.state,
        InstanceState::Completed
    );
}

#[tokio::test]
async fn parallel_progress_survives_restart() {
    let pers = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(pers.clone());
    let lc = LoopCharacteristics::MultiInstance(MultiInstanceDef {
        loop_cardinality: Some("2".into()),
        output_collection: Some("answers".into()),
        output_element: Some("answer".into()),
        ..mi(false)
    });
    let def = single_activity("mi_restart", BpmnElement::UserTask("x".into()), lc.clone());
    let def_key = def.key;
    engine.deploy_definition(def).await;
    let id = engine.start_instance(def_key).await.unwrap();
    let first = engine.get_pending_user_tasks()[0].clone();
    engine
        .complete_user_task(
            first.task_id,
            HashMap::from([("answer".into(), json!("first"))]),
        )
        .await
        .unwrap();
    drop(engine);

    let engine2 = WorkflowEngine::new().with_persistence(pers.clone());
    let mut redeploy = single_activity("mi_restart", BpmnElement::UserTask("x".into()), lc);
    redeploy.key = def_key;
    engine2.deploy_definition(redeploy).await;
    for instance in pers.list_instances().await.unwrap() {
        engine2.restore_instance(instance).await;
    }
    for task in pers.list_user_tasks().await.unwrap() {
        engine2.restore_user_task(task);
    }
    let remaining = engine2.get_pending_user_tasks();
    assert_eq!(remaining.len(), 1);
    engine2
        .complete_user_task(
            remaining[0].task_id,
            HashMap::from([("answer".into(), json!("second"))]),
        )
        .await
        .unwrap();

    let inst = details(&engine2, id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    let answers = inst
        .variables
        .get("answers")
        .and_then(Value::as_array)
        .unwrap();
    assert_eq!(answers.len(), 2);
    assert!(answers.contains(&json!("first")) && answers.contains(&json!("second")));
}

#[tokio::test]
async fn bpmn_error_boundary_on_parallel_service_tasks_cancels_siblings() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("mi_error")
        .node("start", BpmnElement::StartEvent)
        .node(
            "act",
            BpmnElement::ServiceTask {
                topic: "charge".into(),
                multi_instance: None,
            },
        )
        .node(
            "failed",
            BpmnElement::BoundaryErrorEvent {
                attached_to: "act".into(),
                error_code: Some("DECLINED".into()),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .node("error_end", BpmnElement::EndEvent)
        .flow("start", "act")
        .flow("act", "end")
        .flow("failed", "error_end")
        .loop_characteristics(
            "act",
            LoopCharacteristics::MultiInstance(MultiInstanceDef {
                loop_cardinality: Some("3".into()),
                ..mi(false)
            }),
        )
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();

    let tasks = engine
        .fetch_and_lock_service_tasks("w", 10, &["charge".into()], 60)
        .await;
    assert_eq!(tasks.len(), 3);
    engine
        .handle_bpmn_error(tasks[0].id, "w", "DECLINED")
        .await
        .unwrap();

    let inst = details(&engine, id).await;
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.current_node, "error_end");
    assert!(engine.get_pending_service_tasks().is_empty());
}
