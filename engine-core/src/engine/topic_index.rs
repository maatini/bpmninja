//! Secondary topic index for pending service tasks.
//!
//! SSOT remains `pending_service_tasks`. Insert into the task map first, then
//! the index, so `fetch_and_lock` cannot treat a not-yet-inserted task as an
//! orphan and drop the index entry.

use std::collections::HashSet;

use dashmap::DashMap;
use uuid::Uuid;

use super::WorkflowEngine;
use crate::runtime::PendingServiceTask;

pub(crate) type TopicIndex = DashMap<String, HashSet<Uuid>>;

pub(crate) fn insert(index: &TopicIndex, topic: &str, id: Uuid) {
    index.entry(topic.to_string()).or_default().insert(id);
}

pub(crate) fn remove(index: &TopicIndex, topic: &str, id: Uuid) {
    let mut drop_topic = false;
    if let Some(mut set) = index.get_mut(topic) {
        set.remove(&id);
        drop_topic = set.is_empty();
    }
    if drop_topic {
        index.remove_if(topic, |_, set| set.is_empty());
    }
}

/// Task IDs for the requested topics, paired with the topic they were found under.
pub(crate) fn candidate_ids(index: &TopicIndex, topics: &[String]) -> Vec<(String, Uuid)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for topic in topics {
        if let Some(set) = index.get(topic) {
            for id in set.iter() {
                if seen.insert(*id) {
                    out.push((topic.clone(), *id));
                }
            }
        }
    }
    out
}

impl WorkflowEngine {
    pub(crate) fn insert_pending_service_task(&self, task: PendingServiceTask) {
        let id = task.id;
        let topic = task.topic.clone();
        self.pending_service_tasks.insert(id, task);
        insert(&self.service_task_topic_index, &topic, id);
    }

    pub(crate) fn remove_pending_service_task(
        &self,
        task_id: &Uuid,
    ) -> Option<(Uuid, PendingServiceTask)> {
        let removed = self.pending_service_tasks.remove(task_id);
        if let Some((_, ref task)) = removed {
            remove(&self.service_task_topic_index, &task.topic, task.id);
        }
        removed
    }

    pub(crate) fn remove_pending_service_task_if(
        &self,
        task_id: &Uuid,
        f: impl FnOnce(&Uuid, &PendingServiceTask) -> bool,
    ) -> Option<(Uuid, PendingServiceTask)> {
        let removed = self.pending_service_tasks.remove_if(task_id, f);
        if let Some((_, ref task)) = removed {
            remove(&self.service_task_topic_index, &task.topic, task.id);
        }
        removed
    }

    pub(crate) fn clear_pending_service_tasks_for_instance(&self, instance_id: Uuid) {
        let stale: Vec<(String, Uuid)> = self
            .pending_service_tasks
            .iter()
            .filter(|r| r.instance_id == instance_id)
            .map(|r| (r.topic.clone(), r.id))
            .collect();
        self.pending_service_tasks
            .retain(|_, t| t.instance_id != instance_id);
        for (topic, id) in stale {
            remove(&self.service_task_topic_index, &topic, id);
        }
    }
}
