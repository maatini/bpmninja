use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::{
    ActivityMappings, DataMapping, ListenerEvent, LoopCharacteristics, MappingValue,
    MultiInstanceDef, ProcessDefinitionBuilder,
};
use crate::persistence::WorkflowPersistence;
use serde_json::json;

fn mapping(target: &str, source: &str) -> DataMapping {
    DataMapping {
        target: target.into(),
        value: MappingValue::Variable(source.into()),
    }
}
fn definition(activity: BpmnElement) -> ProcessDefinition {
    let mut def = ProcessDefinitionBuilder::new("data")
        .node("start", BpmnElement::StartEvent)
        .node("act", activity)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "act")
        .flow("act", "end")
        .build()
        .unwrap();
    def.data_variables = vec!["customerId".into()];
    def.variable_defaults.insert("customerId".into(), json!(7));
    def.data_mappings.insert(
        "act".into(),
        ActivityMappings {
            inputs: vec![mapping("id", "customerId")],
            outputs: vec![mapping("receipt", "result")],
            ..Default::default()
        },
    );
    def
}
fn service() -> BpmnElement {
    BpmnElement::ServiceTask {
        topic: "mapped".into(),
        multi_instance: None,
    }
}
async fn lock(engine: &WorkflowEngine) -> PendingServiceTask {
    engine
        .fetch_and_lock_service_tasks("worker", 1, &["mapped".into()], 60)
        .await
        .remove(0)
}

#[tokio::test]
async fn defaults_inputs_outputs_and_listener_order() {
    let engine = WorkflowEngine::new();
    let mut def = definition(BpmnElement::ScriptTask {
        script: "let result = id * 2;".into(),
        multi_instance: None,
    });
    def.listeners.insert(
        "act".into(),
        vec![
            crate::domain::ExecutionListener {
                event: ListenerEvent::Start,
                script: "let saw_input = id;".into(),
            },
            crate::domain::ExecutionListener {
                event: ListenerEvent::End,
                script: "let saw_output = receipt;".into(),
            },
        ],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine
        .start_instance_with_variables(key, HashMap::from([("customerId".into(), json!(9))]))
        .await
        .unwrap();
    let inst = engine.get_instance_details(id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.variables["receipt"], json!(18));
    assert_eq!(inst.variables["saw_output"], json!(18));
    for name in ["id", "result", "saw_input"] {
        assert!(!inst.variables.contains_key(name), "{name} leaked");
    }
}

#[tokio::test]
async fn missing_output_rolls_back_and_keeps_retryable_task() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(definition(service())).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = lock(&engine).await;
    assert_eq!(task.variables_snapshot["id"], json!(7));
    assert!(
        !engine
            .get_instance_details(id)
            .await
            .unwrap()
            .variables
            .contains_key("id")
    );
    assert!(
        engine
            .complete_service_task(task.id, "worker", HashMap::new())
            .await
            .is_err()
    );
    assert_eq!(engine.get_pending_service_tasks().len(), 1);
    assert!(
        engine.get_instance_details(id).await.unwrap().tokens[&task.token_id]
            .data_scope
            .is_some()
    );
    engine
        .complete_service_task(
            task.id,
            "worker",
            HashMap::from([("result".into(), json!(42))]),
        )
        .await
        .unwrap();
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().variables["receipt"],
        json!(42)
    );
}

#[tokio::test]
async fn mapping_scope_and_definition_survive_serialization_and_restart() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let def = definition(service());
    let persisted_def: ProcessDefinition =
        serde_json::from_str(&serde_json::to_string(&def).unwrap()).unwrap();
    let (key, _) = engine.deploy_definition(persisted_def).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = lock(&engine).await;
    engine
        .fail_service_task(task.id, "worker", Some(2), Some("retry".into()), None)
        .await
        .unwrap();
    engine
        .update_instance_variables(
            id,
            HashMap::from([("customerId".into(), json!(99)), ("id".into(), json!(100))]),
        )
        .await
        .unwrap();
    drop(engine);
    let restored = WorkflowEngine::new().with_persistence(persistence.clone());
    for def in persistence.list_definitions().await.unwrap() {
        restored.deploy_definition(def).await;
    }
    for instance in persistence.list_instances().await.unwrap() {
        let encoded = serde_json::to_string(&instance).unwrap();
        restored
            .restore_instance(serde_json::from_str(&encoded).unwrap())
            .await;
    }
    for task in persistence.list_service_tasks().await.unwrap() {
        restored.restore_service_task(task);
    }
    let task = lock(&restored).await;
    let token = &restored.get_instance_details(id).await.unwrap().tokens[&task.token_id];
    assert_eq!(
        token.variables["id"],
        json!(7),
        "inputs must not be rerun or replaced by process updates"
    );
    restored
        .complete_service_task(
            task.id,
            "worker",
            HashMap::from([("result".into(), json!(42))]),
        )
        .await
        .unwrap();
    let inst = restored.get_instance_details(id).await.unwrap();
    assert_eq!(inst.variables["receipt"], json!(42));
    assert_eq!(inst.variables["customerId"], json!(99));
    assert_eq!(
        inst.variables["id"],
        json!(100),
        "shadowed process value must survive"
    );
}

#[tokio::test]
async fn user_task_mapping_failure_keeps_pending_activation() {
    let engine = WorkflowEngine::new();
    let (key, _) = engine
        .deploy_definition(definition(BpmnElement::UserTask("user".into())))
        .await;
    let id = engine.start_instance(key).await.unwrap();
    let task = engine.get_pending_user_tasks().remove(0);
    assert!(
        engine
            .complete_user_task(task.task_id, HashMap::new())
            .await
            .is_err()
    );
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    engine
        .complete_user_task(task.task_id, HashMap::from([("result".into(), json!(23))]))
        .await
        .unwrap();
    assert_eq!(
        engine.get_instance_details(id).await.unwrap().variables["receipt"],
        json!(23)
    );
}

#[tokio::test]
async fn parallel_and_sequential_mappings_are_per_iteration() {
    for sequential in [false, true] {
        let engine = WorkflowEngine::new();
        let mut def = definition(BpmnElement::ScriptTask {
            script: "let result = id * 2;".into(),
            multi_instance: None,
        });
        def.data_mappings.get_mut("act").unwrap().inputs = vec![mapping("id", "item")];
        def.loops.insert(
            "act".into(),
            LoopCharacteristics::MultiInstance(MultiInstanceDef {
                is_sequential: sequential,
                loop_cardinality: None,
                collection: Some("${items}".into()),
                element_variable: Some("item".into()),
                completion_condition: None,
                output_collection: Some("receipts".into()),
                output_element: Some("receipt".into()),
            }),
        );
        let (key, _) = engine.deploy_definition(def).await;
        let id = engine
            .start_instance_with_variables(key, HashMap::from([("items".into(), json!([1, 2, 3]))]))
            .await
            .unwrap();
        let inst = engine.get_instance_details(id).await.unwrap();
        assert_eq!(inst.variables["receipts"], json!([2, 4, 6]));
        assert!(!inst.variables.contains_key("id"));
    }
}

#[tokio::test]
async fn explicit_call_transfer_filters_child_variables() {
    let engine = WorkflowEngine::new();
    let child = ProcessDefinitionBuilder::new("child")
        .node("start", BpmnElement::StartEvent)
        .node(
            "work",
            BpmnElement::ScriptTask {
                script: "let answer = childId * 2; let secret = 999;".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "work")
        .flow("work", "end")
        .build()
        .unwrap();
    engine.deploy_definition(child).await;
    let mut parent = definition(BpmnElement::CallActivity {
        called_element: "child".into(),
    });
    let mappings = parent.data_mappings.get_mut("act").unwrap();
    mappings.call_inputs = Some(vec![mapping("childId", "id")]);
    mappings.call_outputs = Some(vec![mapping("result", "answer")]);
    let (key, _) = engine.deploy_definition(parent).await;
    let id = engine.start_instance(key).await.unwrap();
    let inst = engine.get_instance_details(id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert_eq!(inst.variables["receipt"], json!(14));
    for name in ["secret", "childId", "answer", "id"] {
        assert!(!inst.variables.contains_key(name));
    }
}

#[tokio::test]
async fn interrupting_error_boundary_discards_locals_and_outputs() {
    let engine = WorkflowEngine::new();
    let mut def = definition(service());
    def.nodes.insert(
        "boundary".into(),
        BpmnElement::BoundaryErrorEvent {
            attached_to: "act".into(),
            error_code: None,
        },
    );
    def.flows.insert(
        "boundary".into(),
        vec![crate::domain::SequenceFlow {
            target: "end".into(),
            condition: None,
        }],
    );
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    let task = lock(&engine).await;
    engine
        .handle_bpmn_error(task.id, "worker", "FAIL")
        .await
        .unwrap();
    let inst = engine.get_instance_details(id).await.unwrap();
    assert_eq!(inst.state, InstanceState::Completed);
    assert!(!inst.variables.contains_key("id"));
    assert!(!inst.variables.contains_key("receipt"));
}

#[test]
fn old_tokens_and_definitions_deserialize_without_mapping_state() {
    let token = Token::new("act");
    let mut encoded = serde_json::to_value(token).unwrap();
    encoded.as_object_mut().unwrap().remove("data_scope");
    assert!(
        serde_json::from_value::<Token>(encoded)
            .unwrap()
            .data_scope
            .is_none()
    );
    let mut encoded = serde_json::to_value(definition(service())).unwrap();
    for key in ["data_variables", "variable_defaults", "data_mappings"] {
        encoded.as_object_mut().unwrap().remove(key);
    }
    let old: ProcessDefinition = serde_json::from_value(encoded).unwrap();
    assert!(old.data_mappings.is_empty() && old.variable_defaults.is_empty());
}

#[test]
fn mapping_phases_use_one_snapshot_and_fail_atomically() {
    use crate::domain::data::evaluate_mappings;
    let vars = HashMap::from([("a".into(), json!(1)), ("b".into(), json!(2))]);
    let result = evaluate_mappings(&[mapping("a", "b"), mapping("b", "a")], &vars).unwrap();
    assert_eq!(result["a"], json!(2));
    assert_eq!(result["b"], json!(1));
    assert!(evaluate_mappings(&[mapping("a", "b"), mapping("b", "missing")], &vars).is_err());
    assert_eq!(vars["a"], json!(1));
}
