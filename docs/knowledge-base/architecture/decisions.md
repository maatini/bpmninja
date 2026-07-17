# Key Architectural Decisions

## ADR-001: Hexagonal Architecture (Ports & Adapters)

**Decision:** engine-core defines the `WorkflowPersistence` trait as a port. Persistence implementations (NATS, in-memory) are adapters.

**Rationale:**
- engine-core has **zero** network, disk, or NATS code
- Testable with in-memory persistence
- Swappable persistence backends
- Clear boundaries between pure logic and I/O

**Consequences:**
- `WorkflowPersistence` trait is large (30+ methods) — any new persistence operation requires trait changes
- Two implementations must be kept in sync (NATS + in-memory)

## ADR-002: Token-Based Execution Model

**Decision:** BPMN execution uses discrete `Token` structs stored centrally in `ProcessInstance.tokens`. The `NextAction` enum drives an execution loop.

**Rationale:**
- Single source of truth for token state
- Pending tasks hold only `token_id: Uuid`, never copies
- Fork/join parallel execution is natural with token model
- Tokens can be serialized/deserialized for persistence

**Consequences:**
- `ProcessInstance` is large and needs careful serialization
- Token lifecycle must be carefully managed (store before pausing, remove after resuming)

## ADR-003: Sub-Process Flattening at Parse Time

**Decision:** Embedded sub-processes are resolved and inlined into the main graph during BPMN parsing. No runtime scope nesting.

**Rationale:**
- Simplified runtime execution (no nested instance hierarchies)
- Reduced serialization complexity
- Better performance (no scope resolution at runtime)

**Consequences:**
- Deviates from strict BPMN 2.0 spec
- Sub-process boundaries are simulated via `SubProcessEndEvent` nodes
- Cannot support "ad-hoc" or "transaction" sub-processes

## ADR-004: Lock-Free Concurrency via DashMap

**Decision:** Four wait-state queues (`pending_user_tasks`, `pending_service_tasks`, `pending_timers`, `pending_message_catches`) use `Arc<DashMap>` for lock-free sharding. Instance state uses per-instance `RwLock`.

**Rationale:**
- High concurrency under load (multiple HTTP handlers + timer scheduler)
- No global lock contention
- `DashMap` provides lock-free read/write for wait-state operations

**Consequences:**
- Cannot hold a DashMap reference across `.await` (same for RwLock)
- Instance migration/modification requires care with concurrent access

## ADR-005: Camunda-Compatible Service Task API

**Decision:** Service tasks follow Camunda's fetch-and-lock pattern via REST endpoints (`/api/service-task/fetchAndLock`, `/api/service-task/{id}/complete`, etc.).

**Rationale:**
- Familiar API for existing Camunda users
- Enables reuse of Camunda worker libraries (with minor URL changes)
- Long polling for efficiency

**Consequences:**
- `PendingServiceTask` carries Camunda fields (workerId, lockExpiration, retries)
- The TypeScript client mirrors `camunda-external-task-client-js` API

## ADR-006: SSE Push Events for UI

**Decision:** Engine state changes are broadcast via `tokio::sync::broadcast` channel; server exposes `GET /api/events` as SSE. Desktop app subscribes via EventSource.

**Rationale:**
- No polling for UI updates
- Low latency state synchronization
- Simple to implement with tokio broadcast

**Consequences:**
- Channel capacity is 256 — slow consumers may miss events
- SSE is unidirectional; requires separate REST calls for data fetching

## ADR-007: Two-Stage Persistence Retry

**Decision:** Persistence operations use inline retries (2 attempts, 50ms backoff) + a background retry worker queue (mpsc channel, exponential backoff, max 50 retries).

**Rationale:**
- Handles transient NATS outages without data loss
- Inline retry avoids queue overhead for quick recoveries
- Background worker ensures eventual consistency

**Consequences:**
- `PersistJob` enum must cover all persistence operations
- Retry worker needs access to engine state (InstanceStore, DashMaps) to re-read latest data

## ADR-008: Rhai Script Engine (not JavaScript)

**Decision:** BPMN script tasks and execution listeners use the Rhai scripting language, executed in a sandboxed environment with resource limits.

**Rationale:**
- Native Rust integration (no JS runtime overhead)
- Configurable resource limits (max operations, memory, timeout)
- Sufficient for BPMN expression evaluation

**Consequences:**
- Not compatible with Camunda JavaScript/Groovy scripts
- `ScriptConfig` is configurable via environment variables (`RHAI_MAX_OPERATIONS`, `RHAI_TIMEOUT_MS`)
- Heavy scripts may be killed by timeout
