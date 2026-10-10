//! Pagination for `WorkflowEngine::list_instances_page`.

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

async fn deploy_start_end(engine: &WorkflowEngine) -> Uuid {
    let def = ProcessDefinitionBuilder::new("page")
        .node("start", BpmnElement::StartEvent)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    key
}

fn dummy_instance(
    id: Uuid,
    definition_key: Uuid,
    started_at: Option<chrono::DateTime<Utc>>,
) -> ProcessInstance {
    ProcessInstance {
        id,
        definition_key,
        business_key: String::new(),
        parent_instance_id: None,
        state: InstanceState::Running,
        current_node: "start".into(),
        audit_log: vec![],
        variables: HashMap::new(),
        tokens: HashMap::new(),
        active_tokens: vec![],
        join_barriers: HashMap::new(),
        multi_instance_state: HashMap::new(),
        compensation_log: Vec::new(),
        outstanding_calls: HashMap::new(),
        started_at,
        completed_at: None,
    }
}

#[tokio::test]
async fn list_instances_page_offset_limit_clones_slice() {
    let engine = WorkflowEngine::new();
    let key = deploy_start_end(&engine).await;
    for _ in 0..5 {
        engine.start_instance(key).await.unwrap();
    }

    let page = engine.list_instances_page(2, Some(2)).await;
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total, 5);

    let all = engine.list_instances().await;
    assert_eq!(all.len(), 5);
    assert_eq!(page.items[0].id, all[2].id);
    assert_eq!(page.items[1].id, all[3].id);
}

#[tokio::test]
async fn list_instances_without_limit_returns_all() {
    let engine = WorkflowEngine::new();
    let key = deploy_start_end(&engine).await;
    for _ in 0..5 {
        engine.start_instance(key).await.unwrap();
    }

    let instances = engine.list_instances().await;
    assert_eq!(instances.len(), 5);

    let page = engine.list_instances_page(0, None).await;
    assert_eq!(page.items.len(), 5);
    assert_eq!(page.total, 5);
}

#[tokio::test]
async fn list_instances_page_sorts_started_at_desc_then_id() {
    let engine = WorkflowEngine::new();
    let key = deploy_start_end(&engine).await;
    let t0 = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

    // Same timestamp → id ascending as tie-break.
    let early_a = Uuid::from_u128(10);
    let early_b = Uuid::from_u128(20);
    engine
        .restore_instance(dummy_instance(early_b, key, Some(t0)))
        .await;
    engine
        .restore_instance(dummy_instance(early_a, key, Some(t0)))
        .await;

    let ids = [
        Uuid::from_u128(1),
        Uuid::from_u128(2),
        Uuid::from_u128(3),
        Uuid::from_u128(4),
        Uuid::from_u128(5),
    ];
    for (i, id) in ids.iter().enumerate() {
        engine
            .restore_instance(dummy_instance(
                *id,
                key,
                Some(t0 + Duration::seconds((i as i64) + 1)),
            ))
            .await;
    }

    // Desc started_at: 5,4,3,2,1, then (t0, id 10), (t0, id 20)
    let page = engine.list_instances_page(2, Some(2)).await;
    assert_eq!(page.total, 7);
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.items[0].id, ids[2]); // 3rd newest = id 3
    assert_eq!(page.items[1].id, ids[1]); // 4th newest = id 2

    let tail = engine.list_instances_page(5, Some(2)).await;
    assert_eq!(tail.items.len(), 2);
    assert_eq!(tail.items[0].id, early_a);
    assert_eq!(tail.items[1].id, early_b);
}
