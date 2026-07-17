# persistence — Gotchas

### ⚠️ NATS requires JetStream enabled

The NATS server must be started with `--js` flag: `nats-server --js` (docker-compose uses `nats:alpine` with `--js`). Without JetStream, KV stores and Object Stores are unavailable.

### ⚠️ In-memory fallback is opt-in (dev only)

If NATS is unreachable, `engine-server` behavior depends on **`REQUIRE_NATS`**:
- `false` (default local): start with no persistence + error log (data lost on restart)
- `true` (docker-compose / production): **refuse to start**

`/api/ready` returns **503** when `require_nats` is set but no persistence is attached, or when the backend storage check fails.

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

### ⚠️ Transient NATS failures use a bounded retry queue

The engine's `retry_queue` (not the adapter) handles transient write failures:
1. Inline: 2 attempts with ~50ms backoff
2. Background: bounded channel (default 10 000 jobs); exponential backoff up to 60s, max 50 retries
3. If the queue is **full**, new jobs are **dropped** and counted (`bpmn_persistence_retry_dropped_total`) — prevents OOM under prolonged outage

Integration tests in `persistence-nats/src/tests.rs` cover token, history, definition/instance restore, and user-task roundtrips (skip if NATS is not reachable).

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
