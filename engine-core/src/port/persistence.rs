use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

use crate::domain::EngineResult;
use crate::domain::{ProcessDefinition, Token};
use crate::runtime::{PendingServiceTask, PendingUserTask, ProcessInstance};

/// Query filter for searching archived (completed) process instances.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletedInstanceQuery {
    pub definition_key: Option<uuid::Uuid>,
    pub business_key: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    /// Filter by terminal state: "completed", "error", or None (both).
    pub state_filter: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryQuery {
    pub instance_id: uuid::Uuid,
    pub event_types: Option<Vec<crate::history::HistoryEventType>>,
    pub node_id: Option<String>,
    pub actor_type: Option<crate::history::ActorType>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Represents a single entry inside a bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketEntry {
    pub key: String,
    pub size_bytes: Option<u64>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Represents the raw detail of a bucket entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketEntryDetail {
    pub key: String,
    pub data: String, // String representation (JSON or text) or Base64 if binary
    /// Transport encoding of `data`: `"utf8"` for plain text/JSON/XML, `"base64"` for binary files.
    #[serde(default = "BucketEntryDetail::default_encoding")]
    pub encoding: String,
}

impl BucketEntryDetail {
    fn default_encoding() -> String {
        "utf8".to_string()
    }
}

/// Per-bucket storage details for monitoring dashboards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    /// Display name of the bucket (e.g. "instances", "bpmn_xml").
    pub name: String,
    /// Type of storage: "kv", "object_store", or "stream".
    pub bucket_type: String,
    /// Number of entries/messages in the bucket.
    pub entries: u64,
    /// Total size in bytes consumed by this bucket.
    pub size_bytes: u64,
}

/// Generic storage backend information for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub backend_name: String,
    pub version: String,
    pub host: String,
    pub port: u16,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub streams: usize,
    pub consumers: usize,
    /// Per-bucket breakdown of stored data.
    #[serde(default)]
    pub buckets: Vec<BucketInfo>,
}

/// A trait for persisting workflow engine state.
#[async_trait]
pub trait WorkflowPersistence: Send + Sync {
    /// Atomically create or return the existing immutable evaluation identity/result.
    async fn reserve_conditional_start(
        &self,
        _receipt: &crate::ConditionalStartReceipt,
    ) -> EngineResult<crate::ConditionalStartReceipt> {
        Err(crate::EngineError::PersistenceError(
            "Conditional-start receipts unsupported by backend".into(),
        ))
    }
    async fn get_conditional_start(
        &self,
        _id: uuid::Uuid,
    ) -> EngineResult<Option<crate::ConditionalStartReceipt>> {
        Err(crate::EngineError::PersistenceError(
            "Conditional-start receipts unsupported by backend".into(),
        ))
    }
    async fn list_conditional_starts(&self) -> EngineResult<Vec<crate::ConditionalStartReceipt>> {
        Ok(Vec::new())
    }
    async fn complete_conditional_start(
        &self,
        _receipt: &crate::ConditionalStartReceipt,
    ) -> EngineResult<()> {
        Err(crate::EngineError::PersistenceError(
            "Conditional-start receipts unsupported by backend".into(),
        ))
    }

    /// Save a token for a specific process instance.
    async fn save_token(&self, instance_id: uuid::Uuid, token: &Token) -> EngineResult<()>;
    /// Load all tokens belonging to a specific process instance.
    async fn load_tokens(&self, instance_id: uuid::Uuid) -> EngineResult<Vec<Token>>;
    /// Delete a single token of a process instance.
    async fn delete_token(&self, instance_id: uuid::Uuid, token_id: uuid::Uuid)
    -> EngineResult<()>;

    /// Persist the current state of a process instance.
    async fn save_instance(&self, instance: &ProcessInstance) -> EngineResult<()>;
    /// Persist an instance using its already serialized JSON snapshot.
    ///
    /// `json` must be the serialization of `instance`. The engine checks this
    /// payload's size before calling and reuses it for inline retries. Backends
    /// storing JSON can override this to avoid serializing the snapshot again.
    /// The default preserves compatibility with existing backends.
    async fn save_instance_json(
        &self,
        instance: &ProcessInstance,
        _json: &[u8],
    ) -> EngineResult<()> {
        self.save_instance(instance).await
    }
    /// Prepare backend-specific bytes once, before inline retries. The input
    /// JSON has already passed the engine size guard. Alternative encoders must
    /// also enforce the limit on their resulting payload.
    fn prepare_instance_payload(
        &self,
        _instance: &ProcessInstance,
        json: Vec<u8>,
    ) -> EngineResult<Vec<u8>> {
        Ok(json)
    }
    /// Save bytes returned by `prepare_instance_payload` for this snapshot.
    /// Backends overriding preparation must also override this method.
    async fn save_instance_payload(
        &self,
        instance: &ProcessInstance,
        payload: &[u8],
    ) -> EngineResult<()> {
        self.save_instance_json(instance, payload).await
    }
    /// Load all persisted process instances.
    async fn list_instances(&self) -> EngineResult<Vec<ProcessInstance>>;

    /// Delete a process instance.
    async fn delete_instance(&self, id: &str) -> EngineResult<()>;

    /// Persist a process definition metadata (JSON).
    async fn save_definition(&self, definition: &ProcessDefinition) -> EngineResult<()>;
    /// Load all persisted process definitions.
    async fn list_definitions(&self) -> EngineResult<Vec<ProcessDefinition>>;

    /// Delete a process definition.
    async fn delete_definition(&self, key: &str) -> EngineResult<()>;

    /// Persist a pending user task.
    async fn save_user_task(&self, task: &PendingUserTask) -> EngineResult<()>;
    /// Delete a pending user task (e.g. when completed).
    async fn delete_user_task(&self, task_id: uuid::Uuid) -> EngineResult<()>;
    /// Load all persisted pending user tasks.
    async fn list_user_tasks(&self) -> EngineResult<Vec<PendingUserTask>>;

    /// Persist an service task.
    async fn save_service_task(&self, task: &PendingServiceTask) -> EngineResult<()>;
    /// Delete an service task.
    async fn delete_service_task(&self, task_id: uuid::Uuid) -> EngineResult<()>;
    /// Load all persisted service tasks.
    async fn list_service_tasks(&self) -> EngineResult<Vec<PendingServiceTask>>;

    /// Persist a pending timer.
    async fn save_timer(&self, timer: &crate::runtime::PendingTimer) -> EngineResult<()>;
    /// Delete a pending timer.
    async fn delete_timer(&self, timer_id: uuid::Uuid) -> EngineResult<()>;
    /// Load all persisted pending timers.
    async fn list_timers(&self) -> EngineResult<Vec<crate::runtime::PendingTimer>>;

    /// Persist a pending message catch.
    async fn save_message_catch(
        &self,
        catch: &crate::runtime::PendingMessageCatch,
    ) -> EngineResult<()>;
    /// Delete a pending message catch.
    async fn delete_message_catch(&self, catch_id: uuid::Uuid) -> EngineResult<()>;
    /// Load all persisted pending message catches.
    async fn list_message_catches(&self) -> EngineResult<Vec<crate::runtime::PendingMessageCatch>>;

    /// Store a file in the instance_files Object Store.
    async fn save_file(&self, object_key: &str, data: &[u8]) -> EngineResult<()>;
    /// Load a file from the instance_files Object Store.
    async fn load_file(&self, object_key: &str) -> EngineResult<Vec<u8>>;
    /// Delete a file from the instance_files Object Store.
    async fn delete_file(&self, object_key: &str) -> EngineResult<()>;

    /// Store original BPMN 2.0 XML for a definition.
    async fn save_bpmn_xml(&self, definition_key: &str, xml: &str) -> EngineResult<()>;
    /// Load original BPMN 2.0 XML for a definition.
    async fn load_bpmn_xml(&self, definition_key: &str) -> EngineResult<String>;
    /// List all stored BPMN XML definition keys.
    async fn list_bpmn_xml_ids(&self) -> EngineResult<Vec<String>>;

    /// Returns storage backend information (name, version, etc.).
    /// Returns None if the backend doesn't support reporting.
    async fn get_storage_info(&self) -> EngineResult<Option<StorageInfo>>;

    /// Append a new history entry to the instance history log.
    async fn append_history_entry(&self, entry: &crate::history::HistoryEntry) -> EngineResult<()>;
    /// Append several history entries in order (one engine batch).
    ///
    /// Default: one `append_history_entry` call per entry. Backends that can
    /// pipeline writes should override it. Retries may resend entries that were
    /// already stored, so implementations must deduplicate by `HistoryEntry::id`.
    async fn append_history_entries(
        &self,
        entries: &[crate::history::HistoryEntry],
    ) -> EngineResult<()> {
        for entry in entries {
            self.append_history_entry(entry).await?;
        }
        Ok(())
    }
    /// Retrieve all history entries for a specific instance, ordered by time.
    async fn query_history(
        &self,
        query: HistoryQuery,
    ) -> EngineResult<Vec<crate::history::HistoryEntry>>;

    /// Archive a completed process instance to the history store.
    async fn save_completed_instance(&self, instance: &ProcessInstance) -> EngineResult<()>;

    /// Query archived (completed) process instances with filters and pagination.
    async fn query_completed_instances(
        &self,
        query: CompletedInstanceQuery,
    ) -> EngineResult<Vec<ProcessInstance>>;

    /// Load a single archived instance by ID.
    async fn get_completed_instance(&self, id: &str) -> EngineResult<Option<ProcessInstance>>;

    /// Retrieve list of entries inside a specific bucket for monitoring details.
    async fn get_bucket_entries(
        &self,
        bucket_name: &str,
        offset: usize,
        limit: usize,
    ) -> EngineResult<Vec<BucketEntry>>;

    /// Retrieve raw detail string (JSON or base64) of a specific entry in a bucket.
    async fn get_bucket_entry_detail(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> EngineResult<BucketEntryDetail>;
}
