use super::*;
use crate::{ConditionalStartRequest, WorkflowPersistence};
use serde_json::json;

fn definition() -> ProcessDefinition {
    let mut def = crate::ProcessDefinitionBuilder::new("conditional_start")
        .node("start", BpmnElement::ConditionalStartEvent { condition: "amount > 100".into() })
        .node("work", BpmnElement::UserTask("review".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "work").flow("work", "end").build().unwrap();
    def.variable_defaults.insert("amount".into(), json!(150));
    def.variable_defaults.insert("default".into(), json!(true));
    def
}
fn request(key: Uuid, amount: i64) -> ConditionalStartRequest {
    ConditionalStartRequest { request_id: Uuid::new_v4(), definition_key: Some(key), process_id: None,
        business_key: Some("case-42".into()), variables: HashMap::from([("amount".into(), json!(amount))]) }
}
async fn restore(p: Arc<crate::adapter::InMemoryPersistence>) -> WorkflowEngine {
    let engine = WorkflowEngine::new().with_persistence(p.clone());
    for def in p.list_definitions().await.unwrap() { engine.deploy_definition(def).await; }
    for inst in p.list_instances().await.unwrap() { engine.restore_instance(inst).await; }
    for task in p.list_user_tasks().await.unwrap() { engine.restore_user_task(task); }
    engine
}

#[tokio::test]
async fn conditional_start_false_true_defaults_manual_rejection_and_conflicts() {
    let engine = WorkflowEngine::with_in_memory_persistence();
    let (key, _) = engine.deploy_definition(definition()).await;
    assert!(engine.start_instance(key).await.is_err());
    assert!(engine.start_instance_latest("conditional_start", HashMap::new()).await.is_err());
    let no = request(key, 10);
    let result = engine.evaluate_conditional_start(no.clone()).await.unwrap();
    assert!(!result.matched);
    assert!(result.instance_id.is_none());
    assert!(engine.list_instances().await.is_empty());
    assert!(engine.get_pending_user_tasks().is_empty());
    let mut changed = no.clone(); changed.variables.insert("amount".into(), json!(150));
    assert_eq!(engine.evaluate_conditional_start(changed).await.unwrap_err(), EngineError::ConditionalStartConflict(no.request_id));
    assert_eq!(engine.evaluate_conditional_start(no).await.unwrap(), result);
    let mut yes = request(key, 150); yes.variables.clear();
    let result = engine.evaluate_conditional_start(yes.clone()).await.unwrap();
    let id = result.instance_id.unwrap();
    let instance = engine.get_instance_details(id).await.unwrap();
    assert_eq!(instance.business_key, "case-42");
    assert_eq!(instance.variables["amount"], json!(150));
    assert_eq!(instance.variables["default"], json!(true));
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    assert_eq!(engine.evaluate_conditional_start(yes.clone()).await.unwrap(), result);
    for mutation in 0..3 {
        let mut changed = yes.clone();
        match mutation {
            0 => changed.business_key = Some("changed".into()),
            1 => { changed.definition_key = None; changed.process_id = Some("conditional_start".into()); },
            _ => { changed.variables.insert("new".into(), json!(true)); },
        }
        assert_eq!(engine.evaluate_conditional_start(changed).await.unwrap_err(), EngineError::ConditionalStartConflict(yes.request_id));
    }
    engine.update_instance_variables(id, HashMap::from([("amount".into(), json!(200))])).await.unwrap();
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    let task = engine.get_pending_user_tasks().pop().unwrap();
    engine.complete_user_task(task.task_id, HashMap::new()).await.unwrap();
    assert_eq!(engine.evaluate_conditional_start(yes.clone()).await.unwrap(), result);
    assert!(engine.get_pending_user_tasks().is_empty());
    yes.request_id = Uuid::new_v4();
    assert_ne!(engine.evaluate_conditional_start(yes).await.unwrap().instance_id, Some(id));
}

#[tokio::test]
async fn conditional_start_concurrent_structural_inputs_and_latest_pinning() {
    let engine = Arc::new(WorkflowEngine::with_in_memory_persistence());
    let (key, _) = engine.deploy_definition(definition()).await;
    let mut input = request(key, 200);
    input.definition_key = None; input.process_id = Some("conditional_start".into());
    input.variables.insert("object".into(), json!({"a": 1, "b": {"x": 2, "y": 3}}));
    let mut handles = Vec::new();
    for _ in 0..20 {
        let engine = engine.clone(); let input = input.clone();
        handles.push(tokio::spawn(async move { engine.evaluate_conditional_start(input).await.unwrap() }));
    }
    let first = handles.remove(0).await.unwrap();
    for handle in handles { assert_eq!(handle.await.unwrap(), first); }
    assert_eq!(engine.get_pending_user_tasks().len(), 1);
    let mut newer = definition(); newer.nodes.insert("start".into(), BpmnElement::StartEvent);
    engine.deploy_definition(newer).await;
    input.variables.insert("object".into(), serde_json::from_str(r#"{"b":{"y":3,"x":2},"a":1}"#).unwrap());
    assert_eq!(engine.evaluate_conditional_start(input.clone()).await.unwrap(), first);
    assert_eq!(first.definition_key, key);
    input.request_id = Uuid::new_v4();
    assert!(engine.evaluate_conditional_start(input).await.is_err());
}

#[tokio::test]
async fn conditional_start_receipt_and_first_work_recover_without_reevaluation() {
    let p = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(p.clone());
    let (key, _) = engine.deploy_definition(definition()).await;
    let input = request(key, 150);
    let result = engine.evaluate_conditional_start(input.clone()).await.unwrap();
    let id = result.instance_id.unwrap();
    let receipt = p.get_conditional_start(input.request_id).await.unwrap().unwrap();
    // Reserved receipt, no instance snapshot. Stored outcome must survive changed latest/defaults.
    let mut reserved = receipt.clone(); reserved.request.request_id = Uuid::new_v4();
    reserved.result.request_id = reserved.request.request_id;
    reserved.result.instance_id = Some(Uuid::new_v4()); reserved.dispatched = false;
    p.reserve_conditional_start(&reserved).await.unwrap();
    let mut newer = definition(); newer.nodes.insert("start".into(), BpmnElement::ConditionalStartEvent { condition: "amount > 1000".into() });
    engine.deploy_definition(newer).await;
    // Snapshot saved, task publication missing.
    let task = engine.get_pending_user_tasks().pop().unwrap();
    p.delete_user_task(task.task_id).await.unwrap();
    let recovered = restore(p.clone()).await;
    recovered.reconcile_conditional_starts().await.unwrap();
    recovered.reconcile_conditional_starts().await.unwrap();
    assert_eq!(recovered.get_pending_user_tasks().len(), 2);
    assert_eq!(recovered.evaluate_conditional_start(input).await.unwrap(), result);
    assert_eq!(recovered.get_instance_details(id).await.unwrap().definition_key, key);
    let tasks = recovered.get_pending_user_tasks();
    for task in tasks { recovered.complete_user_task(task.task_id, HashMap::new()).await.unwrap(); }
    let recovered = restore(p).await;
    recovered.reconcile_conditional_starts().await.unwrap();
    assert!(recovered.get_pending_user_tasks().is_empty());
}

#[tokio::test]
async fn conditional_start_invalid_input_never_reserves_receipt() {
    let p = Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(p.clone());
    let (key, _) = engine.deploy_definition(definition()).await;
    for variant in 0..5 {
        let mut input = request(key, 200);
        match variant {
            0 => input.request_id = Uuid::nil(),
            1 => input.definition_key = None,
            2 => input.process_id = Some("conditional_start".into()),
            3 => { input.definition_key = None; input.process_id = Some(" ".into()); },
            _ => { input.variables.insert("business_key".into(), json!("wrong place")); },
        }
        assert!(engine.evaluate_conditional_start(input).await.is_err());
    }
    assert!(p.list_conditional_starts().await.unwrap