# persistence — Gotchas

### ⚠️ NATS requires JetStream enabled

The NATS server must be started with `--js` flag: `nats-server --js`. Without JetStream, KV stores and Object Stores are unavailable. The engine falls back to in-memory mode with a warning.

### ⚠️ KV bucket names cannot change after creation

NATS KV bucket names are hardcoded in `trait_impl.rs`. Changing a bucket name requires deleting the old bucket and its data. The current buckets: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history`.

### ⚠️ Token keys include instance ID for scoping

Token KV keys use `token-{instance_id}-{token_id}` format. The `load_tokens` method uses key prefix scanning (`store.keys()` with prefix filter) to load only tokens for a specific instance.

### ⚠️ In-memory persistence loses data on restart

`persistence-memory` stores everything in `HashMap` and `Vec`. Server restart = blank slate. The engine logs a warning when starting in this mode.

### ⚠️ Completed instances are archived, not deleted

When an instance reaches `Completed` state, it's saved to the `history_completed_instances` bucket via `save_completed_instance`, then deleted from the active `instances` bucket. The archived copy has a different storage location.

### ⚠️ Object Store file naming convention

File object keys follow: `file:{instance_id}-{var_name}-{filename}`. The separator is `-`, not `/`. Filename can contain special characters but is URL-encoded when exposed via REST.

### ⚠️ History entries are append-only

History is stored as a KV bucket (not a stream, despite the `WORKFLOW_EVENTS` stream name). Each entry is a separate KV key. Querying history scans all entries for the instance.

### ⚠️ NATS connection failure is handled with retry

The engine's `retry_queue` handles transient NATS failures. Not the persistence layer itself. If NATS is down, persistence operations queue in the background worker and retry with exponential backoff.

### ⚠️ BPMN XML is stored separately from definitions

`ProcessDefinition` (JSON) and the original BPMN XML (string) are stored in separate KV buckets (`definitions` vs `bpmn_xml`). The XML is needed for the desktop UI's bpmn-js modeler and for redeployment. They share the same UUID key.

### ⚠️ Adding a new entity type

To add a new persistent entity:
1. Add the trait method to `WorkflowPersistence` in `engine-core/src/port/persistence.rs`
2. Implement in `persistence-nats/src/trait_impl.rs` (new KV bucket + CRUD)
3. Implement in `persistence-memory/src/lib.rs` (new HashMap + CRUD)
4. Add a `PersistJob` variant in `engine-core/src/engine/retry_queue.rs`
5. Add restoration in `engine-server/src/startup.rs`
6. Update the `RestoreStats` struct
