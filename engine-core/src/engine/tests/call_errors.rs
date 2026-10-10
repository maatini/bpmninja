//! Split from `unit_tests.rs` (M5).

use super::super::*;
use super::helpers::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn boundary_error_event_catches_error() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("bound_err")
        .node("start", BpmnElement::StartEvent)
        .node(
            "task",
            BpmnElement::ServiceTask {
                topic: "err_topic".into(),
                multi_instance: None,
            },
        )
        .node(
            "bound_err",
            BpmnElement::BoundaryErrorEvent {
                attached_to: "task".into(),
                error_code: Some("ERR_CODE_500".into()),
            },
        )
        .node("end1", BpmnElement::EndEvent)
        .node("end2", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end1")
        .flow("bound_err", "end2")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    let tasks = engine
        .fetch_and_lock_service_tasks("worker", 1, &["err_topic".into()], 10)
        .await;
    assert_eq!(tasks.len(), 1);

    engine
        .handle_bpmn_error(tasks[0].id, "worker", "ERR_CODE_500")
        .await
        .unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.current_node, "end2");
}

#[tokio::test]
async fn call_activity_lifecycle() {
    let engine = WorkflowEngine::new();

    // Deploy Child
    let child_def = ProcessDefinitionBuilder::new("child_proc")
        .node("start", BpmnElement::StartEvent)
        .node("child_task", BpmnElement::UserTask("child_assignee".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "child_task")
        .flow("child_task", "end")
        .build()
        .unwrap();
    let (_child_key, _) = engine.deploy_definition(child_def).await;

    // Deploy Parent
    let parent_def = ProcessDefinitionBuilder::new("parent_proc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "call",
            BpmnElement::CallActivity {
                called_element: "child_proc".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "call")
        .flow("call", "end")
        .build()
        .unwrap();
    let (parent_key, _) = engine.deploy_definition(parent_def).await;

    // Start Parent
    let parent_id = engine.start_instance(parent_key).await.unwrap();

    // Parent should be blocked on Call Activity
    let parent_inst = engine.get_instance_details(parent_id).await.unwrap();
    if let InstanceState::WaitingOnCallActivity {
        sub_instance_id, ..
    } = parent_inst.state
    {
        // Child instance should exist
        let child_inst = engine.get_instance_details(sub_instance_id).await.unwrap();
        assert_eq!(child_inst.parent_instance_id, Some(parent_id));
        assert!(matches!(
            child_inst.state,
            InstanceState::WaitingOnUserTask { .. }
        ));
        assert!(matches!(
            child_inst.state,
            InstanceState::WaitingOnUserTask { .. }
        ));

        // Complete the child's user task
        let tasks = engine.get_pending_user_tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].instance_id, sub_instance_id);

        let child_task_id = tasks[0].task_id;

        // Add a variable to child to ensure parent gets it
        let mut vars = std::collections::HashMap::new();
        vars.insert("from_child".into(), serde_json::json!("hello parent"));
        engine
            .complete_user_task(child_task_id, vars)
            .await
            .unwrap();

        // Child should be completed
        let child_inst = engine.get_instance_details(sub_instance_id).await.unwrap();
        assert_eq!(child_inst.state, InstanceState::Completed);

        // Parent should now be automatically resumed and completed
        let parent_inst = engine.get_instance_details(parent_id).await.unwrap();
        assert_eq!(parent_inst.state, InstanceState::Completed);
        assert_eq!(
            parent_inst
                .variables
                .get("from_child")
                .unwrap()
                .as_str()
                .unwrap(),
            "hello parent"
        );
    } else {
        panic!(
            "Parent not waiting on call activity: {:?}",
            parent_inst.state
        );
    }
}

// -----------------------------------------------------------------------
// Validation / ErrorEndEvent / Call Activity Error Propagation Tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn top_level_error_end_event_results_in_completed_with_error() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("top_err")
        .node("start", BpmnElement::StartEvent)
        .node(
            "err_end",
            BpmnElement::ErrorEndEvent {
                error_code: String::from("CRITICAL_FAIL"),
            },
        )
        .flow("start", "err_end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let instance_id = engine.start_instance(key).await.unwrap();

    let state = engine.get_instance_state(instance_id).await.unwrap();
    assert_eq!(
        state,
        InstanceState::CompletedWithError {
            error_code: "CRITICAL_FAIL".into()
        }
    );

    let log = engine.get_audit_log(instance_id).await.unwrap();
    assert!(
        log.iter()
            .any(|l| l.contains("CRITICAL_FAIL") && l.contains("Error End"))
    );
}

#[tokio::test]
async fn call_activity_propagates_error_to_matching_boundary_event() {
    let engine = WorkflowEngine::new();
    let (_, _) = engine
        .deploy_definition(build_child_error_process("ERR_CHILD"))
        .await;

    let parent_def = ProcessDefinitionBuilder::new("parent_proc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "call",
            BpmnElement::CallActivity {
                called_element: "child_proc".into(),
            },
        )
        .node(
            "bound_err",
            BpmnElement::BoundaryErrorEvent {
                attached_to: "call".into(),
                error_code: Some("ERR_CHILD".into()),
            },
        )
        .node("end_normal", BpmnElement::EndEvent)
        .node("end_error", BpmnElement::EndEvent)
        .flow("start", "call")
        .flow("call", "end_normal")
        .flow("bound_err", "end_error")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(parent_def).await;
    let instance_id = engine.start_instance(key).await.unwrap();

    let state = engine.get_instance_state(instance_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);

    let log = engine.get_audit_log(instance_id).await.unwrap();
    // Verify it took the error path
    assert!(log.iter().any(|l| l.contains("'end_error'")));
}

#[tokio::test]
async fn call_activity_propagates_error_to_wildcard_boundary_event() {
    let engine = WorkflowEngine::new();
    let (_, _) = engine
        .deploy_definition(build_child_error_process("ANY_ERR_CODE"))
        .await;

    let parent_def = ProcessDefinitionBuilder::new("parent_wildcard")
        .node("start", BpmnElement::StartEvent)
        .node(
            "call",
            BpmnElement::CallActivity {
                called_element: "child_proc".into(),
            },
        )
        .node(
            "bound_err",
            BpmnElement::BoundaryErrorEvent {
                attached_to: "call".into(),
                error_code: None,
            },
        ) // Wildcard
        .node("end_normal", BpmnElement::EndEvent)
        .node("end_error", BpmnElement::EndEvent)
        .flow("start", "call")
        .flow("call", "end_normal")
        .flow("bound_err", "end_error")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(parent_def).await;
    let instance_id = engine.start_instance(key).await.unwrap();

    let state = engine.get_instance_state(instance_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);

    let log = engine.get_audit_log(instance_id).await.unwrap();
    assert!(log.iter().any(|l| l.contains("'end_error'")));
}

#[tokio::test]
async fn call_activity_unhandled_error_becomes_incident() {
    let engine = WorkflowEngine::new();
    let (_, _) = engine
        .deploy_definition(build_child_error_process("UNHANDLED_CODE"))
        .await;

    let parent_def = ProcessDefinitionBuilder::new("parent_unhandled")
        .node("start", BpmnElement::StartEvent)
        .node(
            "call",
            BpmnElement::CallActivity {
                called_element: "child_proc".into(),
            },
        )
        .node("end_normal", BpmnElement::EndEvent)
        .flow("start", "call")
        .flow("call", "end_normal")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(parent_def).await;
    let instance_id = engine.start_instance(key).await.unwrap();

    // Check state is waiting on call activity because it's an incident
    let state = engine.get_instance_state(instance_id).await.unwrap();
    assert!(matches!(state, InstanceState::WaitingOnCallActivity { .. }));

    let log = engine.get_audit_log(instance_id).await.unwrap();
    assert!(
        log.iter()
            .any(|l| l.contains("INCIDENT") && l.contains("UNHANDLED_CODE"))
    );
}

// ============================================================================
// Escalation Event Tests
// ============================================================================

#[tokio::test]
async fn test_escalation_end_event_completes_instance() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("esc_end")
        .node("start", BpmnElement::StartEvent)
        .node(
            "esc_end",
            BpmnElement::EscalationEndEvent {
                escalation_code: "ESC_001".into(),
            },
        )
        .flow("start", "esc_end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    // EscalationEnd at top level completes the instance (non-fatal)
    assert!(matches!(inst.state, InstanceState::Completed));
    assert!(inst.audit_log.iter().any(|l| l.contains("Escalation")));
}

#[tokio::test]
async fn test_escalation_throw_no_handler_continues() {
    // Intermediate escalation throw with no handler → token continues normally
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("esc_throw_no_handler")
        .node("start", BpmnElement::StartEvent)
        .node(
            "esc_throw",
            BpmnElement::EscalationThrowEvent {
                escalation_code: "ESC_002".into(),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "esc_throw")
        .flow("esc_throw", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    assert!(
        inst.audit_log
            .iter()
            .any(|l| l.contains("no handler found"))
    );
}

#[tokio::test]
async fn test_escalation_throw_with_interrupting_boundary() {
    // Escalation throw inside subprocess, caught by interrupting boundary on task
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("esc_interrupt")
        .node("start", BpmnElement::StartEvent)
        .node(
            "task1",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node(
            "esc_throw",
            BpmnElement::EscalationThrowEvent {
                escalation_code: "REVIEW_NEEDED".into(),
            },
        )
        .node(
            "boundary_esc",
            BpmnElement::BoundaryEscalationEvent {
                attached_to: "task1".into(),
                escalation_code: Some("REVIEW_NEEDED".into()),
                cancel_activity: true,
            },
        )
        .node("esc_handler_end", BpmnElement::EndEvent)
        .node("normal_end", BpmnElement::EndEvent)
        .flow("start", "esc_throw")
        .flow("task1", "normal_end")
        .flow("esc_throw", "normal_end")
        .flow("boundary_esc", "esc_handler_end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    assert!(inst.audit_log.iter().any(|l| l.contains("interrupting")));
}

#[tokio::test]
async fn test_escalation_throw_with_non_interrupting_boundary() {
    // Non-interrupting boundary → spawns handler token, main token continues
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("esc_non_interrupt")
        .node("start", BpmnElement::StartEvent)
        .node(
            "task1",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node(
            "esc_throw",
            BpmnElement::EscalationThrowEvent {
                escalation_code: "INFO".into(),
            },
        )
        .node(
            "boundary_esc",
            BpmnElement::BoundaryEscalationEvent {
                attached_to: "task1".into(),
                escalation_code: Some("INFO".into()),
                cancel_activity: false,
            },
        )
        .node("handler_end", BpmnElement::EndEvent)
        .node("normal_end", BpmnElement::EndEvent)
        .flow("start", "esc_throw")
        .flow("task1", "normal_end")
        .flow("esc_throw", "normal_end")
        .flow("boundary_esc", "handler_end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    assert!(
        inst.audit_log
            .iter()
            .any(|l| l.contains("non-interrupting"))
    );
}

#[tokio::test]
async fn test_escalation_wildcard_boundary_catches_any() {
    // Boundary with escalation_code: None catches any escalation code
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("esc_wildcard")
        .node("start", BpmnElement::StartEvent)
        .node(
            "task1",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node(
            "esc_throw",
            BpmnElement::EscalationThrowEvent {
                escalation_code: "ANY_CODE".into(),
            },
        )
        .node(
            "boundary_esc",
            BpmnElement::BoundaryEscalationEvent {
                attached_to: "task1".into(),
                escalation_code: None, // wildcard
                cancel_activity: true,
            },
        )
        .node("handler_end", BpmnElement::EndEvent)
        .node("normal_end", BpmnElement::EndEvent)
        .flow("start", "esc_throw")
        .flow("task1", "normal_end")
        .flow("esc_throw", "normal_end")
        .flow("boundary_esc", "handler_end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    assert!(
        inst.audit_log
            .iter()
            .any(|l| l.contains("caught") && l.contains("interrupting"))
    );
}

// ============================================================================
// Compensation Event Tests
// ============================================================================

#[tokio::test]
async fn test_compensation_registers_and_executes_handler() {
    // Script task with compensation boundary → compensation throw undoes it
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("comp_basic")
        .node("start", BpmnElement::StartEvent)
        .node(
            "script1",
            BpmnElement::ScriptTask {
                script: r#"let step1 = "done";"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "boundary_comp",
            BpmnElement::BoundaryCompensationEvent {
                attached_to: "script1".into(),
            },
        )
        .node(
            "comp_handler",
            BpmnElement::ScriptTask {
                script: r#"step1 = "undone";"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "comp_throw",
            BpmnElement::CompensationThrowEvent { activity_ref: None },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "script1")
        .flow("script1", "comp_throw")
        .flow("comp_throw", "end")
        .flow("boundary_comp", "comp_handler")
        .flow("comp_handler", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    // Compensation handler should have been registered and executed
    assert!(
        inst.audit_log
            .iter()
            .any(|l| l.contains("Registered compensation"))
    );
    assert!(
        inst.audit_log
            .iter()
            .any(|l| l.contains("Compensation triggered"))
    );
    // After compensation, step1 should be "undone"
    assert_eq!(
        inst.variables.get("step1").and_then(|v| v.as_str()),
        Some("undone")
    );
}

#[tokio::test]
async fn test_compensation_end_event() {
    // CompensationEndEvent triggers compensation and then completes
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("comp_end")
        .node("start", BpmnElement::StartEvent)
        .node(
            "script1",
            BpmnElement::ScriptTask {
                script: r#"let x = 42;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "boundary_comp",
            BpmnElement::BoundaryCompensationEvent {
                attached_to: "script1".into(),
            },
        )
        .node(
            "comp_handler",
            BpmnElement::ScriptTask {
                script: r#"x = 0;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "comp_end",
            BpmnElement::CompensationEndEvent { activity_ref: None },
        )
        .node("handler_end", BpmnElement::EndEvent)
        .flow("start", "script1")
        .flow("script1", "comp_end")
        .flow("boundary_comp", "comp_handler")
        .flow("comp_handler", "handler_end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    assert_eq!(inst.variables.get("x").and_then(|v| v.as_i64()), Some(0));
}

#[tokio::test]
async fn test_compensation_specific_activity() {
    // CompensationThrowEvent with activity_ref targets only one activity's handler
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("comp_specific")
        .node("start", BpmnElement::StartEvent)
        .node(
            "script1",
            BpmnElement::ScriptTask {
                script: r#"let a = 1;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "boundary_comp1",
            BpmnElement::BoundaryCompensationEvent {
                attached_to: "script1".into(),
            },
        )
        .node(
            "comp_handler1",
            BpmnElement::ScriptTask {
                script: r#"a = -1;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "script2",
            BpmnElement::ScriptTask {
                script: r#"let b = 2;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "boundary_comp2",
            BpmnElement::BoundaryCompensationEvent {
                attached_to: "script2".into(),
            },
        )
        .node(
            "comp_handler2",
            BpmnElement::ScriptTask {
                script: r#"b = -2;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "comp_throw",
            BpmnElement::CompensationThrowEvent {
                activity_ref: Some("script1".into()),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .node("handler_end1", BpmnElement::EndEvent)
        .node("handler_end2", BpmnElement::EndEvent)
        .flow("start", "script1")
        .flow("script1", "script2")
        .flow("script2", "comp_throw")
        .flow("comp_throw", "end")
        .flow("boundary_comp1", "comp_handler1")
        .flow("comp_handler1", "handler_end1")
        .flow("boundary_comp2", "comp_handler2")
        .flow("comp_handler2", "handler_end2")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    // Only script1's handler ran, so a=-1, but b stays at 2
    assert_eq!(inst.variables.get("a").and_then(|v| v.as_i64()), Some(-1));
    assert_eq!(inst.variables.get("b").and_then(|v| v.as_i64()), Some(2));
}

#[tokio::test]
async fn test_compensation_no_handlers_still_completes() {
    // CompensationThrowEvent with no registered handlers → just continues
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("comp_empty")
        .node("start", BpmnElement::StartEvent)
        .node(
            "comp_throw",
            BpmnElement::CompensationThrowEvent { activity_ref: None },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "comp_throw")
        .flow("comp_throw", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    assert!(
        inst.audit_log
            .iter()
            .any(|l| l.contains("0 handler(s) to execute"))
    );
}

#[tokio::test]
async fn test_compensation_specific_activity_only_targets_one() {
    // Catches: replace != with == in handle_compensation_throw_event (events.rs:412)
    // This is the same test as test_compensation_specific_activity but we
    // explicitly verify that ONLY script1's handler runs, not script2's
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("comp_filter")
        .node("start", BpmnElement::StartEvent)
        .node(
            "script1",
            BpmnElement::ScriptTask {
                script: r#"let a = 10;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "boundary_comp1",
            BpmnElement::BoundaryCompensationEvent {
                attached_to: "script1".into(),
            },
        )
        .node(
            "comp_handler1",
            BpmnElement::ScriptTask {
                script: r#"a = 0;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "script2",
            BpmnElement::ScriptTask {
                script: r#"let b = 20;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "boundary_comp2",
            BpmnElement::BoundaryCompensationEvent {
                attached_to: "script2".into(),
            },
        )
        .node(
            "comp_handler2",
            BpmnElement::ScriptTask {
                script: r#"b = 0;"#.into(),
                multi_instance: None,
            },
        )
        .node(
            "comp_throw",
            BpmnElement::CompensationThrowEvent {
                activity_ref: Some("script2".into()),
            },
        )
        .node("end", BpmnElement::EndEvent)
        .node("handler_end1", BpmnElement::EndEvent)
        .node("handler_end2", BpmnElement::EndEvent)
        .flow("start", "script1")
        .flow("script1", "script2")
        .flow("script2", "comp_throw")
        .flow("comp_throw", "end")
        .flow("boundary_comp1", "comp_handler1")
        .flow("comp_handler1", "handler_end1")
        .flow("boundary_comp2", "comp_handler2")
        .flow("comp_handler2", "handler_end2")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    // Only script2's handler ran (b → 0), script1's handler did NOT run (a stays 10)
    assert_eq!(inst.variables.get("a").and_then(|v| v.as_i64()), Some(10));
    assert_eq!(inst.variables.get("b").and_then(|v| v.as_i64()), Some(0));
}

/// Catches: Terminate End Event retain predicates (== vs !=)
#[tokio::test]
async fn test_terminate_end_kills_all_pending() {
    let engine = WorkflowEngine::with_in_memory_persistence();

    // Parallel split: one branch goes to user task then end, other to terminate
    let def = ProcessDefinitionBuilder::new("term_kill")
        .node("start", BpmnElement::StartEvent)
        .node("split", BpmnElement::ParallelGateway)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .node("term", BpmnElement::TerminateEndEvent)
        .flow("start", "split")
        .flow("split", "ut")
        .flow("ut", "end")
        .flow("split", "term")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // After terminate, no pending user tasks should remain for this instance
    let remaining = engine
        .pending_user_tasks
        .iter()
        .filter(|r| r.instance_id == inst_id)
        .count();
    assert_eq!(remaining, 0, "Terminate should kill all pending user tasks");

    // Instance should be completed
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(
        matches!(inst.state, InstanceState::Completed),
        "Terminate should set state to Completed, got: {:?}",
        inst.state
    );
}
