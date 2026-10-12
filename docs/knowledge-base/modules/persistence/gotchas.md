# persistence — Gotchas

### ⚠️ NATS requires JetStream enabled

The NATS server must be started with `--js` flag: `nats-server --js` (docker-compose uses `nats:alpine` with `--js --sd /data` plus user/pass). Without JetStream, KV stores and Object Stores are unavailable.

### ⚠️ docker-compose: NATS auth + loopback host ports

`docker-compose.yaml` does **not** publish NATS or cobra-nats on all interfaces:

| Service | Host bind | Purpose |
|---------|-----------|---------|
| `nats` 4222 | `127.0.0.1:4222` | Client protocol (loopback only) |
| `nats` 8222 | `127.0.0.1:8222` | HTTP monitoring (loopback only) |
| `cobra-nats` 3000 | `127.0.0.1:3000` | Dashboard (loopback only) |
| `engine-server` 8081 | `8081:8081` | LAN API (all interfaces) |

NATS requires username/password (local default, **changeable** in compose): user `bpmninja`, pass `bpmninja` via `--user` / `--pass`.

Connection URLs:

- Inside Compose: `nats://bpmninja:bpmninja@nats:4222` (`engine-server` `NATS_URL`; `async-nats` accepts credentials in the URL)
- From the host (tests, CLI, desktop on the same machine): `nats://bpmninja:bpmninja@localhost:4222`

Unauthenticated `nats://localhost:4222` will fail against this stack. cobra-nats may still need the same user/pass in its UI if it ignores `NATS_URL`.

### ⚠️ In-memory fallback is opt-in (dev only)

If NATS is unreachable, `engine-server` behavior depends on **`REQUIRE_NATS`**:
- `true` (default, docker-compose / production): **refuse to start**
- `false` (explicit local opt-in): start with no persistence + error log (data lost on restart)

`/api/ready` returns **503** when `require_nats` is set but no persistence is attached, or when the backend storage check fails.

### ⚠️ Bucket handles are cached

`NatsPersistence` caches KV / object-store handles (`HandleCache`, one `OnceCell` per bucket created in `connect`). Use `self.kv("bucket")` / `self.object_store("bucket")` in `trait_impl.rs` — calling `self.js.get_key_value(..)` per operation adds a JetStream API round trip. Unknown bucket names (monitoring) fall through to an uncached lookup. A failed lookup is not cached.

### ⚠️ KV bucket names cannot change after creation

NATS bucket names are hardcoded in `client.rs` / `trait_impl.rs`. Changing a bucket name requires deleting the old bucket and its data. Current KV buckets: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `messages`, `tokens`, `history_instances`. Object stores: `bpmn_xml`, `instance_files`. See [dependencies.md](dependencies.md) for the full map.

### ⚠️ Token keys include instance ID for scoping

Token KV keys use `token-{instance_id}-{token_id}` format. The `load_tokens` method uses key prefix scanning (`store.keys()` with prefix filter) to load only tokens for a specific instance.

### ⚠️ In-memory persistence loses data on restart

`persistence-memory` stores everything in `HashMap` and `Vec`. Server restart = blank slate. The engine logs a warning when starting in this mode.

### ⚠️ Completed instances are archived, not deleted

When an instance reaches `Completed` state, it's saved to the `history_instances` KV bucket via `save_completed_instance`, then deleted from the active `instances` bucket. The archived copy has a different storage location.

### ⚠️ Object Store file naming convention

File object keys follow: `file:{instance_id}-{var_name}-{filename}`. The separator is `-`, not `/`. Filename can contain special characters but is URL-encoded when exposed via REST.

### ⚠️ History entries are append-only

History entries are published to the JetStream stream `WORKFLOW_HISTORY` (subject `history.instance.{instance_id}`). `query_history` creates an ephemeral pull consumer filtered on that subject and replays all entries for the instance — cost grows with history length.

### ⚠️ Transient NATS failures use a bounded retry queue

The engine's `retry_queue` (not the adapter) handles transient write failures:
1. Inline: 2 attempts with ~50ms backoff
2. Background: bounded channel (default 10 000 jobs); exponential backoff up to 60s, max 50 retries
3. If the queue is **full**, new jobs are **dropped** and counted (`bpmn_persistence_retry_dropped_total`) — prevents OOM under prolonged outage

Integration tests in `persistence-nats/src/tests.rs` cover token, history, definition/instance restore, and user-task roundtrips. Locally they skip if NATS is unreachable; in CI (`CI=true`) they fail instead. GitHub Actions starts NATS with JetStream before `cargo test --workspace`.

### ⚠️ KV `keys()` errors must propagate

`store.keys().await` returning `Err` is a persistence failure, not an empty bucket. `list_kv_entries` (and `load_tokens`) must use `?` after mapping the error. Swallowing it as `Ok(vec![])` makes restore look like “no data” and silently skips recovery.

### ⚠️ BPMN XML is stored separately from definitions

`ProcessDefinition` (JSON, KV `definitions`) and the original BPMN XML (object store `bpmn_xml`) are stored separately. The XML is needed for the desktop UI's bpmn-js modeler and for redeployment. They share the same UUID key.

Startup restore is **JSON-first**: `list_definitions()` deploys stored `ProcessDefinition` objects without re-parsing XML. XML is loaded into `deployed_xml` for the modeler (match by definition UUID). Missing XML is a warning; the definition is still restored. XML-parse fallback runs only when the definitions KV is empty or `list_definitions` returns a persistence error. RestoreStats count actually deployed definitions, never a silent zero that hides a KV failure.

### ⚠️ Adding a new entity type

To add a new persistent entity:
1. Add the trait method to `WorkflowPersistence` in `engine-core/src/port/persistence.rs`
2. Implement in `persistence-nats/src/trait_impl.rs` (new KV bucket + CRUD)
3. Implement in `engine-core/src/adapter/in_memory.rs` (new HashMap + CRUD); `persistence-memory` re-exports that type
4. Add a `PersistJob` variant in `engine-core/src/engine/retry_queue.rs`
5. Add restoration in `engine-server/src/startup.rs`
6. Update the `RestoreStats` struct
