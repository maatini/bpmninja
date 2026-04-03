use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::model::ProcessDefinition;

/// Thread-safe registry for process definitions.
#[derive(Clone, Default)]
pub struct DefinitionRegistry {
    inner: Arc<RwLock<HashMap<Uuid, Arc<ProcessDefinition>>>>,
}

impl DefinitionRegistry {
    pub fn new() -> Self { Self::default() }
    
    pub async fn insert(&self, key: Uuid, def: Arc<ProcessDefinition>) {
        self.inner.write().await.insert(key, def);
    }
    
    pub async fn get(&self, key: &Uuid) -> Option<Arc<ProcessDefinition>> {
        self.inner.read().await.get(key).cloned()
    }
    
    pub async fn remove(&self, key: &Uuid) -> Option<Arc<ProcessDefinition>> {
        self.inner.write().await.remove(key)
    }
    
    pub async fn contains_key(&self, key: &Uuid) -> bool {
        self.inner.read().await.contains_key(key)
    }
    
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
    
    #[allow(dead_code)]
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
    
    pub async fn list(&self) -> Vec<(Uuid, String, usize)> {
        self.inner.read().await.iter()
            .map(|(key, def)| (*key, def.id.clone(), def.nodes.len()))
            .collect()
    }
    
    #[allow(dead_code)]
    pub async fn find_by_bpmn_id(&self, bpmn_id: &str) -> Option<(Uuid, Arc<ProcessDefinition>)> {
        self.inner.read().await.iter()
            .find(|(_, def)| def.id == bpmn_id)
            .map(|(k, v)| (*k, Arc::clone(v)))
    }
    
    pub async fn highest_version(&self, bpmn_id: &str) -> Option<i32> {
        self.inner.read().await.values()
            .filter(|d| d.id == bpmn_id)
            .map(|d| d.version)
            .max()
    }
    
    pub async fn all(&self) -> HashMap<Uuid, Arc<ProcessDefinition>> {
        self.inner.read().await.clone()
    }
}
