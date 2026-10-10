//! Topic-Index tests (R2): fetch_and_lock uses the secondary topic index.

use std::collections::HashMap;

use super::super::*;
use crate::domain::ProcessDefinitionBuilder;

fn topic_ids(engine: &WorkflowEngine, topic: &str) -> Vec<uuid::Uuid> {
    engine
        .service_task_topic_index
        .get(topic)
        .map(|set| set.iter().copied().collect())
        .unwrap_or_default()
}

async fn deploy_two_topic_instance(engine: &WorkflowEngine) -> uuid::Uuid {
    let def = ProcessDefinitionBuilder::new("topics_ab")
        .node("start", BpmnElement::StartEvent)
        .node("fork", BpmnElement::ParallelGateway)
        .node(
            "svc_a",
            BpmnElement::ServiceTask {
                topic: "topic_a".into(),
                multi_instance: None,
            },
        )
        .node(
            "svc_b",
            BpmnElement::ServiceTask {
                topic: "topic_b".into(),
                multi_instance: None,
            },
        )
        .node("join", BpmnElement::ParallelGateway)
        .node("end", BpmnElement::EndEvent)
        .flow("start", "fork")
        .flow("fork", "svc_a")
        .flow("fork", "svc_b")
        .flow("svc_a", "join")
        .flow("svc_b", "join")
        .flow("join", "end")
        .build()
        .unwrap();

    let (key, _) = engine.deploy_definition(def).await;
    engine.start_instance(key).await.unwrap()
}

#[tokio::test]
async fn fetch_only_requested_topic_leaves_other_index_untouched() {
    let engine = WorkflowEngine::new();
    deploy_two_topic_instance(&engine).await;

    let ids_a_before = topic_ids(&engine, "topic_a");
    let ids_b_before = topic_ids(&engine, "topic_b");
    assert_eq!(ids_a_before.len(), 1);
    assert_eq!(ids_b_before.len(), 1);

    let locked = engine
        .fetch_and_lock_service_tasks("w", 10, &["topic_a".into()], 60)
        .await;
    assert_eq!(locked.len(), 1);
    assert_eq!(locked[0].topic, "topic_a");
    assert_eq!(locked[0].id, ids_a_before[0]);

    assert_eq!(topic_ids(&engine, "topic_b"), ids_b_before);
    let b_task = engine.pending_service_tasks.get(&ids_b_before[0]).unwrap();
    assert!(b_task.worker_id.is_none());
    assert!(b_task.lock_expiration.is_none());
}

#[tokio::test]
async fn complete_removes_task_from_topic_index() {
    let engine = WorkflowEngine::new();
    deploy_two_topic_instance(&engine).await;

    let locked = engine
        .fetch_and_lock_service_tasks("w", 1, &["topic_a".into()], 60)
        .await;
    assert_eq!(locked.len(), 1);
    let task_id = locked[0].id;

    engine
        .complete_service_task(task_id, "w", HashMap::new())
        .await
        .unwrap();

    assert!(!topic_ids(&engine, "topic_a").contains(&task_id));
    assert!(!engine.pending_service_tasks.contains_key(&task_id));
    assert_eq!(topic_ids(&engine, "topic_b").len(), 1);
}

#[tokio::test]
async fn fetch_and_lock_skips_incident_in_topic_index() {
    let engine = WorkflowEngine::new();
    let def = ProcessDefinitionBuilder::new("inc_idx")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "inc_topic".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    engine.start_instance(def_key).await.unwrap();

    let tasks = engine
        .fetch_and_lock_service_tasks("worker", 1, &["inc_topic".into()], 60)
        .await;
    assert_eq!(tasks.len(), 1);
    let task_id = tasks[0].id;

    engine
        .fail_service_task(task_id, "worker", Some(0), Some("Fatal".into()), None)
        .await
        .unwrap();

    assert!(topic_ids(&engine, "inc_topic").contains(&task_id));

    let locked = engine
        .fetch_and_lock_service_tasks("worker2", 10, &["inc_topic".into()], 60)
        .await;
    assert!(locked.is_empty());

    let pending = engine.get_pending_service_tasks();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, task_id);
    assert!(pending[0].retries <= 0);
}

#[tokio::test]
async fn fetch_and_lock_drops_orphaned_index_entries() {
    let engine = WorkflowEngine::new();
    let orphan_id = uuid::Uuid::new_v4();
    engine
        .service_task_topic_index
        .entry("orphan".into())
        .or_default()
        .insert(orphan_id);

    let locked = engine
        .fetch_and_lock_service_tasks("w", 10, &["orphan".into()], 60)
        .await;
    assert!(locked.is_empty());
    assert!(topic_ids(&engine, "orphan").is_empty());
}

#[tokio::test]
async fn restore_service_task_updates_topic_index() {
    let engine = WorkflowEngine::new();
    let svc_id = uuid::Uuid::new_v4();
    engine.restore_service_task(crate::runtime::PendingServiceTask {
        id: svc_id,
        instance_id: uuid::Uuid::new_v4(),
        definition_key: uuid::Uuid::new_v4(),
        node_id: "svc".into(),
        topic: "restore_topic".into(),
        token_id: uuid::Uuid::new_v4(),
        variables_snapshot: HashMap::new(),
        created_at: chrono::Utc::now(),
        worker_id: None,
        lock_expiration: None,
        retries: 3,
        error_message: None,
        error_details: None,
        business_key: None,
    });

    assert_eq!(topic_ids(&engine, "restore_topic"), vec![svc_id]);
}
