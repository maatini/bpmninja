# persistence — Interfaces

## Shared Interface: `WorkflowPersistence` trait

Both backends implement this trait (defined in `engine-core/src/port/persistence.rs`). The trait covers:

- **Token CRUD**: `save_token`, `load_tokens`, `delete_token`
- **Instance CRUD**: `save_instance`, `list_instances`, `delete_instance`
- **Definition CRUD**: `save_definition`, `list_definitions`, `delete_definition`
- **Task Queues**: `save_user_task`, `delete_user_task`, `list_user_tasks`; same for service tasks
- **Wait States**: `save_timer`, `delete_timer`, `list_timers`; same for message catches
- **File Storage**: `save_file`, `load_file`, `delete_file` (Object Store)
- **BPMN XML**: `save_bpmn_xml`, `load_bpmn_xml`, `list_bpmn_xml_ids`
- **History**: `append_history_entry`, `query_history`
- **Archive**: `save_completed_instance`, `query_completed_instances`, `get_completed_instance`
- **Monitoring**: `get_storage_info`, `get_bucket_entries`, `get_bucket_entry_detail`

## persistence-nats Public API

```rust
// client.rs
pub struct NatsPersistence {
    // internal fields: jetstream context, KV store handles
}

impl NatsPersistence {
    /// Connect to a NATS server and initialize JetStream resources.
    pub async fn connect(nats_url: &str, stream_name: &str) -> Result<Self, EngineError>;
    
    /// Access the JetStream context (for LogSink, etc.)
    pub fn jetstream(&self) -> &async_nats::jetstream::Context;
}

// models.rs
pub struct NatsInfo {
    // NATS server metadata
}

// trait_impl.rs
impl WorkflowPersistence for NatsPersistence { ... }
```

## persistence-memory Public API

```rust
// lib.rs
pub struct InMemoryPersistence {
    // HashMap fields for each entity type
}

impl InMemoryPersistence {
    pub fn new() -> Self;
}

impl WorkflowPersistence for InMemoryPersistence { ... }
```

## Monitoring Data Types (from engine-core port)

```rust
pub struct StorageInfo {
    pub backend_name: String,
    pub version: String,
    pub host: String,
    pub port: u16,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub streams: usize,
    pub consumers: usize,
    pub buckets: Vec<BucketInfo>,
}

pub struct BucketInfo {
    pub name: String,
    pub bucket_type: String,      // "kv", "object_store", "stream"
    pub entries: u64,
    pub size_bytes: u64,
}

pub struct BucketEntry {
    pub key: String,
    pub size_bytes: Option<u64>,
    pub created_at: Option<DateTime<Utc>>,
}

pub struct BucketEntryDetail {
    pub key: String,
    pub data: String,             // JSON, XML, or base64-encoded binary
    pub encoding: String,         // "utf8" or "base64"
}
```

## Serialization Format

All domain types are serialized to JSON via `serde_json`:
- **NATS**: `serde_json::to_vec(value) → bytes::Bytes` stored in KV
- **In-Memory**: `serde_json::to_value(value) → serde_json::Value` stored in HashMap
