use crate::{
    BpmnElement, Collaboration, DefinitionCollection, MessageEndpoint, MessageFlow, Participant,
    PlannedMessageReceiver as Receiver, ProcessDefinition, ProcessDefinitionBuilder,
    ReceiverUnavailable as Unavailable, WorkflowEngine, WorkflowPersistence,
};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

fn sender() -> ProcessDefinition {
    ProcessDefinitionBuilder::new("sender")
        .node("s", BpmnElement::StartEvent)
        .node(
            "send",
            BpmnElement::SendTask {
                message_name: "order".into(),
                multi_instance: None,
            },
        )
        .node("e", BpmnElement::EndEvent)
        .flow("s", "send")
        .flow("send", "e")
        .build()
        .unwrap()
}
fn receiver(start: bool) -> ProcessDefinition {
    let builder = ProcessDefinitionBuilder::new("receiver");
    let builder = if start {
        builder.node(
            "receive",
            BpmnElement::MessageStartEvent {
                message_name: "order".into(),
            },
        )
    } else {
        builder
            .node("s", BpmnElement::StartEvent)
            .node(
                "receive",
                BpmnElement::MessageCatchEvent {
                    message_name: "order".into(),
                },
            )
            .flow("s", "receive")
    };
    builder
        .node("work", BpmnElement::UserTask("worker".into()))
        .node("e", BpmnElement::EndEvent)
        .flow("receive", "work")
        .flow("work", "e")
        .build()
        .unwrap()
}
fn metadata() -> serde_json::Value {
    serde_json::to_value(vec![Collaboration {
        id: "c".into(),
        participants: vec![
            Participant {
                id: "ps".into(),
                process_id: Some("sender".into()),
                executable: true,
            },
            Participant {
                id: "pt".into(),
                process_id: Some("receiver".into()),
                executable: true,
            },
        ],
        message_flows: vec![MessageFlow {
            id: "flow".into(),
            message_id: "declaration".into(),
            message_name: "order".into(),
            source: MessageEndpoint {
                participant_id: "ps".into(),
                process_id: "sender".into(),
                element_id: "send".into(),
            },
            target: MessageEndpoint {
                participant_id: "pt".into(),
                process_id: "receiver".into(),
                element_id: "receive".into(),
            },
        }],
    }])
    .unwrap()
}
async fn deploy(engine: &WorkflowEngine, receiver: ProcessDefinition) -> DefinitionCollection {
    engine
        .deploy_collection(vec![sender(), receiver], "xml".into(), metadata())
        .await
        .unwrap()
}
fn key(collection: &DefinitionCollection, id: &str) -> Uuid {
    collection
        .definitions
        .iter()
        .find(|d| d.id == id)
        .unwrap()
        .key
}
async fn wait(engine: &WorkflowEngine, definition: Uuid, business: &str) -> Uuid {
    engine
        .start_instance_with_variables(
            definition,
            HashMap::from([("business_key".into(), json!(business))]),
        )
        .await
        .unwrap()
}
async fn plan(
    engine: &WorkflowEngine,
    collection: &DefinitionCollection,
) -> Vec<crate::InternalMessagePlan> {
    engine
        .plan_internal_messages(
            key(collection, "sender"),
            "send",
            "order-1",
            HashMap::from([("amount".into(), json!(5))]),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn start_routes_pin_version_and_snapshot_without_starting_instances() {
    let engine = WorkflowEngine::new();
    let old = deploy(&engine, receiver(true)).await;
    let newer = deploy(&engine, receiver(true)).await;
    let plans = plan(&engine, &old).await;
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].receiver, Receiver::Start);
    assert_eq!(plans[0].route.target.definition_key, key(&old, "receiver"));
    assert_ne!(
        plans[0].route.target.definition_key,
        key(&newer, "receiver")
    );
    assert_eq!(plans[0].route.target.version, 1);
    assert_eq!(plans[0].route.collection_id, old.id);
    assert_eq!(plans[0].route.message_id, "declaration");
    assert_eq!(plans[0].variables["amount"], 5);
    assert!(engine.instances.all().await.is_empty());
    let roundtrip: crate::InternalMessagePlan =
        serde_json::from_value(serde_json::to_value(&plans[0]).unwrap()).unwrap();
    assert_eq!(roundtrip, plans[0]);
}
#[tokio::test]
async fn exact_catch_key_business_key_and_element_are_required() {
    let engine = WorkflowEngine::new();
    let old = deploy(&engine, receiver(false)).await;
    let newer = deploy(&engine, receiver(false)).await;
    let intended = wait(&engine, key(&old, "receiver"), "order-1").await;
    wait(&engine, key(&old, "receiver"), "order-2").await;
    wait(&engine, key(&newer, "receiver"), "order-1").await;
    // Another pool with the same message name never participates.
    let mut other = receiver(false);
    other.id = "unrelated".into();
    let (other, _) = engine.deploy_definition(other).await;
    wait(&engine, other, "order-1").await;
    let mut stale = engine
        .pending_message_catches
        .iter()
        .find(|c| c.instance_id == intended)
        .unwrap()
        .value()
        .clone();
    stale.id = Uuid::new_v4();
    let stale_id = stale.id;
    engine.pending_message_catches.insert(stale.id, stale);
    let plans = plan(&engine, &old).await;
    assert!(
        matches!(plans[0].receiver, Receiver::Catch { instance_id, catch_id, .. } if instance_id == intended && catch_id != stale_id)
    );
    assert_eq!(engine.pending_message_catches.len(), 5); // planning consumes nothing
    assert!(
        engine
            .plan_internal_messages(key(&old, "sender"), "send", " ", HashMap::new())
            .await
            .is_err()
    );
    assert!(
        engine
            .plan_internal_messages(key(&old, "sender"), "e", "order-1", HashMap::new())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn unavailable_ambiguous_and_suspended_receivers_are_explicit() {
    let engine = WorkflowEngine::new();
    let batch = deploy(&engine, receiver(false)).await;
    assert_eq!(
        plan(&engine, &batch).await[0].receiver,
        Receiver::Unavailable(Unavailable::NoMatchingCatch)
    );
    let a = wait(&engine, key(&batch, "receiver"), "order-1").await;
    let b = wait(&engine, key(&batch, "receiver"), "order-1").await;
    assert_eq!(
        plan(&engine, &batch).await[0].receiver,
        Receiver::Unavailable(Unavailable::AmbiguousCatches { count: 2 })
    );
    engine.suspend_instance(a).await.unwrap();
    assert!(
        matches!(plan(&engine,&batch).await[0].receiver, Receiver::Catch { instance_id, .. } if instance_id==b)
    );
    // An archived/removed receiver's stale catch must not be selected.
    engine.instances.remove(&b).await;
    assert_eq!(
        plan(&engine, &batch).await[0].receiver,
        Receiver::Unavailable(Unavailable::NoMatchingCatch)
    );
}
#[tokio::test]
async fn deleted_target_does_not_fall_back_and_deleted_source_cannot_route() {
    let engine = WorkflowEngine::new();
    let batch = deploy(&engine, receiver(true)).await;
    deploy(&engine, receiver(true)).await;
    engine
        .delete_definition(key(&batch, "receiver"), false)
        .await
        .unwrap();
    assert_eq!(
        plan(&engine, &batch).await[0].receiver,
        Receiver::Unavailable(Unavailable::DefinitionDeleted)
    );
    engine
        .delete_definition(key(&batch, "sender"), false)
        .await
        .unwrap();
    assert!(
        engine
            .plan_internal_messages(key(&batch, "sender"), "send", "order-1", HashMap::new())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn explicit_flows_fan_out_in_document_order_and_unrouted_sends_do_not_broadcast() {
    let engine = WorkflowEngine::new();
    let mut metadata = metadata();
    let mut extra = metadata[0]["message_flows"][0].clone();
    extra["id"] = json!("second-flow");
    metadata[0]["message_flows"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    let batch = engine
        .deploy_collection(vec![sender(), receiver(true)], "xml".into(), metadata)
        .await
        .unwrap();
    let plans = plan(&engine, &batch).await;
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0].route.flow_id, "flow");
    assert_eq!(plans[1].route.flow_id, "second-flow");
    let (legacy, _) = engine.deploy_definition(sender()).await;
    assert!(
        engine
            .plan_internal_messages(legacy, "send", "order-1", HashMap::new())
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn invalid_routes_fail_before_commit_and_before_restoration_publication() {
    let persistence = std::sync::Arc::new(crate::adapter::InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(persistence.clone());
    let mut bad = metadata();
    bad[0]["message_flows"][0]["target"]["element_id"] = json!("work");
    assert!(
        engine
            .deploy_collection(vec![sender(), receiver(true)], "xml".into(), bad)
            .await
            .is_err()
    );
    assert!(engine.list_definitions().await.is_empty());
    assert!(
        persistence
            .list_definition_collections()
            .await
            .unwrap()
            .is_empty()
    );
    let batch = deploy(&engine, receiver(true)).await;
    let mut restored = serde_json::to_value(&batch).unwrap();
    restored["collaborations"][0]["message_flows"][0]["message_name"] = json!("different");
    let restored = serde_json::from_value(restored).unwrap();
    let fresh = WorkflowEngine::new();
    assert!(fresh.restore_definition_collection(&restored).is_err());
    assert!(fresh.list_definitions().await.is_empty());
    engine.shutdown().await;
}

#[tokio::test]
async fn gateway_alternatives_select_only_the_routed_element() {
    let engine = WorkflowEngine::new();
    let target = ProcessDefinitionBuilder::new("receiver")
        .node("s", BpmnElement::StartEvent)
        .node("gateway", BpmnElement::EventBasedGateway)
        .node(
            "receive",
            BpmnElement::MessageCatchEvent {
                message_name: "order".into(),
            },
        )
        .node(
            "other",
            BpmnElement::MessageCatchEvent {
                message_name: "order".into(),
            },
        )
        .node("e", BpmnElement::EndEvent)
        .flow("s", "gateway")
        .flow("gateway", "receive")
        .flow("gateway", "other")
        .flow("receive", "e")
        .flow("other", "e")
        .build()
        .unwrap();
    let batch = deploy(&engine, target).await;
    let instance = wait(&engine, key(&batch, "receiver"), "order-1").await;
    let plans = plan(&engine, &batch).await;
    let Receiver::Catch {
        instance_id,
        catch_id,
        ..
    } = plans[0].receiver
    else {
        panic!("expected exact gateway alternative: {:?}", plans[0]);
    };
    assert_eq!(instance_id, instance);
    assert_eq!(
        engine
            .pending_message_catches
            .get(&catch_id)
            .unwrap()
            .node_id,
        "receive"
    );
    assert_eq!(engine.pending_message_catches.len(), 2);
}
#[tokio::test]
async fn boundary_receiver_remains_an_observation_of_the_current_activity() {
    let engine = WorkflowEngine::new();
    let target = ProcessDefinitionBuilder::new("receiver")
        .node("s", BpmnElement::StartEvent)
        .node("work", BpmnElement::UserTask("worker".into()))
        .node(
            "receive",
            BpmnElement::BoundaryMessageEvent {
                attached_to: "work".into(),
                message_name: "order".into(),
                cancel_activity: true,
            },
        )
        .node("e", BpmnElement::EndEvent)
        .flow("s", "work")
        .flow("work", "e")
        .flow("receive", "e")
        .build()
        .unwrap();
    let batch = deploy(&engine, target).await;
    let instance = wait(&engine, key(&batch, "receiver"), "order-1").await;
    assert!(
        matches!(plan(&engine,&batch).await[0].receiver,Receiver::Catch { instance_id, .. } if instance_id==instance)
    );
    assert_eq!(engine.pending_user_tasks.len(), 1);
    let task = engine.pending_user_tasks.iter().next().unwrap().task_id;
    engine
        .complete_user_task(task, HashMap::new())
        .await
        .unwrap();
    assert_eq!(
        plan(&engine, &batch).await[0].receiver,
        Receiver::Unavailable(Unavailable::NoMatchingCatch)
    );
}
