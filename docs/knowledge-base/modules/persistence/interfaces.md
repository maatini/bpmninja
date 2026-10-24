# persistence — Interfaces

## Shared Interface: `WorkflowPersistence` trait

Both backends implement this trait (defined in `engine-core/src/port/persistence.rs`). The trait covers:

- **Token CRUD**: `save_token`, `load_tokens`, `delete_token`
- **Instance CRUD**: `save_instance`, `save_instance_json`, `prepare_instance_payload`, `save_instance_payload`, `list_instances`, `delete_instance`
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
pub use engine_core::adapter::InMemoryPersistence;
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

JSON is the default storage format:
- **NATS**: `serde_json::to_vec(value) → bytes::Bytes` stored in KV
- **In-Memory**: `serde_json::to_value(value) → serde_json::Value` stored in HashMap

The engine serializes each instance snapshot once into a size-limited JSON buffer, then calls `prepare_instance_payload(instance, json)` once. It checks the prepared payload size and passes it to `save_instance_payload` for each inline retry. Existing backends inherit defaults that reuse the JSON and delegate to `save_instance_json` / `save_instance`. Serialization and encoding errors are counted as persistence failures and never sent to the backend.

### Optional compact instance encoding (P8)

Build the server with `cargo build --release -p engine-server --features compact-instances` (or enable `persistence-nats/compact-instances` for embedded users). The default remains JSON. The feature changes only active and archived **instance** writes; definitions, tokens, tasks, history events, and files retain their formats.

Compact records use a versioned `\x89BPMNI` envelope with version byte `1`, followed by MessagePack with named fields and human-readable scalar representations (including UUID strings). The payload limit includes the envelope. Named fields preserve compatibility with serde defaults when fields are added. Unknown envelope versions, malformed data, and trailing bytes produce codec errors.

Both feature-enabled and default builds include the decoder and accept mixed legacy JSON and compact records. Turning off compact writes allows rollback within this version: existing compact records still restore and new writes use JSON. Binaries predating this decoder cannot read compact records; upgrade all readers before enabling compact writes. No bulk migration runs, and old records change format only when rewritten.

Instance list/restore, archive lookup/query, and monitoring details decode either format. Monitoring displays ordinary JSON with `encoding: "utf8"`; storage size metrics report actual stored bytes. Compact encoding runs once per snapshot before inline retries. The engine still performs its conservative JSON size guard first, so compact encoding adds CPU work and does not allow instances exceeding the JSON limit. Evaluate storage size and workload latency before enabling it in production.

### Optional separate audit storage (P8)

Enable `--features separate-audit-log` on `engine-server` (or `persistence-nats`); it can be combined with `compact-instances`. Default writes retain inline audit logs. Separate-audit writes use a version-2 MessagePack envelope containing an instance with an empty `audit_log` plus an `audit_key`; audit text lives in the `instance_audit` KV bucket as a JSON array. Connections initialize this bucket in all builds, so disabling separate writes does not disable restore support.

Audit keys are `<instance UUID>.<SHA-256 of audit bytes>`. Each snapshot is immutable. The backend acknowledges the audit write **before** publishing the referring instance document; if the state write fails, the previous document still selects its previous audit. Inline retries reuse the prepared state and audit bytes. A bounded, shared cache (1024 keys) avoids rewriting unchanged audit snapshots. A cache miss uses idempotent create, accepting an existing key only when its contents match. This is a reference-based crash-consistency protocol, not an atomic transaction across KV buckets.

Restore, archived-instance reads, and monitoring hydrate the exact referenced snapshot and verify its digest. A missing, corrupt, invalid, or unsupported audit reference is a persistence error; the engine never silently substitutes an empty audit log. Legacy inline JSON and version-1 compact records remain readable. All readers must support version 2 before detached writes are enabled; readers from before this change reject version-2 records.

Unreferenced snapshots can remain after interrupted state writes. Referenced audit versions remain immutable; unreferenced versions can be pruned using the maintenance method below. Deleting an active instance purges its audit snapshots only if no completed archive exists, preserving archived audit text. Archived audits share the archive's existing retention policy; this step does not add automatic garbage collection. These retained snapshots increase total storage when the log changes frequently; the feature reduces audit data in each state document and skips writes when the audit is unchanged. Delta/checkpoint persistence remains open under P8. The conservative full-JSON size guard still applies, and prepared state plus audit bytes must also fit the payload limit.

### Per-instance audit cleanup (P8, 2026-10-13)

`NatsPersistence::prune_instance_audits(instance_id, dry_run)` plans or applies cleanup for one explicitly selected instance. `AuditPruneReport` lists the current active/archive keys retained, sorted unreferenced candidates, and the number purged. Dry-run never changes audit storage or the cache. Apply rereads references and rescans candidates rather than trusting a previous plan.

```rust
let plan = persistence.prune_instance_audits(instance_id, true).await?;
// After reviewing the plan and stopping external clients:
let result = persistence.prune_instance_audits(instance_id, false).await?;
```

Cleanup verifies both the active and archived documents and their referenced audit digests, then finishes the key scan before deleting anything. Invalid documents, missing/corrupt references, and scan errors abort cleanup. Only keys under the selected instance UUID prefix are candidates. Cache entries are invalidated before purge, so even a lost purge acknowledgement cannot leave a stale successful-write cache entry. If a purge fails partway through, rerunning the operation is safe.

A process-wide async gate excludes cleanup from saves, instance deletion, list/restore hydration, archive reads, and monitoring detail reads through all `NatsPersistence` clones/connections in that process. Instance deletion is also exclusive, preventing local save/audit-purge races. This gate does **not** protect against other processes or raw JetStream access. Application readers and writers using the same storage must be paused during cleanup, including other processes and raw JetStream clients. Outstanding requests (including cancelled requests) must have settled. Keep clients paused after failures or cancellation until pending purges have settled. The local gate is an additional concurrency guard, not a network cancellation barrier. No background scheduler or REST deletion endpoint is introduced; automatic online cleanup awaits distributed ownership/fencing (S1/F4). Existing workflow history events are unaffected; superseded and interrupted audit snapshots that no current instance/archive references may be pruned.

### Optional delta/checkpoint journal (P8, 2026-10-13)

Enable `--features delta-instances` on `engine-server` or `persistence-nats`. This is independent of, and can be combined with, compact encoding and separate audits. JSON/full-document writes remain the default. All builds include the journal reader; upgrade readers before enabling version-3 heads. Turning the feature off writes ordinary JSON/compact/detached records again while existing heads remain readable.

`instances` and `history_instances` store a small version-3 head (`\x89BPMNI\x03` followed by JSON: instance ID, immutable node key, delta depth). `instance_states` stores SHA-256-addressed checkpoint or delta nodes. A checkpoint contains the complete serialized version-1/2/JSON record. A delta retains a common byte prefix and suffix from its parent and stores only the replacement middle. This preserves every field and byte, including Unicode, nested variables, tokens, and audit references. No semantic JSON patch interpretation is involved.

A writer publishes and acknowledges the immutable node before updating the head with NATS KV compare-and-swap. If node creation or head publication fails, the previous head still selects its complete state. Conflicts invalidate the cache and return a persistence error; engine inline retries reread the committed head and reuse already acknowledged nodes. Single-owner deployment is still required; compare-and-swap does not provide distributed instance ownership or resolve application-level stale snapshots.

A checkpoint replaces the chain after eight deltas, or sooner when a delta would be as large as a full checkpoint. Cold restore follows at most nine nodes, verifies each digest/owner and the declared depth, reconstructs within the existing payload limit, then hydrates audit text if needed. Missing or corrupt nodes, excessive depth, malformed splice bounds, and ID mismatches fail restore. A writer cache holds at most 32 full snapshots per connection (shared by clones), bounded by the existing 900 KiB snapshot limit. Process-wide striped locks serialize state saves for an instance across local connections. The conservative full-JSON guard remains in place.

Monitoring renders head records as hydrated instance JSON; raw `instance_states` entries use `encoding: "base64"`. State-node storage is included in bucket metrics. Audit maintenance resolves journal heads before selecting audit references. Active deletion retains journal nodes while an archive exists and purges the instance's nodes only after no archive remains. Interrupted/superseded nodes remain until deletion; online node garbage collection needs ownership/fencing and is not scheduled automatically.

This feature reduces stored/write bytes for localized changes. It adds hashing, comparisons, and extra I/O (particularly during cold restore and cache misses), so throughput/latency improvements must be measured on the intended workload. Tests cover small edits storing nodes below 4 KiB alongside a 16 KiB unchanged variable, checkpoint rollover, interrupted publication, CAS conflicts/idempotent retry, missing/corrupt nodes, cold reconnect, archive preservation, and rollback reads in a default build.
