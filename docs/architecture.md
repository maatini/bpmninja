# bpmninja — Architecture Documentation

> BPMN 2.0 workflow engine in Rust, token-based execution
> As of: 2026-10-12 (synced with code; v0.7.24)

---

## 1. Workspace Overview

The project is a Cargo workspace with 6 crates, plus a Tauri desktop app, a TypeScript worker client, an API spec and a fuzz workspace:

| Crate | LoC (incl. tests) | Purpose |
|---|---|---|
| **engine-core** | ~19,100 | Pure state machine, token execution, gateways, scripting, history |
| **bpmn-parser** | ~3,040 | BPMN 2.0 XML → `ProcessDefinition` (quick-xml + serde) |
| **persistence-nats** | ~1,440 | `WorkflowPersistence` via NATS JetStream KV / Object Store / Streams |
| **persistence-memory** | ~120 | Re-export of `engine_core::adapter::InMemoryPersistence` |
| **engine-server** | ~6,740 | Axum REST API, SSE, timer scheduler, auth/CORS, startup restore |
| **agent-orchestrator** | ~130 | One-shot demo worker (not a production loop) |
| **desktop-tauri** | ~9,250 TS + ~1,130 Rust | Tauri + React + Tailwind + bpmn-js (thin client) |
| **external-task-client** | ~1,070 (+ tests) | TypeScript Camunda-style worker SDK |

### Workspace Dependency Graph

```mermaid
graph TD
    subgraph "Rust Workspace"
        EC["engine-core<br/><i>Pure Logic</i>"]
        BP["bpmn-parser<br/><i>XML → Model</i>"]
        PN["persistence-nats<br/><i>NATS JetStream</i>"]
        ES["engine-server<br/><i>Axum REST</i>"]
    end
    
    subgraph "Frontend"
        DT["desktop-tauri<br/><i>React + bpmn-js</i>"]
    end
    
    BP --> EC
    PN --> EC
    ES --> EC
    ES --> BP
    ES --> PN
    DT -->|HTTP / Tauri Commands| ES
    
    style EC fill:#4a9eff,stroke:#333,color:#fff
    style BP fill:#ff9f43,stroke:#333,color:#fff
    style PN fill:#2ed573,stroke:#333,color:#fff
    style ES fill:#a55eea,stroke:#333,color:#fff
    style DT fill:#ff6b81,stroke:#333,color:#fff
```

---

## 2. engine-core — Core Architecture

### 2.1 Module Structure

```mermaid
graph TB
    subgraph "engine-core/src"
        LIB["lib.rs<br/><i>public exports + legacy aliases</i>"]
        DOMAIN["domain/<br/>definition, element (29 variants),<br/>token, flow, timer, listener, error"]
        RUNTIME["runtime/<br/>ProcessInstance, InstanceState,<br/>NextAction, Pending*, stats"]
        PORT["port/persistence.rs<br/>WorkflowPersistence (35 async fns)"]
        ADAPTER["adapter/in_memory.rs<br/>InMemoryPersistence"]
        HIST["history/<br/>HistoryEntry, DiffSnapshot, diffs"]
        SCRIPT["scripting/runner.rs<br/>Rhai sandbox"]
        COND["condition.rs<br/>expression evaluator"]

        subgraph "engine/"
            MOD["mod.rs<br/>WorkflowEngine"]
            EXEC["executor/<br/>run_instance_batch, execute_step,<br/>next_action, parallel, completion"]
            HANDLERS["handlers/<br/>tasks, events, gateways, sub_processes"]
            GW["gateway.rs"]
            BOUND["boundary.rs"]
            OPS["definition_ops / instance_ops /<br/>process_start / user_task / service_task"]
            PROC["timer_processor / message_processor"]
            IDX["topic_index / timer_index<br/>(secondary indexes)"]
            STORE["registry.rs / instance_store.rs"]
            PERSIST["persistence_ops.rs + retry_queue.rs"]
            EVT["events.rs<br/>EngineEvent broadcast"]
        end
    end

    LIB --> DOMAIN
    LIB --> MOD
    MOD --> EXEC
    EXEC --> HANDLERS
    HANDLERS --> GW
    HANDLERS --> BOUND
    HANDLERS --> SCRIPT
    GW --> COND
    MOD --> OPS
    MOD --> PROC
    MOD --> IDX
    MOD --> STORE
    MOD --> PERSIST
    MOD --> EVT
    PERSIST --> PORT
    PERSIST --> HIST
    ADAPTER -. implements .-> PORT
    EXEC --> RUNTIME
```

### 2.2 WorkflowEngine — Component Breakdown (K2)

The engine is split into focused components:

```rust
pub struct WorkflowEngine {
    pub(crate) definitions:              DefinitionRegistry,          // Arc<DashMap<Uuid, Arc<ProcessDefinition>>>
    pub(crate) instances:                InstanceStore,               // Arc<DashMap<Uuid, Arc<RwLock<ProcessInstance>>>>
    pub(crate) pending_user_tasks:       Arc<DashMap<Uuid, PendingUserTask>>,
    pub(crate) pending_service_tasks:    Arc<DashMap<Uuid, PendingServiceTask>>,
    pub(crate) service_task_topic_index: TopicIndex,                  // secondary: topic -> task ids
    pub(crate) pending_timers:           Arc<DashMap<Uuid, PendingTimer>>,
    pub(crate) timer_due_index:          TimerDueIndex,               // secondary: BTreeMap<(expires_at, id)>
    pub(crate) pending_message_catches:  Arc<DashMap<Uuid, PendingMessageCatch>>,
    pub(crate) persistence:              Option<Arc<dyn WorkflowPersistence>>,
    pub(crate) persistence_error_count:  Arc<AtomicU64>,
    pub(crate) retry_tx:                 Option<RetryQueueTx>,        // bounded mpsc
    pub(crate) retry_worker_handle:      Mutex<Option<JoinHandle<()>>>,
    pub(crate) script_config:            ScriptConfig,
    pub(crate) event_tx:                 broadcast::Sender<EngineEvent>,
}
```

| Component | Struct | Locking Strategy |
|---|---|---|
| **DefinitionRegistry** | `Arc<DashMap<Uuid, Arc<ProcessDefinition>>>` | Sharded map, definitions immutable after deploy |
| **InstanceStore** | `Arc<DashMap<Uuid, Arc<RwLock<ProcessInstance>>>>` | Sharded map + per-instance `RwLock` (K1) |
| **PendingTask queues** | `Arc<DashMap<Uuid, Pending*>>` | Sharded, concurrent O(1) ops |
| **Secondary indexes** | `TopicIndex`, `TimerDueIndex` | Derived from the queues (SSOT stays in the DashMaps) |

---

## 3. Data Model

### 3.1 ProcessInstance (after K4 refactoring)

```mermaid
classDiagram
    class ProcessInstance {
        +Uuid id
        +Uuid definition_key
        +String business_key
        +Option~Uuid~ parent_instance_id
        +InstanceState state
        +String current_node
        +Vec~String~ audit_log
        +HashMap~String, Value~ variables
        +HashMap~Uuid, Token~ tokens ★
        +Vec~ActiveToken~ active_tokens
        +HashMap~String, JoinBarrier~ join_barriers
    }

    class Token {
        +Uuid id
        +String current_node
        +HashMap~String, Value~ variables
        +bool is_merged
    }

    class ActiveToken {
        +Token token
        +Option~String~ fork_id
        +usize branch_index
        +bool completed
    }

    class JoinBarrier {
        +String gateway_node_id
        +usize expected_count
        +Vec~Token~ arrived_tokens
    }

    class PendingUserTask {
        +Uuid task_id
        +Uuid instance_id
        +String node_id
        +String assignee
        +Uuid token_id ★
        +DateTime created_at
    }

    class PendingServiceTask {
        +Uuid id
        +Uuid instance_id
        +Uuid definition_key
        +String node_id
        +String topic
        +Uuid token_id ★
        +HashMap~String, Value~ variables_snapshot
        +Option~String~ worker_id
        +i32 retries
    }

    ProcessInstance "1" --o "*" Token : tokens (central store)
    ProcessInstance "1" --o "*" ActiveToken : active_tokens
    ProcessInstance "1" --o "*" JoinBarrier : join_barriers
    PendingUserTask ..> Token : references via token_id
    PendingServiceTask ..> Token : references via token_id

    note for ProcessInstance "★ = K4 refactoring: tokens stored centrally"
```

### 3.2 BPMN Element Types

29 variants in `engine-core/src/domain/element.rs` (closed enum, exhaustive matching):

```rust
pub enum BpmnElement {
    // Start / end
    StartEvent,
    TimerStartEvent(TimerDefinition),             // Duration | AbsoluteDate | CronCycle | RepeatingInterval
    MessageStartEvent { message_name: String },
    EndEvent,
    TerminateEndEvent,
    ErrorEndEvent { error_code: String },
    EscalationEndEvent { escalation_code: String },
    CompensationEndEvent { activity_ref: Option<String> },
    // Tasks (multi_instance is always None — MI is rejected at parse time)
    ServiceTask { topic: String, multi_instance: Option<MultiInstanceDef> },
    UserTask(String),                             // assignee
    ScriptTask { script: String, multi_instance: Option<MultiInstanceDef> },
    SendTask { message_name: String, multi_instance: Option<MultiInstanceDef> },
    // Gateways
    ExclusiveGateway { default: Option<String> },
    InclusiveGateway,
    ParallelGateway,
    EventBasedGateway,
    ComplexGateway { join_condition: Option<String>, default: Option<String> },
    // Intermediate / boundary events
    TimerCatchEvent(TimerDefinition),
    MessageCatchEvent { message_name: String },
    EscalationThrowEvent { escalation_code: String },
    CompensationThrowEvent { activity_ref: Option<String> },
    BoundaryTimerEvent { attached_to: String, timer: TimerDefinition, cancel_activity: bool },
    BoundaryMessageEvent { attached_to: String, message_name: String, cancel_activity: bool },
    BoundaryErrorEvent { attached_to: String, error_code: Option<String> },
    BoundaryEscalationEvent { attached_to: String, escalation_code: Option<String>, cancel_activity: bool },
    BoundaryCompensationEvent { attached_to: String },
    // Sub-processes
    CallActivity { called_element: String },
    EmbeddedSubProcess { start_node_id: String },  // flattened at parse time
    SubProcessEndEvent { sub_process_id: String }, // synthetic end event
}
```

---

## 4. Execution Architecture

### 4.1 Token Lifecycle (K4)

Tokens exist in **exactly one place** at any moment in time:

```mermaid
flowchart TD
    START(("●")) -->|"Token new"| LV["Local Variable"]
    LV -->|"queue.push_back"| EL

    subgraph EL ["Execution Loop"]
        direction LR
        ES["execute_step"] --> NA["NextAction"]
        NA -->|"Continue"| ES
    end

    EL -->|"WaitForUser / Service / Timer"| CS["Central Store<br/>instance.tokens"]
    CS -->|"complete task<br/>tokens.remove"| LV
    EL -->|"WaitForJoin"| MG["Merged"]
    MG -->|"All tokens arrived"| EL
    EL -->|"EndEvent reached"| DONE(("●"))

    style CS fill:#fef3c7,stroke:#ca8a04,color:#0f172a
    style EL fill:#eff6ff,stroke:#2563eb
    style DONE fill:#16a34a,stroke:#16a34a,color:#fff
    style START fill:#1e293b,stroke:#1e293b,color:#fff
```

> **Central store**: `instance.tokens: HashMap<Uuid, Token>` — pending tasks only hold `token_id: Uuid`.

### 4.2 Execution Loop (run_instance_batch)

```mermaid
flowchart TD
    START["run_instance_batch(instance_id, initial_token)"] --> PUSH["queue.push_back(initial_token)"]
    PUSH --> POP{"queue.pop_front()"}
    POP -->|Some token| STEP["execute_step(instance_id, &mut token)"]
    POP -->|None| PERSIST["persist_instance()"]
    
    STEP --> MATCH{NextAction?}
    
    MATCH -->|Continue| QUEUE["queue.push_back(next_token)"]
    QUEUE --> POP
    
    MATCH -->|ContinueMultiple| FORK["register_join_barrier()<br/>register_active_tokens()<br/>queue ← all forked tokens"]
    FORK --> POP
    
    MATCH -->|WaitForJoin| JOIN{"All tokens arrived?"}
    JOIN -->|No| POP
    JOIN -->|Yes| MERGE["Merge variables<br/>queue ← merged_token"]
    MERGE --> POP
    
    MATCH -->|WaitForUser| STORE_UT["Store token → instance.tokens<br/>Push PendingUserTask"]
    STORE_UT --> POP
    
    MATCH -->|WaitForServiceTask| STORE_ST["Store token → instance.tokens<br/>Push PendingServiceTask"]
    STORE_ST --> POP
    
    MATCH -->|WaitForTimer| STORE_TI["Store token → instance.tokens<br/>Push PendingTimer"]
    STORE_TI --> POP
    
    MATCH -->|WaitForCallActivity| CHILD["spawn_call_activity()<br/>Wait for child instance"]
    CHILD --> POP
    
    MATCH -->|Complete| BRANCH["complete_branch_token()"]
    BRANCH --> ALL{"All tokens done?"}
    ALL -->|Yes| DONE["state = Completed<br/>resume_parent_if_needed()"]
    ALL -->|No| POP
    DONE --> PERSIST
    
    PERSIST --> END["return Ok(())"]
    
    style START fill:#4a9eff,color:#fff
    style END fill:#2ed573,color:#fff
    style FORK fill:#ff9f43,color:#fff
    style MERGE fill:#ff9f43,color:#fff
    style DONE fill:#2ed573,color:#fff
```

### 4.3 Gateway Routing

```mermaid
flowchart LR
    subgraph "ExclusiveGateway (XOR)"
        XOR_IN["Token incoming"] --> XOR_EVAL["Evaluate conditions<br/>(first match wins)"]
        XOR_EVAL -->|Match| XOR_OUT["1 Token → target"]
        XOR_EVAL -->|No match| XOR_DEF{"Default?"}
        XOR_DEF -->|Yes| XOR_OUT2["1 Token → default"]
        XOR_DEF -->|No| XOR_ERR["❌ NoMatchingCondition"]
    end

    subgraph "ParallelGateway (AND)"
        AND_IN["Token incoming"] --> AND_CHECK{"incoming ≥ 2<br/>and !is_merged?"}
        AND_CHECK -->|Yes| AND_WAIT["WaitForJoin<br/>(JoinBarrier)"]
        AND_CHECK -->|No| AND_FORK["Fork: N tokens<br/>(one per outgoing)"]
        AND_WAIT --> AND_MERGE["Merge variables<br/>is_merged = true"]
        AND_MERGE --> AND_FORK
    end

    subgraph "InclusiveGateway (OR)"
        OR_IN["Token incoming"] --> OR_EVAL["Evaluate all conditions"]
        OR_EVAL --> OR_FORK["N tokens<br/>(one per match)"]
        OR_FORK --> OR_JOIN["Join: taken-path barrier<br/>(expected = split branch count)"]
    end

    style XOR_ERR fill:#ff4757,color:#fff
    style AND_WAIT fill:#ff9f43,color:#fff
```

Inclusive join is a taken-path barrier: after a matching split (`find_downstream_join`), `expected_count` is the number of branches actually taken. Unstructured joins without a matching split fall back to `incoming_flow_count`.

---

## 5. Persistence Architecture

### 5.1 WorkflowPersistence Trait

`engine-core/src/port/persistence.rs` defines **35 async methods**, grouped as:

| Group | Methods |
|---|---|
| Tokens | `save_token`, `load_tokens`, `delete_token` |
| Instances / Definitions | `save_instance`, `list_instances`, `delete_instance`, `save_definition`, `list_definitions`, `delete_definition` |
| Wait states | save / delete / list for user tasks, service tasks, timers, message catches |
| Files | `save_file`, `load_file`, `delete_file` |
| BPMN XML | `save_bpmn_xml`, `load_bpmn_xml`, `list_bpmn_xml_ids` |
| History | `append_history_entry`, `query_history` |
| Archive | `save_completed_instance`, `query_completed_instances`, `get_completed_instance` |
| Monitoring | `get_storage_info`, `get_bucket_entries`, `get_bucket_entry_detail` |

The full signature list lives in [knowledge-base/modules/engine-core/interfaces.md](knowledge-base/modules/engine-core/interfaces.md).

### 5.2 Implementations

| Backend | Crate | Storage |
|---|---|---|
| `InMemoryPersistence` | `persistence-memory` | `HashMap` + `Vec` (tests & dev) |
| `NatsPersistence` | `persistence-nats` | NATS JetStream KV + ObjectStore |

**NATS storage layout** (source: `persistence-nats/src/client.rs`, `trait_impl.rs`):

| Bucket / Store | Type | Contents | Key |
|---|---|---|---|
| `definitions` | KV | `ProcessDefinition` (JSON) | `{definition_uuid}` |
| `instances` | KV | Active `ProcessInstance` (JSON) | `{instance_uuid}` |
| `user_tasks` / `service_tasks` / `timers` / `messages` | KV | Pending wait states (JSON) | `{uuid}` |
| `tokens` | KV | `Token` (JSON) | `token-{instance_uuid}-{token_uuid}` |
| `history_instances` | KV | Archived (completed) instances | `{instance_uuid}` |
| `bpmn_xml` | Object Store | BPMN XML source | `{definition_uuid}` |
| `instance_files` | Object Store | File variables | `file:{instance}-{var}-{filename}` |
| `WORKFLOW_HISTORY` | Stream | `HistoryEntry`, subject `history.instance.{id}` | — |
| `WORKFLOW_EVENTS` | Stream | subjects `WORKFLOW_EVENTS.*` | — |
| `ENGINE_LOGS` | Stream | Log buffer backup (created by engine-server) | — |

### 5.3 Fault-Tolerant Retry Queue (K6)

Since NATS can experience outages, the engine uses a two-stage retry mechanism for stateful I/O operations:
1. **Inline retry**: short backoff (e.g. 50ms) on the direct call. On success, execution continues immediately.
2. **Bounded background retry queue**: if the inline retry fails (e.g. NATS is offline), a `PersistJob` is enqueued on a bounded channel (default capacity 10 000, env `PERSISTENCE_RETRY_QUEUE_CAPACITY`). A background worker re-reads the latest in-memory state and retries with *exponential backoff* (max 50 attempts). If the queue is full, jobs are **dropped** and counted (`bpmn_persistence_retry_dropped_total`) to avoid OOM under prolonged outage.
This prevents unbounded memory growth while still recovering from transient network failure.

---

## 6. REST API (engine-server)

> Complete OpenAPI 3.0 specification: **[docs/openapi.yaml](openapi.yaml)**

### 6.1 Route Overview (46 endpoints)

```mermaid
graph LR
    subgraph "Process Definitions"
        D1["POST /api/deploy"]
        D2["GET /api/definitions"]
        D3["GET /api/definitions/:id/xml"]
        D4["DELETE /api/definitions/:id"]
        D5["DELETE /api/definitions/bpmn/:bpmn_id"]
    end
    
    subgraph "Process Instances"
        I1["POST /api/start"]
        I1b["POST /api/start/latest"]
        I1c["POST /api/start/timer"]
        I2["GET /api/instances"]
        I3["GET /api/instances/:id"]
        I4["DELETE /api/instances/:id"]
        I5["PUT /api/instances/:id/variables"]
        I6["POST /api/instances/:id/suspend"]
        I7["POST /api/instances/:id/resume"]
    end
    
    subgraph "User Tasks"
        T1["GET /api/tasks"]
        T2["POST /api/complete/:id"]
    end
    
    subgraph "Service Tasks (Camunda-style)"
        S1["GET /api/service-tasks"]
        S2["POST /api/service-task/fetchAndLock"]
        S3["POST /api/service-task/:id/complete"]
        S4["POST /api/service-task/:id/failure"]
        S5["POST /api/service-task/:id/extendLock"]
        S6["POST /api/service-task/:id/bpmnError"]
        S7["POST /api/service-task/:id/retry"]
        S8["POST /api/service-task/:id/resolve"]
    end
    
    subgraph "Files"
        F1["POST /api/instances/:id/files/:var"]
        F2["GET /api/instances/:id/files/:var"]
        F3["DELETE /api/instances/:id/files/:var"]
    end
    
    subgraph "Events"
        E1["POST /api/message"]
        E2["POST /api/timers/process"]
    end
    
    subgraph "Monitoring & Health"
        M0["GET /api/health"]
        M0b["GET /api/ready"]
        M1["GET /api/info"]
        M2["GET /api/monitoring"]
        M2b["GET /api/monitoring/buckets/:bucket/entries"]
        M2c["GET /api/monitoring/buckets/:bucket/entries/:key"]
        M3["GET /api/instances/:id/history"]
        M3b["GET /api/instances/:id/history/:eid"]
    end
    
    style D1 fill:#2ed573,color:#fff
    style I1 fill:#2ed573,color:#fff
    style I1b fill:#2ed573,color:#fff
    style S2 fill:#ff9f43,color:#fff
    style M0 fill:#2ed573,color:#fff
    style M0b fill:#2ed573,color:#fff
```

### 6.2 Server Architecture

```rust
pub struct AppState {
    pub(crate) engine:           Arc<WorkflowEngine>,                  // Shared engine (no outer lock)
    pub(crate) persistence:      Option<Arc<dyn WorkflowPersistence>>, // NATS backend if connected
    pub(crate) deployed_xml:     Arc<RwLock<HashMap<String, String>>>, // XML cache (definition key → XML)
    pub(crate) nats_url:         String,                               // For /api/info
    pub(crate) log_buffer:       Arc<LogBuffer>,
    pub(crate) require_nats:     bool,                                 // REQUIRE_NATS → readiness / durability
    pub(crate) max_upload_bytes: usize,                                // MAX_UPLOAD_BYTES (default 5 MiB)
    pub(crate) max_xml_bytes:    usize,                                // MAX_XML_BYTES (default 10 MiB)
    pub(crate) api_key:          Option<String>,                       // BPMNINJA_API_KEY; None = auth off
    pub(crate) cors_origins:     Vec<String>,                          // CORS_ORIGINS allowlist; "*" = any (dev)
}
```

> The server shares the engine only via `Arc<WorkflowEngine>`. Because all inner collections (`DashMap`, per-instance `RwLock`) are thread-safe and mutations go through `&self`, there is no longer a monolithic read/write lock over the entire engine. This eliminates contention under heavy HTTP traffic. Instances are isolated via **K1 (per-instance locking)** through `InstanceStore`.

### 6.3 Background Timer Scheduler

The server spawns a Tokio background task that periodically calls `engine.process_timers()`:

```rust
// main.rs — automatic timer polling (lock-free via Arc<WorkflowEngine>)
let timer_interval_ms: u64 = env::var("TIMER_INTERVAL_MS")
    .ok().and_then(|v| v.parse().ok()).unwrap_or(1000);

tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_millis(timer_interval_ms)).await;
        match timer_engine.process_timers().await {
            Ok(n) => tracing::info!("Timer scheduler: processed {} expired timer(s)", n),
            Err(e) => tracing::warn!("Timer scheduler error: {}", e),
        }
    }
});
```

> **Configuration**: `TIMER_INTERVAL_MS` (default: 1000ms). No external cron required.

### 6.4 Health & Readiness

| Endpoint | Function | Check |
|----------|----------|-------|
| `GET /api/health` | Liveness probe | Always `200 OK` when the server is running |
| `GET /api/ready` | Readiness probe | `503` if `REQUIRE_NATS` is set but no persistence is configured, or if NATS storage check fails; otherwise `200` |

`REQUIRE_NATS` defaults to true so a missing NATS backend fails startup instead of silently running in-memory. Set `REQUIRE_NATS=false` only for ephemeral local runs.

---

## 7. Desktop App (Tauri)

### 7.1 Frontend Components

| File | LoC | Purpose |
|---|---|---|
| `App.tsx` | ~180 | Main layout, tab navigation (7 tabs), timer-start detection |
| `ModelerPage.tsx` | ~350 | bpmn-js modeler with deploy, start & variable dialog |
| `InstancesPage.tsx` | ~245 | Instance list (grouped by definition), suspend icon |
| `InstanceDetailDialog.tsx` | ~345 | Instance details with suspend/resume button, timer cycle banner, auto-refresh |
| `InstanceViewer.tsx` | ~125 | Read-only BPMN viewer with active node highlighting + timer pulse |
| `HistoryTimeline.tsx` | ~225 | Event table with filters, detail dialog, diff display |
| `DeployedProcessesPage.tsx` | ~330 | Version grouping, accordion, cascade delete |
| `VariableEditor.tsx` | ~480 | Typed editor (6 types including file), upload/download |
| `MonitoringPage.tsx` | ~365 | Metric cards, NATS storage breakdown, KV browser, auto-refresh |
| `PendingTasksPage.tsx` | ~290 | User & service task lists with completion dialogs |
| `IncidentsPage.tsx` | ~165 | Incident cards with quick retry, detail link, auto-refresh |
| `IncidentDetailDialog.tsx` | ~160 | Retry (configurable retries) + resolve (with VariableEditor) |
| `SettingsPage.tsx` | ~165 | API URL config + connection verify |
| `ErrorBoundary.tsx` | ~72 | React error boundary |
| `MessageDialog.tsx` | ~93 | Message correlation dialog |
| `lib/tauri.ts` | ~170 | All Tauri command wrappers (typed API layer) |
| Custom Properties | ~290 | Condition, script, topic extensions for bpmn-js |
| `index.css` | ~165 | TailwindCSS + HSL design token variables |

### 7.2 Thin-Client Architecture

The desktop app operates as a **thin client** — all workflow logic lives in `engine-server`.

```mermaid
graph TD
    UI["React UI<br/>(desktop-tauri/src)"]
    UI -->|"invoke('deploy_definition')"| TC["Tauri Commands<br/>(src-tauri/src/, 623 LoC)"]
    TC -->|"HTTP REST (reqwest)"| SERVER["engine-server<br/>:8081"]
    SERVER --> ENGINE["WorkflowEngine"]
    ENGINE -.-> NATS[("NATS JetStream")]
    
    style UI fill:#ff6b81,color:#fff
    style TC fill:#a55eea,color:#fff
    style SERVER fill:#4a9eff,color:#fff
    style NATS fill:#2ed573,color:#fff
```

> **Configuration**: `ENGINE_API_URL` environment variable (default: `http://localhost:8081`).

---

## 8. Concurrency & Locking (K1)

### 8.1 Lock Hierarchy

```
WorkflowEngine (Arc)
├── DefinitionRegistry       → Arc<DashMap>                  (sharded)
├── InstanceStore             → Arc<DashMap>                  (sharded map)
│   └── ProcessInstance[i]   → Arc<RwLock<ProcessInstance>>  (per-instance lock!)
├── pending_user_tasks       → Arc<DashMap>                  (lock-free / sharded)
├── pending_service_tasks    → Arc<DashMap>                  (lock-free / sharded)
├── pending_timers           → Arc<DashMap>                  (lock-free / sharded)
└── pending_message_catches  → Arc<DashMap>                  (lock-free / sharded)
```

### 8.2 Deadlock Prevention Pattern

```rust
// ❌ FORBIDDEN: hold a lock across .await
let inst = instance_arc.write().await;
self.some_async_method().await;  // DEADLOCK!

// ✅ CORRECT: lock scoped before .await
{
    let mut inst = instance_arc.write().await;
    inst.state = InstanceState::Running;
}  // Lock dropped
self.some_async_method().await;  // Safe!
```

---

## 9. History & Audit Trail

Every state transition is stored as a `HistoryEntry`:

| Field | Type | Description |
|---|---|---|
| `event_type` | `HistoryEventType` | InstanceStarted, TaskCompleted, TokenForked, ... |
| `diff` | `Option<HistoryDiff>` | Automatically computed diff (variables, status, node) |
| `actor_type` | `ActorType` | Engine, User, ServiceWorker, Timer, Listener |
| `full_state_snapshot` | `Option<Value>` | Snapshot every 8 audit entries |

**Diff calculation:** `calculate_diff(old: &ProcessInstance, new: &ProcessInstance) → HistoryDiff`
- Variable diff: added, removed, changed (with value truncation >1KB)
- Status diff: "Running → Completed"
- Node diff: "task1 → end"
- File upload detection: "File 'report.pdf' uploaded (1.2 MB)"

---

## 10. Code Statistics

> As of: 2026-10-12 — measured via `wc -l` (Rust totals include inline/test modules) and by counting test attributes

| Area | Files | LOC |
|---|---|---|
| engine-core | 74 | ~19,100 |
| bpmn-parser | 4 | ~3,040 |
| persistence-nats | 5 | ~1,440 |
| persistence-memory | 1 | ~120 |
| engine-server (lib + E2E tests) | 37 | ~6,740 |
| agent-orchestrator | 1 | ~130 |
| desktop-tauri (TypeScript) | 63 | ~9,250 |
| desktop-tauri (Rust backend) | — | ~1,130 |
| external-task-client (lib) | — | ~1,070 |
| fuzz targets | 9 | ~710 |

### Test Overview (approximate)

| Crate | Unit | E2E / Integration | Total |
|---|---|---|---|
| engine-core | ~247 | 5 | ~252 |
| bpmn-parser | ~46 | — | ~46 |
| persistence-nats | 4 | — | 4 |
| engine-server | ~15 | ~70 | ~85 |
| desktop-tauri (Playwright) | — | 52 | 52 |
| external-task-client (Vitest) | 76 | — | 76 |

Source of truth: `cargo test --workspace`.
