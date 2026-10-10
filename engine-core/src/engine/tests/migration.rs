//! Split from `unit_tests.rs` (M5).

use super::super::*;
use super::helpers::*;
use crate::domain::ProcessDefinitionBuilder;

#[tokio::test]
async fn migrate_instance_same_node_ids_succeeds() {
    let (engine, key_v1, key_v2) = setup_migration_engine_same_ids().await;

    let inst_id = engine.start_instance(key_v1).await.unwrap();

    // Instanz wartet jetzt auf UserTask "ut"
    let state = engine.get_instance_state(inst_id).await.unwrap();
    assert!(
        matches!(
            state,
            crate::runtime::InstanceState::WaitingOnUserTask { .. }
        ),
        "Unexpected state: {state:?}"
    );

    // Migration ohne Mapping (gleiche Node-IDs)
    let result = engine
        .migrate_instance(inst_id, key_v2, HashMap::new())
        .await;
    assert!(result.is_ok(), "Migration fehlgeschlagen: {result:?}");

    // definition_key muss jetzt v2 sein
    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(details.definition_key, key_v2);
    // Audit-Log enthält Migration-Eintrag
    assert!(
        details.audit_log.iter().any(|e| e.contains("Migriert")),
        "Audit-Log enthält keinen Migrations-Eintrag: {:?}",
        details.audit_log
    );
}

#[tokio::test]
async fn migrate_instance_with_node_mapping_succeeds() {
    let engine = WorkflowEngine::new();

    let def_v1 = ProcessDefinitionBuilder::new("remappable")
        .node("start", BpmnElement::StartEvent)
        .node("old_task", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "old_task")
        .flow("old_task", "end")
        .build()
        .unwrap();

    let def_v2 = ProcessDefinitionBuilder::new("remappable")
        .node("start", BpmnElement::StartEvent)
        .node("new_task", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "new_task")
        .flow("new_task", "end")
        .build()
        .unwrap();

    let (key_v1, _) = engine.deploy_definition(def_v1).await;
    let (key_v2, _) = engine.deploy_definition(def_v2).await;

    let inst_id = engine.start_instance(key_v1).await.unwrap();

    // Mapping: old_task → new_task
    let mut mapping = HashMap::new();
    mapping.insert("old_task".to_string(), "new_task".to_string());

    let result = engine.migrate_instance(inst_id, key_v2, mapping).await;
    assert!(
        result.is_ok(),
        "Migration mit Mapping fehlgeschlagen: {result:?}"
    );

    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(details.definition_key, key_v2);
    assert_eq!(details.current_node, "new_task");
}

#[tokio::test]
async fn migrate_instance_orphaned_token_without_mapping_fails() {
    let engine = WorkflowEngine::new();

    let def_v1 = ProcessDefinitionBuilder::new("orphan_test")
        .node("start", BpmnElement::StartEvent)
        .node("old_task", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "old_task")
        .flow("old_task", "end")
        .build()
        .unwrap();

    let def_v2 = ProcessDefinitionBuilder::new("orphan_test")
        .node("start", BpmnElement::StartEvent)
        .node(
            "completely_different_task",
            BpmnElement::UserTask("alice".into()),
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "completely_different_task")
        .flow("completely_different_task", "end")
        .build()
        .unwrap();

    let (key_v1, _) = engine.deploy_definition(def_v1).await;
    let (key_v2, _) = engine.deploy_definition(def_v2).await;

    let inst_id = engine.start_instance(key_v1).await.unwrap();

    // Kein Mapping → OrphanedToken
    let result = engine
        .migrate_instance(inst_id, key_v2, HashMap::new())
        .await;
    assert!(
        matches!(result, Err(EngineError::OrphanedToken(_))),
        "Erwartet OrphanedToken, got: {result:?}"
    );

    // Instanz darf NICHT verändert worden sein
    let details = engine.get_instance_details(inst_id).await.unwrap();
    assert_eq!(
        details.definition_key, key_v1,
        "definition_key darf sich nicht geändert haben"
    );
    assert_eq!(
        details.current_node, "old_task",
        "current_node darf sich nicht geändert haben"
    );
}

#[tokio::test]
async fn migrate_instance_completed_rejected() {
    let engine = WorkflowEngine::new();

    let def = ProcessDefinitionBuilder::new("auto_complete")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();

    let def_v2 = ProcessDefinitionBuilder::new("auto_complete")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();

    let (key_v1, _) = engine.deploy_definition(def).await;
    let (key_v2, _) = engine.deploy_definition(def_v2).await;

    let inst_id = engine.start_instance(key_v1).await.unwrap();

    // Instanz soll Completed sein (direkt Start→End)
    let state = engine.get_instance_state(inst_id).await.unwrap();
    assert_eq!(state, crate::runtime::InstanceState::Completed);

    let result = engine
        .migrate_instance(inst_id, key_v2, HashMap::new())
        .await;
    assert!(
        matches!(result, Err(EngineError::AlreadyCompleted)),
        "Erwartet AlreadyCompleted, got: {result:?}"
    );
}

#[tokio::test]
async fn migrate_instance_suspended_rejected() {
    let (engine, key_v1, key_v2) = setup_migration_engine_same_ids().await;

    let inst_id = engine.start_instance(key_v1).await.unwrap();
    engine.suspend_instance(inst_id).await.unwrap();

    let result = engine
        .migrate_instance(inst_id, key_v2, HashMap::new())
        .await;
    assert!(
        matches!(result, Err(EngineError::InstanceSuspended(_))),
        "Erwartet InstanceSuspended, got: {result:?}"
    );
}

#[tokio::test]
async fn migrate_instance_unknown_target_definition_fails() {
    let (engine, key_v1, _) = setup_migration_engine_same_ids().await;

    let inst_id = engine.start_instance(key_v1).await.unwrap();
    let nonexistent = Uuid::new_v4();

    let result = engine
        .migrate_instance(inst_id, nonexistent, HashMap::new())
        .await;
    assert!(
        matches!(result, Err(EngineError::NoSuchDefinition(_))),
        "Erwartet NoSuchDefinition, got: {result:?}"
    );
}
