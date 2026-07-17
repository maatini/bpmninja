# persistence — Responsibilities

## Summary

Both backends implement `engine_core::port::WorkflowPersistence` — the hexagonal port for engine state storage. The engine calls persistence operations through this trait, never directly.

## persistence-nats (Production)

**Crate:** `persistence-nats/`  
**Source:** 5 files: `lib.rs`, `client.rs`, `trait_impl.rs`, `models.rs`, `tests.rs` (~1,150 LoC)

**Owns:**
1. NATS JetStream connection lifecycle (`NatsPersistence::connect(url, stream_name)`)
2. 9 KV buckets: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history`
3. Object Store: `instance_files` for binary file variables
4. JetStream: `WORKFLOW_EVENTS` consumer for history event streaming
5. `ENGINE_LOGS` stream for persistent log buffer backup
6. Full `WorkflowPersistence` trait implementation (30+ methods)

**Invariants:**
- KV keys use prefixes: `def-{uuid}`, `inst-{uuid}`, `ut-{uuid}`, `st-{uuid}`, `tmr-{uuid}`, `msg-{uuid}`, `tok-{uuid}`, `xml-{uuid}`, `hist-{uuid}`
- File object keys: `file:{instance_id}-{var_name}-{filename}`
- All values are JSON-serialized (except file data, which is raw bytes)
- Failed operations return `EngineError::PersistenceError(msg)`

## persistence-memory (Testing/Development)

**Crate:** `persistence-memory/`  
**Source:** 1 file: `lib.rs` (~450 LoC)

**Owns:**
1. In-memory `HashMap`-backed storage for all entity types
2. `Vec`-backed history and completed instance archive
3. Full `WorkflowPersistence` trait implementation

**Invariants:**
- Data is NOT persisted to disk — lost on restart
- Used in unit/integration tests (instant setup, no external dependencies)
- May be used by the server only when NATS is down **and** `REQUIRE_NATS` is false (dev)
- Must implement ALL `WorkflowPersistence` methods to serve as a valid backend

## Key Entry Points

| Backend | Entry Point | Returns |
|---------|------------|---------|
| NATS | `NatsPersistence::connect(nats_url: &str, stream_name: &str) -> Result<Self>` | Connected NATS persistence |
| In-Memory | `InMemoryPersistence::new() -> Self` | Empty in-memory persistence |
