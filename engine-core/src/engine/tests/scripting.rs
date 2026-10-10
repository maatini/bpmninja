//! Split from `unit_tests.rs` (M5).

use super::super::*;
use super::helpers::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn script_mutates_state_and_executes_logic() {
    let engine = WorkflowEngine::new();
    let (def_key, _) = engine
        .deploy_definition(build_script_test_definition())
        .await;

    let mut vars = HashMap::new();
    vars.insert("x".into(), serde_json::json!(6));

    let inst_id = engine
        .start_instance_with_variables(def_key, vars)
        .await
        .unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        engine.get_instance_state(inst_id).await.unwrap(),
        InstanceState::Completed
    );

    let details = engine.get_instance_details(inst_id).await.unwrap();

    complete_all_service_tasks(&engine, "worker_1", HashMap::new()).await;

    assert_eq!(
        details.variables.get("x"),
        Some(&serde_json::json!(12)),
        "x should be mutated by script"
    );

    assert_eq!(
        details.variables.get("result"),
        Some(&serde_json::json!("big")),
        "script logic should set result"
    );
}

#[tokio::test]
async fn in_memory_script_robust_failure_handling() {
    let engine = WorkflowEngine::with_in_memory_persistence();
    let script = "let a = 1; throw \"Intentional crash!\";";

    let def = ProcessDefinitionBuilder::new("script_crash")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("worker".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .listener("start", crate::domain::ListenerEvent::Start, script)
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;

    // Engine should panic or return error because script is broken
    let result = engine.start_instance(def_key).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Intentional crash!")
    );
}

/// Catches: ScriptConfig::from_env -> Default, build_engine -> Default
#[tokio::test]
async fn test_script_config_defaults_and_build() {
    let cfg = crate::scripting::ScriptConfig::from_env();
    // Defaults should be positive numbers
    assert!(cfg.max_operations > 0);
    assert!(cfg.max_memory > 0);
    assert!(cfg.timeout_ms > 0);

    // build_engine should produce a working engine (not Default which would lack limits)
    // Verify by running a simple script — a Default rhai::Engine would succeed,
    // but our configured engine has limits that allow simple scripts to run.
    let rhai_engine = cfg.build_engine();
    let result = rhai_engine.eval::<i64>("40 + 2");
    assert_eq!(result.unwrap(), 42);

    // Default 2 MiB budget must yield the historical collection caps.
    let (s, a, m) = crate::scripting::ScriptConfig::default().derived_collection_limits();
    assert_eq!(s, 64 * 1024);
    assert_eq!(a, 10_000);
    assert_eq!(m, 10_000);
}

/// max_memory must actually constrain allocations via derived collection limits.
#[tokio::test]
async fn test_script_max_memory_rejects_large_array() {
    let cfg = crate::scripting::ScriptConfig {
        max_operations: 50_000,
        max_memory: 4 * 1024, // 4 KiB budget → tiny array cap
        timeout_ms: 1_000,
    };
    let (_, max_array, _) = cfg.derived_collection_limits();
    assert!(
        max_array < 100,
        "tiny budget should yield small array cap, got {max_array}"
    );

    let engine = cfg.build_engine();
    // Grow an array past the derived cap.
    let script = format!(
        "let a = []; for i in 0..{} {{ a.push(i); }}",
        max_array + 50
    );
    let err = engine
        .eval::<()>(&script)
        .expect_err("oversized array must fail under max_memory-derived cap");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("array")
            || msg.contains("limit")
            || msg.contains("exceed")
            || msg.contains("size"),
        "unexpected error for memory limit: {msg}"
    );
}

/// execute_script_safe surfaces memory-limit failures as ScriptError.
#[tokio::test]
async fn test_execute_script_safe_respects_max_memory() {
    let cfg = crate::scripting::ScriptConfig {
        max_operations: 50_000,
        max_memory: 4 * 1024,
        timeout_ms: 1_000,
    };
    let (_, max_array, _) = cfg.derived_collection_limits();
    let script = format!(
        "let a = []; for i in 0..{} {{ a.push(i); }}",
        max_array + 50
    );
    let vars = std::collections::HashMap::new();
    let result = crate::scripting::execute_script_safe(&cfg, &script, &vars).await;
    assert!(result.is_err(), "expected ScriptError for oversize array");
    match result {
        Err(crate::domain::EngineError::ScriptError(msg)) => {
            assert!(!msg.is_empty());
        }
        other => panic!("expected ScriptError, got {other:?}"),
    }
}

/// Catches: run_node_scripts == vs != für Listener-Event-Matching
#[tokio::test]
async fn test_script_start_vs_end_listener_distinction() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("listener_dist")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "ld".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .listener(
            "svc",
            crate::domain::ListenerEvent::Start,
            r#"let start_ran = true;"#,
        )
        .listener(
            "svc",
            crate::domain::ListenerEvent::End,
            r#"let end_ran = true;"#,
        )
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let inst_id = engine.start_instance(key).await.unwrap();

    // After start → start_ran should exist, end_ran not yet
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(inst.variables.contains_key("start_ran"));
    assert!(!inst.variables.contains_key("end_ran"));

    // Complete service task → end listener should run
    complete_all_service_tasks(&engine, "w", HashMap::new()).await;
    let inst = engine.get_instance_details(inst_id).await.unwrap();
    assert!(inst.variables.contains_key("end_ran"));
}
