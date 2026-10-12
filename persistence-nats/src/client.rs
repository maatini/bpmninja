use std::collections::HashMap;
use std::sync::Arc;

use async_nats::Client;
use async_nats::jetstream::object_store::{Config as ObjectStoreConfig, ObjectStore};
use async_nats::jetstream::{self, context::Context, kv, stream::Config as StreamConfig};
use engine_core::error::{EngineError, EngineResult};
use futures::StreamExt;
use tokio::sync::OnceCell;

/// KV buckets created on connect; their handles are cached after first use.
const KV_BUCKETS: [&str; 9] = [
    "tokens",
    "instances",
    "definitions",
    "user_tasks",
    "service_tasks",
    "timers",
    "messages",
    "history_instances",
    "instance_audit",
];

/// Object stores created on connect; their handles are cached after first use.
const OBJECT_STORES: [&str; 2] = ["bpmn_xml", "instance_files"];

/// Cached bucket handles. Without the cache every operation paid an extra
/// JetStream API round trip (`get_key_value` / `get_object_store`) before the
/// actual read or write. A failed lookup is not cached, so the next call retries.
pub(crate) struct HandleCache {
    kv: HashMap<&'static str, OnceCell<kv::Store>>,
    object_stores: HashMap<&'static str, OnceCell<ObjectStore>>,
}

impl HandleCache {
    fn new() -> Self {
        Self {
            kv: KV_BUCKETS.iter().map(|b| (*b, OnceCell::new())).collect(),
            object_stores: OBJECT_STORES
                .iter()
                .map(|b| (*b, OnceCell::new()))
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct NatsPersistence {
    pub(crate) client: Client,
    pub(crate) js: Context,
    pub(crate) handles: Arc<HandleCache>,
    pub(crate) audit_cache: Arc<tokio::sync::Mutex<std::collections::HashSet<String>>>,
}

impl NatsPersistence {
    pub async fn connect(url: &str, stream_name: &str) -> EngineResult<Self> {
        let client = async_nats::connect(url).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to connect to NATS: {}", e))
        })?;

        let js = jetstream::new(client.clone());

        // Optional: Ensure the stream exists.
        // We ignore the error if it already exists.
        let _ = js
            .get_or_create_stream(StreamConfig {
                name: stream_name.to_string(),
                subjects: vec![format!("{}.*", stream_name)],
                ..Default::default()
            })
            .await;

        // Ensure the bpmn_xml Object Store bucket exists (per NATS rules).
        let _ = js
            .create_object_store(ObjectStoreConfig {
                bucket: "bpmn_xml".to_string(),
                description: Some("Original BPMN 2.0 XML artifacts".to_string()),
                ..Default::default()
            })
            .await;

        // Ensure the instance_files Object Store bucket exists.
        let _ = js
            .create_object_store(ObjectStoreConfig {
                bucket: "instance_files".to_string(),
                description: Some(
                    "Binary file attachments for process instance variables".to_string(),
                ),
                ..Default::default()
            })
            .await;

        // Ensure the tokens KV bucket exists.
        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "tokens".to_string(),
                description: "Token state per process instance (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        // Ensure the instances KV bucket exists (per NATS rules).
        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "instances".to_string(),
                description: "ProcessInstance state (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        // Ensure the definitions KV bucket exists (per NATS rules).
        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "definitions".to_string(),
                description: "ProcessDefinition metadata (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        // Ensure the user_tasks KV bucket exists (per NATS rules).
        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "user_tasks".to_string(),
                description: "PendingUserTask objects (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        // Ensure the service_tasks KV bucket exists.
        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "service_tasks".to_string(),
                description: "PendingServiceTask objects (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "timers".to_string(),
                description: "PendingTimer objects (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "messages".to_string(),
                description: "PendingMessageCatch objects (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        // Ensure the history_instances KV bucket exists for archived completed instances.
        let _ = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: "history_instances".to_string(),
                description: "Archived completed ProcessInstance objects (JSON)".to_string(),
                ..Default::default()
            })
            .await;

        // Ensure the WORKFLOW_HISTORY JetStream stream exists for event-sourcing.
        let _ = js
            .get_or_create_stream(StreamConfig {
                name: "WORKFLOW_HISTORY".to_string(),
                subjects: vec!["history.instance.*".to_string()],
                ..Default::default()
            })
            .await;

        // Readers always need this bucket, including after disabling detached writes.
        if js.get_key_value("instance_audit").await.is_err() {
            if let Err(create_error) = js
                .create_key_value(async_nats::jetstream::kv::Config {
                    bucket: "instance_audit".into(),
                    description: "Immutable instance audit snapshots".into(),
                    ..Default::default()
                })
                .await
            {
                // A concurrent connection may have created the same bucket.
                js.get_key_value("instance_audit").await.map_err(|_| {
                    EngineError::PersistenceError(format!(
                        "Failed to initialize audit bucket: {create_error}"
                    ))
                })?;
            }
        }

        Ok(Self {
            client,
            js,
            handles: Arc::new(HandleCache::new()),
            audit_cache: Arc::new(tokio::sync::Mutex::new(std::collections::HashSet::new())),
        })
    }

    /// Returns the KV handle for `bucket` (cached for known buckets).
    pub(crate) async fn kv(&self, bucket: &str) -> EngineResult<kv::Store> {
        let lookup = || async {
            self.js.get_key_value(bucket).await.map_err(|e| {
                EngineError::PersistenceError(format!("Failed to get {bucket} KV: {e}"))
            })
        };
        match self.handles.kv.get(bucket) {
            Some(cell) => cell.get_or_try_init(lookup).await.cloned(),
            None => lookup().await,
        }
    }

    /// Returns the object store handle for `bucket` (cached for known stores).
    pub(crate) async fn object_store(&self, bucket: &str) -> EngineResult<ObjectStore> {
        let lookup = || async {
            self.js.get_object_store(bucket).await.map_err(|e| {
                EngineError::PersistenceError(format!("Failed to get {bucket} Object Store: {e}"))
            })
        };
        match self.handles.object_stores.get(bucket) {
            Some(cell) => cell.get_or_try_init(lookup).await.cloned(),
            None => lookup().await,
        }
    }

    /// Gibt einen Klon des JetStream-Kontexts zurück, z. B. für den Log-Sink.
    pub fn jetstream(&self) -> async_nats::jetstream::context::Context {
        self.js.clone()
    }

    pub(crate) async fn list_kv_entries<T: serde::de::DeserializeOwned>(
        &self,
        bucket: &str,
        entity_name: &str,
    ) -> EngineResult<Vec<T>> {
        let _audit_access = crate::audit::AUDIT_ACCESS.read().await;
        let store = self.kv(bucket).await?;

        // keys() Err is a persistence failure, not an empty bucket — must propagate.
        let mut keys = store.keys().await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to list {entity_name} keys: {}", e))
        })?;

        let mut entries = Vec::new();
        while let Some(key_result) = keys.next().await {
            match key_result {
                Ok(key) => match store.get(&key).await {
                    Ok(Some(entry)) => match if crate::codec::detached(&entry) {
                        let restored = self.restore_instance_record(&entry).await?;
                        serde_json::to_value(restored)
                            .and_then(serde_json::from_value::<T>)
                            .map_err(|e| EngineError::PersistenceError(e.to_string()))
                    } else {
                        crate::codec::decode::<T>(&entry)
                    } {
                        Ok(item) => entries.push(item),
                        // A recognized compact record must not disappear from
                        // restore on corruption or an unsupported version.
                        Err(e) if crate::codec::is_compact(&entry) => return Err(e),
                        Err(e) => {
                            tracing::warn!("Failed to deserialize {entity_name} '{}': {}", key, e)
                        }
                    },
                    Ok(None) => {}
                    Err(e) => tracing::warn!("Failed to get {entity_name} '{key}': {}", e),
                },
                Err(e) => tracing::warn!("Failed to stream {entity_name} key: {}", e),
            }
        }

        Ok(entries)
    }
}
