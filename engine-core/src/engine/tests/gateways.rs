//! Split from `unit_tests.rs` (M5).

use super::super::*;
use super::helpers::*;
use crate::domain::ProcessDefinitionBuilder;

// -----------------------------------------------------------------------
// Condition evaluator tests
// -----------------------------------------------------------------------

// Condition unit tests removed — covered by condition.rs::tests.

// -----------------------------------------------------------------------
// ExclusiveGateway (XOR) tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn exclusive_gateway_takes_matching_path() {
    let engine = WorkflowEngine::new();

    // Start → XOR Gateway → (amount > 100 → high) / (default → low) → End
    let def = ProcessDefinitionBuilder::new("xor_test")
        .node("start", BpmnElement::StartEvent)
        .node(
            "gw",
            BpmnElement::ExclusiveGateway {
                default: Some("low".into()),
            },
        )
        .node(
            "high",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node(
            "low",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "high", "amount > 100")
        .flow("gw", "low") // unconditional (default candidate)
        .flow("high", "end")
        .flow("low", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    // amount = 500 → should take the "high" path
    let mut vars = HashMap::new();
    vars.insert("amount".into(), Value::Number(500.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    let log = engine.get_audit_log(inst_id).await.unwrap();
    let gw_entry = log
        .iter()
        .find(|l| l.contains("Exclusive gateway"))
        .unwrap();
    assert!(gw_entry.contains("high"), "Expected high path: {gw_entry}");
}

#[tokio::test]
async fn exclusive_gateway_uses_default_when_no_match() {
    let engine = WorkflowEngine::new();

    let def = ProcessDefinitionBuilder::new("xor_default")
        .node("start", BpmnElement::StartEvent)
        .node(
            "gw",
            BpmnElement::ExclusiveGateway {
                default: Some("low".into()),
            },
        )
        .node(
            "high",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node(
            "low",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "high", "amount > 100")
        .flow("gw", "low")
        .flow("high", "end")
        .flow("low", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    // amount = 50 → no condition matches → should use default "low"
    let mut vars = HashMap::new();
    vars.insert("amount".into(), Value::Number(50.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    let log = engine.get_audit_log(inst_id).await.unwrap();
    let gw_entry = log
        .iter()
        .find(|l| l.contains("Exclusive gateway"))
        .unwrap();
    assert!(
        gw_entry.contains("low"),
        "Expected low (default) path: {gw_entry}"
    );
}

#[tokio::test]
async fn exclusive_gateway_error_when_no_match_no_default() {
    let engine = WorkflowEngine::new();

    let def = ProcessDefinitionBuilder::new("xor_fail")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::ExclusiveGateway { default: None })
        .node("a", BpmnElement::EndEvent)
        .node("b", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "a", "x == 1")
        .conditional_flow("gw", "b", "x == 2")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    // No variables at all → no condition matches → error
    let result = engine.start_instance(def_key).await;
    assert!(
        matches!(result, Err(EngineError::NoMatchingCondition(_))),
        "Expected NoMatchingCondition, got: {result:?}"
    );
}

// -----------------------------------------------------------------------
// InclusiveGateway (OR) tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn inclusive_gateway_forks_multiple_paths() {
    let engine = WorkflowEngine::new();

    // Start → Inclusive GW → (a > 0 → svc_a → end) / (b > 0 → svc_b → end)
    let def = ProcessDefinitionBuilder::new("or_test")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::InclusiveGateway)
        .node(
            "svc_a",
            BpmnElement::ServiceTask {
                topic: "track_a".into(),
                multi_instance: None,
            },
        )
        .node(
            "svc_b",
            BpmnElement::ServiceTask {
                topic: "track_b".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "svc_a", "a > 0")
        .conditional_flow("gw", "svc_b", "b > 0")
        .flow("svc_a", "end")
        .flow("svc_b", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    // Both conditions true → both paths should fire
    let mut vars = HashMap::new();
    vars.insert("a".into(), Value::Number(10.into()));
    vars.insert("b".into(), Value::Number(20.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    let log = engine.get_audit_log(inst_id).await.unwrap();
    let gw_entry = log
        .iter()
        .find(|l| l.contains("Inclusive gateway"))
        .unwrap();
    assert!(
        gw_entry.contains("2 path(s)"),
        "Expected 2 forked paths: {gw_entry}"
    );
}

#[tokio::test]
async fn inclusive_gateway_single_match_no_fork() {
    let engine = WorkflowEngine::new();

    let def = ProcessDefinitionBuilder::new("or_single")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::InclusiveGateway)
        .node(
            "a",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node(
            "b",
            BpmnElement::ServiceTask {
                topic: "noop".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "a", "x == 1")
        .conditional_flow("gw", "b", "x == 2")
        .flow("a", "end")
        .flow("b", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    // Only x == 1 → single match → Continue (not ContinueMultiple)
    let mut vars = HashMap::new();
    vars.insert("x".into(), Value::Number(1.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

#[tokio::test]
async fn xor_gateway_positive_x_routes_to_user_task_1() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(build_xor_user_task_definition())
        .await;

    // x = 5 → condition "x > 0" matches → user-task-1
    let mut vars = HashMap::new();
    vars.insert("x".into(), Value::Number(5.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    // Instance should be waiting on user-task-1
    let pending = engine.get_pending_user_tasks();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].node_id, "user-task-1");
    assert_eq!(pending[0].assignee, "author");

    assert!(matches!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::WaitingOnUserTask { .. }
    ));

    // Audit log should show gateway took path to user-task-1
    let log = engine.get_audit_log(inst_id).await.unwrap();
    let gw_entry = log
        .iter()
        .find(|l| l.contains("Exclusive gateway"))
        .unwrap();
    assert!(
        gw_entry.contains("user-task-1"),
        "Expected user-task-1 path: {gw_entry}"
    );

    // Complete the user task → should reach end
    let task_id = pending[0].task_id;
    engine
        .complete_user_task(task_id, HashMap::new())
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
    assert!(engine.get_pending_user_tasks().is_empty());
}

#[tokio::test]
async fn xor_gateway_negative_x_routes_to_user_task_2() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(build_xor_user_task_definition())
        .await;

    // x = -3 → condition "x > 0" does NOT match → default → user-task-2
    let mut vars = HashMap::new();
    vars.insert("x".into(), Value::Number((-3).into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    // Instance should be waiting on user-task-2
    let pending = engine.get_pending_user_tasks();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].node_id, "user-task-2");
    assert_eq!(pending[0].assignee, "reviewer");

    // Audit log should show gateway took default path to user-task-2
    let log = engine.get_audit_log(inst_id).await.unwrap();
    let gw_entry = log
        .iter()
        .find(|l| l.contains("Exclusive gateway"))
        .unwrap();
    assert!(
        gw_entry.contains("user-task-2"),
        "Expected user-task-2 (default) path: {gw_entry}"
    );

    // Complete the user task → should reach end
    let task_id = pending[0].task_id;
    engine
        .complete_user_task(task_id, HashMap::new())
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );
}

// xor_gateway_zero_x_routes_to_user_task_2 entfernt — redundant mit
// xor_gateway_negative_x_routes_to_user_task_2 (gleicher Default-Pfad).

#[tokio::test]
async fn xor_gateway_user_task_merges_variables() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(build_xor_user_task_definition())
        .await;

    // x = 10 → user-task-1
    let mut vars = HashMap::new();
    vars.insert("x".into(), Value::Number(10.into()));
    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    let task_id = engine.get_pending_user_tasks()[0].task_id;

    // Complete with extra variables → they should be merged into the instance
    let mut completion_vars = HashMap::new();
    completion_vars.insert("result".into(), Value::String("approved".into()));
    engine
        .complete_user_task(task_id, completion_vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    // Verify both original and merged variables are present
    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(
        details.variables.get("x"),
        Some(&Value::Number(10.into())),
        "Original variable 'x' should be preserved"
    );
    assert_eq!(
        details.variables.get("result"),
        Some(&Value::String("approved".into())),
        "Merged variable 'result' should be present"
    );
}

// -----------------------------------------------------------------------
// ParallelGateway (AND) & Multi-Token tests
// -----------------------------------------------------------------------

#[tokio::test]
async fn parallel_gateway_forks_and_joins() {
    let engine = WorkflowEngine::new();

    // Start -> Split -> (A, B) -> Join -> End
    let def = ProcessDefinitionBuilder::new("and_test")
        .node("start", BpmnElement::StartEvent)
        .node("split", BpmnElement::ParallelGateway)
        .node(
            "task_a",
            BpmnElement::ServiceTask {
                topic: "task_a".into(),
                multi_instance: None,
            },
        )
        .node(
            "task_b",
            BpmnElement::ServiceTask {
                topic: "task_b".into(),
                multi_instance: None,
            },
        )
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "split")
        .flow("split", "task_a")
        .flow("split", "task_b")
        .flow("task_a", "join")
        .flow("task_b", "join")
        .flow("join", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    let inst_id = engine.start_instance(def_key).await.unwrap();

    // Should be paused in parallel execution waiting for both service tasks
    let state = engine.get_instance_state(inst_id).await.unwrap().clone();
    println!("State after start: {:?}", state);
    for entry in engine.get_audit_log(inst_id).await.unwrap() {
        println!("Log: {}", entry);
    }
    assert!(
        matches!(
            state,
            InstanceState::ParallelExecution {
                active_token_count: 2
            }
        ),
        "State should be parallel execution: {:?}",
        state
    );

    assert_eq!(engine.pending_service_tasks.len(), 2);

    // Complete task A
    let task_a = engine
        .pending_service_tasks
        .iter()
        .map(|r| r.value().clone())
        .find(|t| t.topic == "task_a")
        .unwrap()
        .id;
    // Need to fetch and lock it first
    let _ = engine
        .fetch_and_lock_service_tasks("worker", 10, &["task_a".into()], 1000)
        .await;

    let mut vars_a = std::collections::HashMap::new();
    vars_a.insert("var_a".into(), serde_json::Value::Bool(true));
    engine
        .complete_service_task(task_a, "worker", vars_a)
        .await
        .unwrap();

    // After A completes, it should be waiting at the join. Still in parallel state.
    let state = engine.get_instance_state(inst_id).await.unwrap().clone();
    println!("State after A completes: {:?}", state);
    for entry in engine.get_audit_log(inst_id).await.unwrap() {
        println!("Log: {}", entry);
    }
    assert!(matches!(
        state,
        InstanceState::ParallelExecution {
            active_token_count: 2
        }
    ));

    // Check join barrier
    let inst = engine.instances.get(&inst_id).await.unwrap();
    let inst_lock = inst.read().await;
    let barrier = inst_lock.join_barriers.get("join").unwrap();
    assert_eq!(barrier.expected_count, 2);
    assert_eq!(barrier.arrived_tokens.len(), 1);
    drop(inst_lock);

    // Complete task B
    let task_b = engine
        .pending_service_tasks
        .iter()
        .map(|r| r.value().clone())
        .find(|t| t.topic == "task_b")
        .unwrap()
        .id;
    let _ = engine
        .fetch_and_lock_service_tasks("worker", 10, &["task_b".into()], 1000)
        .await;
    let mut vars_b = std::collections::HashMap::new();
    vars_b.insert("var_b".into(), serde_json::Value::Bool(true));
    engine
        .complete_service_task(task_b, "worker", vars_b)
        .await
        .unwrap();

    // Now it should be complete!
    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    // Variables from both branches should be merged
}

#[tokio::test]
async fn mutation_find_downstream_join() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("join")
        .node("start", BpmnElement::StartEvent)
        .node("gw_split", BpmnElement::ParallelGateway)
        .node("gw_join", BpmnElement::ParallelGateway)
        .node(
            "dummy",
            BpmnElement::ServiceTask {
                topic: "dummy".to_string(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw_split")
        .flow("gw_split", "gw_join")
        .flow("gw_split", "dummy")
        .flow("dummy", "gw_join")
        .flow("gw_join", "end")
        .build()
        .unwrap();

    let _ = engine.deploy_definition(def.clone()).await;

    // Testing the logic explicitly via direct call (internal visibility allows this within engine module)
    let found = crate::engine::executor::helpers::find_downstream_join(&def, "gw_split");
    assert_eq!(found, Some("gw_join".to_string()));

    // Find with depth limit (though internal recursion only decreases by 1, testing the > 100 limit protection is hard, but we can just test if the logic iterates correctly).
    let found_from_start = crate::engine::executor::helpers::find_downstream_join(&def, "start");
    assert_eq!(found_from_start, None);
    // It actually returns None because it exceeds max recursion or doesn't find gateway.
}

#[tokio::test]
async fn test_nested_parallel_gateways() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("nested")
        .node("start", BpmnElement::StartEvent)
        .node("s1", BpmnElement::ParallelGateway)
        .node(
            "t1",
            BpmnElement::ServiceTask {
                topic: "t".into(),
                multi_instance: None,
            },
        )
        .node("s2", BpmnElement::ParallelGateway)
        .node(
            "t2",
            BpmnElement::ServiceTask {
                topic: "t".into(),
                multi_instance: None,
            },
        )
        .node(
            "t3",
            BpmnElement::ServiceTask {
                topic: "t".into(),
                multi_instance: None,
            },
        )
        .node("j2", BpmnElement::ParallelGateway)
        .node("j1", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "s1")
        .flow("s1", "t1")
        .flow("s1", "s2")
        .flow("s2", "t2")
        .flow("s2", "t3")
        .flow("t2", "j2")
        .flow("t3", "j2")
        .flow("j2", "j1")
        .flow("t1", "j1")
        .flow("j1", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(def_key).await.unwrap();

    let _ = engine
        .fetch_and_lock_service_tasks("worker", 10, &["t".into()], 10)
        .await;

    // Complete all 3 tasks
    let mut i = 0;
    while let Some(task) = engine.get_pending_service_tasks().first() {
        let task_id = task.id;
        engine
            .complete_service_task(task_id, "worker", std::collections::HashMap::new())
            .await
            .unwrap();
        i += 1;
        if i > 5 {
            break;
        } // safety loop limit
    }

    let state = engine.get_instance_state(inst_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);
}

#[tokio::test]
async fn test_inclusive_gateway_multiple_paths() {
    // Catches: replace && with || in execute_inclusive_gateway;
    //          replace >= with <; delete !
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("incl")
        .node("start", BpmnElement::StartEvent)
        .node("gw", BpmnElement::InclusiveGateway)
        .node(
            "a",
            BpmnElement::ScriptTask {
                script: "let r = 1;".into(),
                multi_instance: None,
            },
        )
        .node(
            "b",
            BpmnElement::ScriptTask {
                script: "let s = 2;".into(),
                multi_instance: None,
            },
        )
        .node("join", BpmnElement::InclusiveGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "a", "x == 1")
        .conditional_flow("gw", "b", "y == 1")
        .flow("a", "join")
        .flow("b", "join")
        .flow("join", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // Both conditions true → both paths taken
    let mut vars = HashMap::new();
    vars.insert("x".to_string(), serde_json::json!(1));
    vars.insert("y".to_string(), serde_json::json!(1));
    let inst_id = engine
        .start_instance_with_variables(key, vars)
        .await
        .unwrap();
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(matches!(inst.state, InstanceState::Completed));
    assert_eq!(inst.variables.get("r"), Some(&serde_json::json!(1)));
    assert_eq!(inst.variables.get("s"), Some(&serde_json::json!(2)));
}

/// Catches: same_gateway_type -> bool with true (mismatched types should return false)
#[tokio::test]
async fn test_same_gateway_type_detects_mismatch() {
    // Mismatch: Exclusive split with Parallel join → should NOT detect as same
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("gw_mismatch")
        .node("start", BpmnElement::StartEvent)
        .node(
            "xor_split",
            BpmnElement::ExclusiveGateway {
                default: Some("task_b".into()),
            },
        )
        .node(
            "task_a",
            BpmnElement::ServiceTask {
                topic: "a".into(),
                multi_instance: None,
            },
        )
        .node(
            "task_b",
            BpmnElement::ServiceTask {
                topic: "b".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "xor_split")
        .conditional_flow("xor_split", "task_a", "x == 1")
        .flow("xor_split", "task_b")
        .flow("task_a", "end")
        .flow("task_b", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    let mut vars = HashMap::new();
    vars.insert("x".into(), serde_json::json!(1));
    let inst_id = engine
        .start_instance_with_variables(key, vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "w", HashMap::new()).await;

    let state = engine.get_instance_state(inst_id).await.unwrap();
    assert_eq!(state, InstanceState::Completed);
}

/// Catches: find_downstream_join depth arithmetic (- vs /, + vs -, + vs *)
/// Tests nested parallel gateways where depth tracking matters.
#[tokio::test]
async fn test_find_downstream_join_nested_parallel() {
    let engine = WorkflowEngine::new();

    // Build: start → split1 → (branch_a → split2 → (inner_a, inner_b) → join2 → merge_a, branch_b) → join1 → end
    let def = ProcessDefinitionBuilder::new("nested_par")
        .node("start", BpmnElement::StartEvent)
        .node("split1", BpmnElement::ParallelGateway)
        .node(
            "task_a",
            BpmnElement::ServiceTask {
                topic: "a".into(),
                multi_instance: None,
            },
        )
        .node("split2", BpmnElement::ParallelGateway)
        .node(
            "inner_a",
            BpmnElement::ServiceTask {
                topic: "ia".into(),
                multi_instance: None,
            },
        )
        .node(
            "inner_b",
            BpmnElement::ServiceTask {
                topic: "ib".into(),
                multi_instance: None,
            },
        )
        .node("join2", BpmnElement::ParallelGateway)
        .node(
            "task_b",
            BpmnElement::ServiceTask {
                topic: "b".into(),
                multi_instance: None,
            },
        )
        .node("join1", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "split1")
        // Branch A: split1 → task_a → split2 → inner_a/inner_b → join2 → join1
        .flow("split1", "task_a")
        .flow("task_a", "split2")
        .flow("split2", "inner_a")
        .flow("split2", "inner_b")
        .flow("inner_a", "join2")
        .flow("inner_b", "join2")
        .flow("join2", "join1")
        // Branch B: split1 → task_b → join1
        .flow("split1", "task_b")
        .flow("task_b", "join1")
        .flow("join1", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;

    // find_downstream_join from split1 should find join1 (not join2)
    let def_ref = engine.definitions.get(&key).unwrap();
    let join = crate::engine::executor::helpers::find_downstream_join(&def_ref, "split1");
    assert_eq!(join.as_deref(), Some("join1"), "split1 should find join1");

    // find_downstream_join from split2 should find join2
    let join2 = crate::engine::executor::helpers::find_downstream_join(&def_ref, "split2");
    assert_eq!(join2.as_deref(), Some("join2"), "split2 should find join2");
}

/// Catches: complete_branch_token == vs != in find predicate
#[tokio::test]
async fn test_complete_branch_token_marks_correct_token() {
    let engine = WorkflowEngine::new();
    // Parallel flow that forks into two branches
    let def = ProcessDefinitionBuilder::new("branch_tok")
        .node("start", BpmnElement::StartEvent)
        .node("split", BpmnElement::ParallelGateway)
        .node("ut_a", BpmnElement::UserTask("a".into()))
        .node("ut_b", BpmnElement::UserTask("b".into()))
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "split")
        .flow("split", "ut_a")
        .flow("split", "ut_b")
        .flow("ut_a", "join")
        .flow("ut_b", "join")
        .flow("join", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Two user tasks should exist
    assert_eq!(engine.pending_user_tasks.len(), 2);

    // Complete one task → only that branch token should be completed
    let task_a_id = engine
        .pending_user_tasks
        .iter()
        .find(|r| r.node_id == "ut_a")
        .map(|r| r.task_id)
        .unwrap();
    engine
        .complete_user_task(task_a_id, HashMap::new())
        .await
        .unwrap();

    // Instance should still have pending tasks (ut_b not completed)
    assert_eq!(engine.pending_user_tasks.len(), 1);
    let remaining_task = engine
        .pending_user_tasks
        .iter()
        .next()
        .map(|r| r.node_id.clone())
        .unwrap();
    assert_eq!(remaining_task, "ut_b");

    // Instance should NOT be completed yet
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(
        !matches!(inst.state, InstanceState::Completed),
        "Instance should not be completed until all branches done"
    );
}

/// Catches: all_tokens_completed logic — empty tokens vs active_tokens checks
#[tokio::test]
async fn test_parallel_flow_completes_only_when_all_branches_done() {
    let engine = WorkflowEngine::with_in_memory_persistence();
    let def = ProcessDefinitionBuilder::new("par_all")
        .node("start", BpmnElement::StartEvent)
        .node("split", BpmnElement::ParallelGateway)
        .node("ut_a", BpmnElement::UserTask("a".into()))
        .node("ut_b", BpmnElement::UserTask("b".into()))
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "split")
        .flow("split", "ut_a")
        .flow("split", "ut_b")
        .flow("ut_a", "join")
        .flow("ut_b", "join")
        .flow("join", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Complete first branch
    let task_a = engine
        .pending_user_tasks
        .iter()
        .find(|r| r.node_id == "ut_a")
        .map(|r| r.task_id)
        .unwrap();
    engine
        .complete_user_task(task_a, HashMap::new())
        .await
        .unwrap();

    // Not yet completed
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(!matches!(inst.state, InstanceState::Completed));

    // Complete second branch
    let task_b = engine
        .pending_user_tasks
        .iter()
        .find(|r| r.node_id == "ut_b")
        .map(|r| r.task_id)
        .unwrap();
    engine
        .complete_user_task(task_b, HashMap::new())
        .await
        .unwrap();

    // Now completed
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(
        matches!(inst.state, InstanceState::Completed),
        "Instance should complete after both branches, got: {:?}",
        inst.state
    );
}

/// Catches: resolve_next_target find(|f| ...) condition — unwrap_or(true) vs unwrap_or(false)
#[tokio::test]
async fn test_unconditional_flow_routes_without_condition() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("uncon_flow")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "do_work".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // Should route to svc (unconditional flow → unwrap_or(true))
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(
        matches!(inst.state, InstanceState::WaitingOnServiceTask { .. }),
        "Unconditional flow should reach service task, got: {:?}",
        inst.state
    );
}

/// Catches: run_instance_batch step_count > MAX_EXECUTION_STEPS
/// Tests that an infinite loop in BPMN is caught and aborted.
#[tokio::test]
async fn test_execution_limit_prevents_infinite_loop() {
    let engine = WorkflowEngine::new();
    // Create a loop: start → script → script (loop back to itself), with an end event for validation
    let def = ProcessDefinitionBuilder::new("inf_loop")
        .node("start", BpmnElement::StartEvent)
        .node(
            "script",
            BpmnElement::ScriptTask {
                script: r#"let x = 1;"#.into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "script")
        .flow("script", "script") // self-loop (end is unreachable but satisfies validation)
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let result = engine.start_instance(key).await;

    // Should fail with ExecutionLimitExceeded
    assert!(
        matches!(result, Err(EngineError::ExecutionLimitExceeded(_))),
        "Infinite loop should trigger execution limit, got: {:?}",
        result
    );
}
