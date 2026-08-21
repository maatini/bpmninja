//! In-memory `WorkflowPersistence` — Reexport der einzigen Implementierung in `engine-core`.

pub use engine_core::adapter::InMemoryPersistence;

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::history::{ActorType, HistoryEntry, HistoryEventType};
    use engine_core::persistence::{HistoryQuery, WorkflowPersistence};
    use engine_core::runtime::{InstanceState, ProcessInstance};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_query_history_filters() {
        let p = InMemoryPersistence::new();
        let inst_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        let make_entry = |event_type, actor_type, node: &str, ts| HistoryEntry {
            id: uuid::Uuid::new_v4(),
            instance_id: inst_id,
            event_type,
            description: "test".into(),
            actor_type,
            actor_id: None,
            node_id: Some(node.into()),
            diff: None,
            timestamp: ts,
            context: HashMap::new(),
            metadata: None,
            definition_version: None,
            is_snapshot: false,
            full_state_snapshot: None,
        };

        let e1 = make_entry(
            HistoryEventType::InstanceStarted,
            ActorType::Engine,
            "start",
            now - chrono::Duration::hours(2),
        );
        let e2 = make_entry(
            HistoryEventType::TaskCompleted,
            ActorType::User,
            "task1",
            now - chrono::Duration::hours(1),
        );
        let e3 = make_entry(
            HistoryEventType::InstanceCompleted,
            ActorType::Engine,
            "end",
            now,
        );

        p.append_history_entry(&e1).await.unwrap();
        p.append_history_entry(&e2).await.unwrap();
        p.append_history_entry(&e3).await.unwrap();

        let all = p
            .query_history(HistoryQuery {
                instance_id: inst_id,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(all.len(), 3);

        let tasks_only = p
            .query_history(HistoryQuery {
                instance_id: inst_id,
                event_types: Some(vec![HistoryEventType::TaskCompleted]),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(tasks_only.len(), 1);

        let page = p
            .query_history(HistoryQuery {
                instance_id: inst_id,
                offset: Some(1),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].event_type, HistoryEventType::TaskCompleted);
    }

    #[tokio::test]
    async fn test_get_storage_info_arithmetic() {
        let p = InMemoryPersistence::new();
        p.save_file("file1", &[0u8; 100]).await.unwrap();
        p.save_file("file2", &[0u8; 200]).await.unwrap();

        let inst = ProcessInstance {
            id: uuid::Uuid::new_v4(),
            definition_key: uuid::Uuid::new_v4(),
            business_key: String::new(),
            parent_instance_id: None,
            state: InstanceState::Running,
            current_node: "x".into(),
            audit_log: vec![],
            variables: HashMap::new(),
            tokens: HashMap::new(),
            active_tokens: vec![],
            join_barriers: HashMap::new(),
            multi_instance_state: HashMap::new(),
            compensation_log: Vec::new(),
            started_at: None,
            completed_at: None,
        };
        p.save_instance(&inst).await.unwrap();

        let info = p.get_storage_info().await.unwrap().unwrap();
        assert_eq!(info.backend_name, "InMemoryPersistence");
        assert_eq!(info.memory_bytes, 2 * 1024 + 512);
    }
}
