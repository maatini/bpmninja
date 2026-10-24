//! Conditional-Events: Intermediate Catch, Boundary und Persistenz.

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::ProcessDefinitionBuilder;

fn user_task_then_conditional_catch(name: &str, condition: &str) -> ProcessDefinition {
    ProcessDefinitionBuilder::new(name)
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("worker".into()))
        .node(
            "catch",
            BpmnElement::ConditionalCatchEvent {
                condition: condition.to_string(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "catch")
        .flow("catch", "end")
        .build()
        .unwrap()
}

fn conditional_catch_definition(name: &str, condition: &str) -> ProcessDefinition {
    ProcessDefinitionBuilder::new(name)
        .node("start", BpmnElement::StartEvent)
        .node(
            "catch",
            BpmnElement::ConditionalCatchEvent {
                condition: condition.to_string(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "catch")
        .flow("catch", "end")
        .build()
        .unwrap()
}

fn user_task_with_boundary_conditional(name: &str, cancel_activity: bool) -> ProcessDefinition {
    ProcessDefinitionBuilder::new(name)
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("worker".into()))
        .node(
            "bound_cond",
            BpmnElement::BoundaryConditionalEvent {
                attached_to: "task".into(),
                condition: "x > 10".into(),
                cancel_activity,
            },
        )
        .node("boundary_end", BpmnElement::EndEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .flow("bound_cond", "boundary_end")
        .build()
        .unwrap()
}

fn variable_updates(entries: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

fn caught_count(log: &[String]) -> usize {
    log.iter()
        .filter(|line| line.to_lowercase().contains("conditional") && line.contains("fired"))
        .count()
}

#[tokio::test]
async fn intermediate_catch_fires_once_after_user_task_variable() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(user_task_then_conditional_catch("cond_once", "x > 0"))
        .await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.pending_conditional_catches.len(), 0);
    let tasks = engine.get_pending_user_tasks();
    assert_eq!(tasks.len(), 1);

    engine
        .complete_user_task(tasks[0].task_id, variable_updates(&[("x", 1.into())]))
        .await
        .unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(engine.pending_conditional_catches.len(), 0);

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 1, "catch must fire exactly once");

    // Auf einer abgeschlossenen Instanz darf keine weitere Auswertung feuern.
    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 2.into())]))
        .await
        .unwrap();
    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 1);
}

#[tokio::test]
async fn catch_true_on_entry_does_not_wait() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(conditional_catch_definition("cond_entry", "x > 0"))
        .await;

    let inst_id = engine
        .start_instance_with_variables(def_key, variable_updates(&[("x", 5.into())]))
        .await
        .unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(engine.pending_conditional_catches.len(), 0);
    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 1);
}

#[tokio::test]
async fn update_instance_variables_fires_catch() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(conditional_catch_definition("cond_update", "x > 0"))
        .await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert!(matches!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::WaitingOnCondition { .. }
    ));
    assert_eq!(engine.pending_conditional_catches.len(), 1);

    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 1.into())]))
        .await
        .unwrap();

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(engine.pending_conditional_catches.len(), 0);
}

#[tokio::test]
async fn service_task_completion_fires_catch() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("cond_service")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "cond_topic".into(),
                multi_instance: None,
            },
        )
        .node(
            "catch",
            BpmnElement::ConditionalCatchEvent {
                condition: "x > 0".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "catch")
        .flow("catch", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    super::helpers::complete_all_service_tasks(
        &engine,
        "worker",
        variable_updates(&[("x", 3.into())]),
    )
    .await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert_eq!(engine.pending_conditional_catches.len(), 0);
}

#[tokio::test]
async fn non_interrupting_boundary_keeps_task_and_fires_once_per_transition() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(user_task_with_boundary_conditional("cond_nonint", false))
        .await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.pending_conditional_catches.len(), 1);

    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 20.into())]))
        .await
        .unwrap();

    assert_eq!(
        engine.get_pending_user_tasks().len(),
        1,
        "non-interrupting boundary must keep the user task"
    );
    assert_eq!(
        engine.pending_conditional_catches.len(),
        1,
        "non-interrupting catch stays registered"
    );
    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 1);
    assert!(
        log.iter().any(|line| line.contains("'boundary_end'")),
        "handler token must reach the boundary end event"
    );

    // Unverändert wahre Bedingung darf nicht erneut feuern.
    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 30.into())]))
        .await
        .unwrap();
    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 1);

    // false -> true feuert erneut.
    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 0.into())]))
        .await
        .unwrap();
    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 40.into())]))
        .await
        .unwrap();
    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 2);
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
}

#[tokio::test]
async fn interrupting_boundary_removes_pending_task() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(user_task_with_boundary_conditional("cond_int", true))
        .await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.get_pending_user_tasks().len(), 1);

    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 20.into())]))
        .await
        .unwrap();

    assert!(
        engine.get_pending_user_tasks().is_empty(),
        "interrupting boundary must remove the pending user task"
    );
    assert_eq!(engine.pending_conditional_catches.len(), 0);
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 1);
    assert!(log.iter().any(|line| line.contains("activity cancelled")));
    assert!(log.iter().any(|line| line.contains("'boundary_end'")));
}

#[tokio::test]
async fn task_completion_clears_boundary_conditional() {
    let pers = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(pers.clone());
    let (def_key, _) = engine
        .deploy_definition(user_task_with_boundary_conditional("cond_cleanup", false))
        .await;
    engine.start_instance(def_key).await.unwrap();

    assert_eq!(engine.pending_conditional_catches.len(), 1);
    assert_eq!(pers.list_conditional_catches().await.unwrap().len(), 1);

    let tasks = engine.get_pending_user_tasks();
    engine
        .complete_user_task(tasks[0].task_id, HashMap::new())
        .await
        .unwrap();

    assert_eq!(engine.pending_conditional_catches.len(), 0);
    assert!(
        pers.list_conditional_catches().await.unwrap().is_empty(),
        "boundary conditional must be removed from persistence on task completion"
    );
}

#[tokio::test]
async fn false_condition_stays_waiting_without_busy_loop() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(conditional_catch_definition("cond_false", "x > 0"))
        .await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    for value in [0, -1] {
        engine
            .update_instance_variables(inst_id, variable_updates(&[("x", value.into())]))
            .await
            .unwrap();
        assert!(matches!(
            engine.get_instance_state(inst_id).await.unwrap(),
            InstanceState::WaitingOnCondition { .. }
        ));
    }
    assert_eq!(engine.pending_conditional_catches.len(), 1);
    let log = engine.get_audit_log(inst_id).await.unwrap();
    assert_eq!(caught_count(&log), 0);

    engine
        .update_instance_variables(inst_id, variable_updates(&[("x", 7.into())]))
        .await
        .unwrap();
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn only_satisfied_catch_fires() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("cond_two")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node(
            "catch_a",
            BpmnElement::ConditionalCatchEvent {
                condition: "a > 0".into(),
            },
        )
        .node(
            "catch_b",
            BpmnElement::ConditionalCatchEvent {
                condition: "b > 0".into(),
            },
        )
        .node("end_a", BpmnElement::EndEvent)
        .node("end_b", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("fork", "catch_a")
        .flow("fork", "catch_b")
        .flow("catch_a", "end_a")
        .flow("catch_b", "end_b")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();
    assert_eq!(engine.pending_conditional_catches.len(), 2);

    engine
        .update_instance_variables(inst_id, variable_updates(&[("a", 1.into())]))
        .await
        .unwrap();

    assert_eq!(engine.pending_conditional_catches.len(), 1);
    let pending = engine.get_pending_conditional_catches();
    assert_eq!(pending[0].node_id, "catch_b");
    assert!(matches!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::ParallelExecution { .. }
    ));

    engine
        .update_instance_variables(inst_id, variable_updates(&[("b", 1.into())]))
        .await
        .unwrap();

    assert_eq!(engine.pending_conditional_catches.len(), 0);
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn conditional_catch_persists_and_restores() {
    let pers = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(pers.clone());

    let def = conditional_catch_definition("cond_restore", "x > 0");
    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    let persisted = pers.list_conditional_catches().await.unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].instance_id, inst_id);
    assert_eq!(persisted[0].node_id, "catch");
    assert!(!persisted[0].last_result);

    let engine2 = WorkflowEngine::new().with_persistence(pers.clone());
    let restored_def = ProcessDefinitionBuilder::new("cond_restore")
        .with_key(def_key)
        .node("start", BpmnElement::StartEvent)
        .node(
            "catch",
            BpmnElement::ConditionalCatchEvent {
                condition: "x > 0".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "catch")
        .flow("catch", "end")
        .build()
        .unwrap();
    engine2.deploy_definition(restored_def).await;
    for instance in pers.list_instances().await.unwrap() {
        engine2.restore_instance(instance).await;
    }
    for catch in pers.list_conditional_catches().await.unwrap() {
        engine2.restore_conditional_catch(catch);
    }

    assert_eq!(engine2.pending_conditional_catches.len(), 1);

    engine2
        .update_instance_variables(inst_id, variable_updates(&[("x", 1.into())]))
        .await
        .unwrap();

    assert_eq!(
        engine2.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert!(pers.list_conditional_catches().await.unwrap().is_empty());
}
