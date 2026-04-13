use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::ProcessDefinition;

/// Thread-safe registry for process definitions.
#[derive(Clone, Default)]
pub struct DefinitionRegistry {
    inner: Arc<DashMap<Uuid, Arc<ProcessDefinition>>>,
}

impl DefinitionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, key: Uuid, def: Arc<ProcessDefinition>) {
        self.inner.insert(key, def);
    }

    pub fn get(&self, key: &Uuid) -> Option<Arc<ProcessDefinition>> {
        self.inner
            .get(key)
            .map(|r: dashmap::mapref::one::Ref<'_, Uuid, Arc<ProcessDefinition>>| r.value().clone())
    }

    pub fn remove(&self, key: &Uuid) -> Option<Arc<ProcessDefinition>> {
        self.inner.remove(key).map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: &Uuid) -> bool {
        self.inner.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn list(&self) -> Vec<(Uuid, String, i32, usize)> {
        self.inner
            .iter()
            .map(
                |r: dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    (
                        *r.key(),
                        r.value().id.clone(),
                        r.value().version,
                        r.value().nodes.len(),
                    )
                },
            )
            .collect()
    }

    #[allow(dead_code)]
    pub fn find_by_bpmn_id(&self, bpmn_id: &str) -> Option<(Uuid, Arc<ProcessDefinition>)> {
        self.inner
            .iter()
            .find(
                |r: &dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    r.value().id == bpmn_id
                },
            )
            .map(
                |r: dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    (*r.key(), Arc::clone(r.value()))
                },
            )
    }

    pub fn highest_version(&self, bpmn_id: &str) -> Option<i32> {
        self.inner
            .iter()
            .filter(
                |r: &dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    r.value().id == bpmn_id
                },
            )
            .map(
                |r: dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    r.value().version
                },
            )
            .max()
    }

    /// Returns the definition with the highest version for a given BPMN process ID.
    pub fn find_latest_by_bpmn_id(&self, bpmn_id: &str) -> Option<(Uuid, Arc<ProcessDefinition>)> {
        self.inner
            .iter()
            .filter(
                |r: &dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    r.value().id == bpmn_id
                },
            )
            .max_by_key(
                |r: &dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    r.value().version
                },
            )
            .map(
                |r: dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    (*r.key(), Arc::clone(r.value()))
                },
            )
    }

    /// Returns all versions of a given BPMN process ID, sorted by version ascending.
    pub fn all_versions_of(&self, bpmn_id: &str) -> Vec<(Uuid, Arc<ProcessDefinition>)> {
        let mut versions: Vec<_> = self
            .inner
            .iter()
            .filter(
                |r: &dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    r.value().id == bpmn_id
                },
            )
            .map(
                |r: dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    (*r.key(), Arc::clone(r.value()))
                },
            )
            .collect();
        versions.sort_by_key(|(_, def)| def.version);
        versions
    }

    pub fn all(&self) -> HashMap<Uuid, Arc<ProcessDefinition>> {
        self.inner
            .iter()
            .map(
                |r: dashmap::mapref::multiple::RefMulti<'_, Uuid, Arc<ProcessDefinition>>| {
                    (*r.key(), r.value().clone())
                },
            )
            .collect()
    }
}
