# Shared Patterns

Reusable patterns and conventions that span multiple modules.

## Hexagonal Architecture (Ports & Adapters)

**Pattern:** Domain logic (engine-core) defines a trait (port). I/O implementations (persistence-nats, persistence-memory) implement it (adapters). The domain layer never imports I/O crates.

**Used by:** engine-core ↔ persistence-nats, engine-core ↔ persistence-memory

**How to follow:**
- Define traits in `engine-core/src/port/`
- Implement in adapter crates
- Engine uses `Arc<dyn Trait>` — never concrete types
- Adapters can import engine-core types but NOT vice versa

## Lock-Free Wait-State Queues

**Pattern:** Concurrent access to wait-state collections uses `Arc<DashMap<Key, Value>>` for lock-free sharding instead of `RwLock<HashMap>`.

**Used by:** `WorkflowEngine` — four queues: `pending_user_tasks`, `pending_service_tasks`, `pending_timers`, `pending_message_catches`

**How to follow:**
- Use `DashMap` for collections that need concurrent O(1) operations
- Use `RwLock<HashMap>` for collections that need atomic multi-operation updates
- Never hold a `DashMap::Ref` across `.await`

## Fine-Grained Per-Instance Locking

**Pattern:** Rather than a single global lock on all instances, each `ProcessInstance` is wrapped in `Arc<RwLock<ProcessInstance>>` for independent locking.

**Used by:** `InstanceStore` in engine-core

**How to follow:**
- `InstanceStore` has one `RwLock<HashMap<Uuid, Arc<RwLock<ProcessInstance>>>>`
- The outer lock protects the map (insert/remove)
- The inner lock protects the individual instance
- Two concurrent operations on different instances don't block each other

## Fault-Tolerant Persistence Retry

**Pattern:** Two-stage retry: inline (fast, 2 attempts) + **bounded** background queue (slow, exponential backoff).

**Used by:** All persistence operations in engine-core

**How to follow:**
- Inline: 2 attempts with 50ms backoff; if succeeds, no queue
- Queue: bounded `mpsc::channel` (default 10 000; `PERSISTENCE_RETRY_QUEUE_CAPACITY`) → background worker → exponential backoff (1s → 2s → … → 60s, max 50 retries)
- Enqueue via non-blocking `try_send`; on full → drop + `bpmn_persistence_retry_dropped_total`
- Permanent failure after max retries → `bpmn_persistence_retry_exhausted_total`
- Job types: `PersistJob` enum covering all entity types
- Worker re-reads latest state from in-memory before retrying (not stale snapshots)

## Fail-Closed Durability (Server)

**Pattern:** Opt-in production gate `REQUIRE_NATS`; readiness mirrors durability.

**How to follow:**
- Docker/production: `REQUIRE_NATS=true` → refuse start without NATS
- Dev: default `false` → in-memory with warning if NATS down
- `/api/health` always 200; `/api/ready` fails when required persistence is missing or disconnected

## Broadcast Events (SSE Push)

**Pattern:** Engine state changes are broadcast via `tokio::sync::broadcast` channel. Consumers (SSE handler) subscribe and forward to clients.

**Used by:** engine-core → engine-server SSE handler → desktop-tauri

**How to follow:**
- `WorkflowEngine::emit_event(EngineEvent)` after every state change
- Channel capacity 256 — slow consumers miss events (fire-and-forget)
- Events are coarse-grained (`InstanceChanged`, `TaskChanged`, `DefinitionChanged`)
- Clients re-fetch data via REST after receiving an event

## Environment-Driven Configuration

**Pattern:** All configuration comes from environment variables with sensible defaults. No config files.

**Used by:** engine-server main.rs, engine-core ScriptConfig

| Variable | Default | Purpose |
|----------|---------|---------|
| `NATS_URL` | `nats://localhost:4222` | NATS server address |
| `REQUIRE_NATS` | `false` | Fail-fast without NATS; readiness requires persistence |
| `MAX_UPLOAD_BYTES` | `5242880` | Multipart file upload limit (bytes) |
| `PORT` | `8081` | HTTP listen port |
| `TIMER_INTERVAL_MS` | `1000` | Timer check interval |
| `RUST_LOG` | `info` | Log level filter |
| `LOG_FORMAT` | `text` | `text` or `json` |
| `LOG_FILE` | `engine_logs.jsonl` | Log file path (or `off`) |
| `RHAI_MAX_OPERATIONS` | `50000` | Rhai operation limit |
| `RHAI_MAX_MEMORY_BYTES` | `2097152` | Rhai memory budget (derives collection caps) |
| `RHAI_TIMEOUT_MS` | `1000` | Rhai timeout |
| `PERSISTENCE_RETRY_QUEUE_CAPACITY` | `10000` | Bounded background retry queue size |

## Thin Client Pattern (Desktop App)

**Pattern:** The desktop app is a pure presentation layer. All logic, state, and persistence live in engine-server.

**Used by:** desktop-tauri ↔ engine-server

**How to follow:**
- Tauri commands are thin HTTP proxies (no business logic)
- React components consume SSE events → re-fetch via REST
- No local caching of engine state (always fetch from server)
- Configurable API URL (`ENGINE_API_URL`) for different environments

## Camunda-Compatible Service Task API

**Pattern:** Service tasks use Camunda's fetch-and-lock pattern with long polling, lock management, incident creation, and BPMN error throwing.

**Used by:** engine-core (service_task.rs), engine-server (tasks.rs), external-task-client

**API endpoints:**
- `POST /api/service-task/fetchAndLock` — long polling, returns locked tasks
- `POST /api/service-task/{id}/complete` — mark as done
- `POST /api/service-task/{id}/failure` — report failure (decrement retries)
- `POST /api/service-task/{id}/extendLock` — extend lock duration
- `POST /api/service-task/{id}/bpmnError` — trigger BPMN error boundary
- `POST /api/service-task/{id}/retry` — retry an incident
- `POST /api/service-task/{id}/resolve` — manually resolve an incident

## Sub-Process Flattening

**Pattern:** Embedded sub-processes are resolved at parse time into the main graph. At runtime, there are no nested instance hierarchies.

**Used by:** bpmn-parser (parser.rs)

**How it works:**
- Parser detects `<subProcess>` elements
- Inlines all sub-process nodes into the parent graph with prefixed IDs
- Creates synthetic `SubProcessEndEvent` nodes at sub-process boundaries
- Execution listeners and boundary events on sub-processes are propagated to parent scope

Note: This is a deliberate deviation from BPMN 2.0 spec for simplified runtime execution.
