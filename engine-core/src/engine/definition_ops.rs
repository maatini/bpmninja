use std::sync::Arc;
use uuid::Uuid;

use crate::domain::ProcessDefinition;
use crate::domain::{EngineError, EngineResult};

use super::WorkflowEngine;

impl WorkflowEngine {
    /// Returns a list of all deployed definitions (key, BPMN-ID, version, node count).
    pub async fn list_definitions(&self) -> Vec<(Uuid, String, i32, usize)> {
        self.definitions.list()
    }

    /// Returns a given definition by key
    pub async fn get_definition(&self, key: &Uuid) -> Option<Arc<ProcessDefinition>> {
        self.definitions.get(key)
    }

    /// Returns all versions of a specific BPMN process definition, sorted ascending.
    pub async fn list_definition_versions(&self, bpmn_id: &str) -> Vec<(Uuid, i32, usize)> {
        self.definitions
            .all_versions_of(bpmn_id)
            .into_iter()
            .map(|(key, def)| (key, def.version, def.nodes.len()))
            .collect()
    }

    /// Deploys a process definition so instances can be started from it.
    /// Deployment semantics: if a definition with the same BPMN process ID already
    /// exists, the new definition receives a fresh UUID key and an incremented version.
    /// Existing running instances continue on their original definition untouched.
    /// Returns (definition_key, version).
    pub async fn deploy_definition(&self, definition: ProcessDefinition) -> (Uuid, i32) {
        let _guard = self.deployment_lock.lock().await;
        self.deploy_definition_inner(definition).await
    }

    async fn deploy_definition_inner(&self, definition: ProcessDefinition) -> (Uuid, i32) {
        let highest = self.definitions.highest_version(&definition.id);
        let mut def = Self::normalize_definition(definition, highest);
        // Cloning a deployed definition also clones its key. Preserve the
        // immutable definition used by existing instances on redeployment.
        // Restoring distinct persisted keys into an empty registry keeps them.
        if self.definitions.contains_key(&def.key) {
            def.key = Uuid::new_v4();
        }
        let key = def.key;
        let version = def.version;

        let sub_processes = {
            let mut d = def;
            let subs = std::mem::take(&mut d.sub_processes);
            self.definitions.insert(key, Arc::new(d));
            subs
        };

        for mut sub in sub_processes {
            sub.version = version;
            Box::pin(self.deploy_definition_inner(sub)).await;
        }

        tracing::info!(
            "Deployed definition '{}' (v{}, key: {})",
            self.definitions
                .get(&key)
                .map(|d| d.id.clone())
                .unwrap_or_default(),
            version,
            key
        );
        self.persist_definition(key).await;
        self.emit_event(crate::engine::events::EngineEvent::DefinitionChanged {
            definition_key: key,
        });
        (key, version)
    }

    /// Persist once before atomically publishing all executable pools. A failed
    /// commit is never queued for retry: callers must explicitly retry deployment.
    pub async fn deploy_collection(
        &self,
        processes: Vec<ProcessDefinition>,
        xml: String,
        collaborations: serde_json::Value,
    ) -> EngineResult<crate::DefinitionCollection> {
        let _guard = self.deployment_lock.lock().await;
        if processes.is_empty() {
            return Err(EngineError::InvalidDefinition(
                "Collection has no executable processes".into(),
            ));
        }
        fn flatten(
            engine: &WorkflowEngine,
            mut d: ProcessDefinition,
            out: &mut Vec<ProcessDefinition>,
        ) -> EngineResult<()> {
            if out.iter().any(|previous| previous.id == d.id) {
                return Err(EngineError::InvalidDefinition(format!(
                    "Duplicate deployed process ID '{}'",
                    d.id
                )));
            }
            d.version = engine
                .definitions
                .highest_version(&d.id)
                .map_or(d.version, |v| v + 1);
            d.key = Uuid::new_v4();
            let subs = std::mem::take(&mut d.sub_processes);
            let version = d.version;
            out.push(d);
            for mut sub in subs {
                sub.version = version;
                flatten(engine, sub, out)?;
            }
            Ok(())
        }
        let mut definitions = Vec::new();
        for process in processes {
            flatten(self, process, &mut definitions)?;
        }
        let collection = crate::DefinitionCollection {
            id: Uuid::new_v4(),
            definitions,
            xml,
            collaborations,
            deleted: Vec::new(),
        };
        let routes = super::message_routing::compile_routes(&collection)?;
        if let Some(persistence) = &self.persistence {
            persistence.save_definition_collection(&collection).await?;
        }
        self.install_routes(routes);
        // No await between durable commit and publication, including cancellation.
        self.definitions
            .insert_collection(collection.definitions.clone());
        for definition in &collection.definitions {
            self.emit_event(crate::engine::EngineEvent::DefinitionChanged {
                definition_key: definition.key,
            });
        }
        Ok(collection)
    }

    /// Restore already committed definitions without renumbering or writing them.
    pub fn restore_definitions(&self, definitions: Vec<ProcessDefinition>) {
        self.definitions.insert_collection(definitions);
    }

    /// Startup installs persisted identities and versions without redeploying them.
    pub fn restore_definition_collection(
        &self,
        collection: &crate::DefinitionCollection,
    ) -> EngineResult<()> {
        let routes = super::message_routing::compile_routes(collection)?;
        self.install_routes(routes);
        self.definitions.insert_collection(
            collection
                .definitions
                .iter()
                .filter(|d| !collection.deleted.contains(&d.key))
                .cloned()
                .collect(),
        );
        Ok(())
    }

    /// Assigns version and key to a definition without side effects (pure, testable).
    pub(crate) fn normalize_definition(
        mut definition: ProcessDefinition,
        highest_version: Option<i32>,
    ) -> ProcessDefinition {
        definition.version = highest_version.map(|v| v + 1).unwrap_or(definition.version);
        definition
    }

    /// Deletes a process definition.
    /// If cascade is true, deletes all associated process instances first.
    pub async fn delete_definition(&self, definition_key: Uuid, cascade: bool) -> EngineResult<()> {
        let _guard = self.deployment_lock.lock().await;
        if !self.definitions.contains_key(&definition_key) {
            return Err(EngineError::NoSuchDefinition(definition_key));
        }

        // Check for instances
        let all_insts = self.instances.all().await;
        let mut associated_instances = Vec::new();
        for lk in all_insts.values() {
            if lk.read().await.definition_key == definition_key {
                associated_instances.push(lk.read().await.id);
            }
        }

        if !associated_instances.is_empty() {
            if !cascade {
                return Err(EngineError::DefinitionHasInstances(
                    associated_instances.len(),
                ));
            }
            // Cascade delete instances
            for instance_id in associated_instances {
                self.delete_instance(instance_id).await?;
            }
        }

        if let Some(ref persistence) = self.persistence {
            persistence
                .delete_definition(&definition_key.to_string())
                .await?;
        }

        self.definitions.remove(&definition_key);
        self.emit_event(crate::engine::events::EngineEvent::DefinitionChanged { definition_key });
        Ok(())
    }

    /// Deletes all process definition versions for a given BPMN ID.
    /// If cascade is true, deletes all associated process instances first.
    pub async fn delete_all_definitions(&self, bpmn_id: &str, cascade: bool) -> EngineResult<()> {
        let versions = self.definitions.all_versions_of(bpmn_id);
        for (key, _) in versions {
            self.delete_definition(key, cascade).await?;
        }
        Ok(())
    }
}
