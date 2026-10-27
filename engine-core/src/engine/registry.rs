use crate::domain::ProcessDefinition;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// One lock makes collection publication visible as a single registry change.
#[derive(Clone, Default)]
pub struct DefinitionRegistry {
    inner: Arc<RwLock<HashMap<Uuid, Arc<ProcessDefinition>>>>,
}
impl DefinitionRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&self, key: Uuid, mut def: Arc<ProcessDefinition>) {
        Arc::make_mut(&mut def).compile_conditions();
        self.inner.write().unwrap().insert(key, def);
    }
    pub fn insert_collection(&self, definitions: Vec<ProcessDefinition>) {
        let definitions = definitions
            .into_iter()
            .map(|mut d| {
                d.compile_conditions();
                (d.key, Arc::new(d))
            })
            .collect::<Vec<_>>();
        self.inner.write().unwrap().extend(definitions);
    }
    pub fn get(&self, key: &Uuid) -> Option<Arc<ProcessDefinition>> {
        self.inner.read().unwrap().get(key).cloned()
    }
    pub fn remove(&self, key: &Uuid) -> Option<Arc<ProcessDefinition>> {
        self.inner.write().unwrap().remove(key)
    }
    pub fn contains_key(&self, key: &Uuid) -> bool {
        self.inner.read().unwrap().contains_key(key)
    }
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }
    pub fn list(&self) -> Vec<(Uuid, String, i32, usize)> {
        self.inner
            .read()
            .unwrap()
            .iter()
            .map(|(k, d)| (*k, d.id.clone(), d.version, d.nodes.len()))
            .collect()
    }
    #[allow(dead_code)]
    pub fn find_by_bpmn_id(&self, id: &str) -> Option<(Uuid, Arc<ProcessDefinition>)> {
        self.inner
            .read()
            .unwrap()
            .iter()
            .find(|(_, d)| d.id == id)
            .map(|(k, d)| (*k, d.clone()))
    }
    pub fn highest_version(&self, id: &str) -> Option<i32> {
        self.inner
            .read()
            .unwrap()
            .values()
            .filter(|d| d.id == id)
            .map(|d| d.version)
            .max()
    }
    pub fn find_latest_by_bpmn_id(&self, id: &str) -> Option<(Uuid, Arc<ProcessDefinition>)> {
        self.inner
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d)| d.id == id)
            .max_by_key(|(_, d)| d.version)
            .map(|(k, d)| (*k, d.clone()))
    }
    pub fn all_versions_of(&self, id: &str) -> Vec<(Uuid, Arc<ProcessDefinition>)> {
        let mut found: Vec<_> = self
            .inner
            .read()
            .unwrap()
            .iter()
            .filter(|(_, d)| d.id == id)
            .map(|(k, d)| (*k, d.clone()))
            .collect();
        found.sort_by_key(|(_, d)| d.version);
        found
    }
    pub fn all(&self) -> HashMap<Uuid, Arc<ProcessDefinition>> {
        self.inner.read().unwrap().clone()
    }
}
