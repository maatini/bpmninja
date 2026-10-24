# persistence — Responsibilities

## Summary

Both backends implement `engine_core::port::WorkflowPersistence` — the hexagonal port for engine state storage. The engine calls persistence operations through this trait, never directly.

## persistence-nats (Production)

**Crate:** `persistence-nats/`  
**Source:** 5 files: `lib.rs`, `client.rs`, `trait_impl.rs`, `models.rs`, `tests.rs` (~1,440 LoC)

**Owns:**
1. NATS JetStream connection lifecycle (`NatsPersistence::connect(url, stream_name)`)
2. 8 KV buckets: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `messages`, `tokens`, `history_instances`
3. Object Stores: `bpmn_xml` (BPMN source), `instance_files` (binary file variables)
4. Streams: `WORKFLOW_HISTORY` (history entries, subject `history.instance.{id}`), `WORKFLOW_EVENTS`
5. `ENGINE_LOGS` stream for persistent log buffer backup (created by `engine-server`)
6. Full `WorkflowPersistence` trait implementation (35 async methods)

**Invariants:**
- KV keys are the bare entity UUID; only tokens use `token-{instance_id}-{token_id}`
- File object keys: `file:{instance_id}-{var_name}-{filename}`
- All values are JSON-serialized (except file data, which is raw bytes)
- Failed operations return `EngineError::PersistenceError(msg)`

## persistence-memory (Testing/Development)

**Crate:** `persistence-memory/`  
**Source:** `lib.rs` re-exports `engine_core::adapter::InMemoryPersistence`

**Owns:**
1. The public crate name used by `engine-server`, fuzz, and integration tests
2. No separate HashMap implementation — the type lives in `engine-core`

**Invariants:**
- Data is NOT persisted to disk — lost on restart
- Used in unit/integration tests (instant setup, no external dependencies)
- May be used by the server only when NATS is down **and** `REQUIRE_NATS` is false (dev)

## Key Entry Points

| Backend | Entry Point | Returns |
|---------|------------|---------|
| NATS | `NatsPersistence::connect(nats_url: &str, stream_name: &str) -> Result<Self>` | Connected NATS persistence |
| In-Memory | `InMemoryPersistence::new() -> Self` | Empty in-memory persistence |
