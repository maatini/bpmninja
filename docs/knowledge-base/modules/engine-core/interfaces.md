# engine-core — Interfaces

## Public API Surface

Stable module aliases (not deprecated): `engine_core::model` → `domain`, `engine_core::persistence` → `port`, `engine_core::timer_definition` → `domain::timer`.

### Core Structs (re-exported via `engine-core::*`)

| Type | Location | Purpose |
|------|----------|---------|
| `WorkflowEngine` | `engine/mod.rs` | Central engine — all operations |
| `ProcessDefinition` | `domain/definition.rs` | Immutable BPMN definition with nodes, edges, listeners |
| `ProcessDefinitionBuilder` | `domain/definition.rs` | Builder pattern for constructing definitions (used in tests) |
| `BpmnElement` | `domain/element.rs` | Enum: 29 BPMN element variants |
| `Token` | `domain/token.rs` | Execution token with id, current_node, variables |
| `SequenceFlow` | `domain/flow.rs` | Edge between nodes with optional condition |
| `TimerDefinition` | `domain/timer.rs` | ISO 8601 timer: Duration, AbsoluteDate, CronCycle, RepeatingInterval |
| `ExecutionListener` | `domain/listener.rs` | Script attached to a node (start/end event) |
| `ListenerEvent` | `domain/listener.rs` | Enum: Start, End |
| `ScopeEventListener` | `domain/listener.rs` | Timer/Message/Error event sub-process listener |
| `MultiInstanceDef`, `StandardLoopDef`, `LoopCharacteristics` | `domain/multi_instance.rs` | Loop configuration per node in `ProcessDefinition.loops` (builder: `loop_characteristics`) |
| `FileReference` | `domain/file_ref.rs` | File variable metadata (object_key, filename, mime_type, size) |

### Runtime Types

| Type | Location | Purpose |
|------|----------|---------|
| `ProcessInstance` | `runtime/instance.rs` | Live process instance with tokens, state, variables |
| `InstanceState` | `runtime/instance.rs` | Enum: Running, WaitingOn*, Completed, Suspended, etc. |
| `NextAction` | `runtime/instance.rs` | Execution loop dispatch: Continue, WaitFor*, Complete, Terminate, etc. |
| `ActiveToken` | `runtime/instance.rs` | Token traveling through graph with fork_id and branch_index |
| `JoinBarrier` | `runtime/instance.rs` | Synchronization barrier at converging gateway |
| `MultiInstanceProgress` | `runtime/instance.rs` | Persisted progress per loop activity (`ProcessInstance.multi_instance_state`): counts, active children, outputs, merged + shadowed variables |
| `CompensationRecord` | `runtime/instance.rs` | Completed compensatable activity + handler node |
| `PendingUserTask` | `runtime/pending.rs` | User task awaiting completion (has token_id ref) |
| `PendingServiceTask` | `runtime/pending.rs` | Service task with lock, retries, variables_snapshot |
| `PendingTimer` | `runtime/pending.rs` | Timer with expiry and optional repeat config |
| `PendingMessageCatch` | `runtime/pending.rs` | Message subscription awaiting correlation |
| `EngineStats` | `runtime/stats.rs` | Monitoring statistics (instance/task/timer counts) |

### Error Types

| Type | Purpose |
|------|---------|
| `EngineError` | Enum: all engine errors (NoSuchDefinition, NoSuchInstance, TaskNotPending, ServiceTaskLocked, etc.) |
| `EngineResult<T>` | Type alias: `Result<T, EngineError>` |

### History Types

| Type | Purpose |
|------|---------|
| `HistoryEntry` | Single audit event with timestamp, type, diff, actor, snapshot |
| `HistoryEventType` | Enum: 29 event types (InstanceStarted, TaskCompleted, TokenForked, etc.) |
| `HistoryDiff` | Diff between two states (variables, status, node) |
| `VariableDiff` | Added/removed/changed variables |
| `ActorType` | Who caused the change: Engine, User, ServiceWorker, Timer, Listener |

### Persistence Trait

```rust
#[async_trait]
pub trait WorkflowPersistence: Send + Sync {
    // Token CRUD
    async fn save_token(&self, instance_id: Uuid, token: &Token) -> EngineResult<()>;
    async fn load_tokens(&self, instance_id: Uuid) -> EngineResult<Vec<Token>>;
    async fn delete_token(&self, instance_id: Uuid, token_id: Uuid) -> EngineResult<()>;
    
    // Instance & Definition CRUD
    async fn save_instance(&self, instance: &ProcessInstance) -> EngineResult<()>;
    async fn list_instances(&self) -> EngineResult<Vec<ProcessInstance>>;
    async fn delete_instance(&self, id: &str) -> EngineResult<()>;
    async fn save_definition(&self, definition: &ProcessDefinition) -> EngineResult<()>;
    async fn list_definitions(&self) -> EngineResult<Vec<ProcessDefinition>>;
    async fn delete_definition(&self, key: &str) -> EngineResult<()>;
    
    // Task Queues (4 types)
    async fn save_user_task(&self, task: &PendingUserTask) -> EngineResult<()>;
    async fn delete_user_task(&self, task_id: Uuid) -> EngineResult<()>;
    async fn list_user_tasks(&self) -> EngineResult<Vec<PendingUserTask>>;
    async fn save_service_task(&self, task: &PendingServiceTask) -> EngineResult<()>;
    async fn delete_service_task(&self, task_id: Uuid) -> EngineResult<()>;
    async fn list_service_tasks(&self) -> EngineResult<Vec<PendingServiceTask>>;
    async fn save_timer(&self, timer: &PendingTimer) -> EngineResult<()>;
    async fn delete_timer(&self, timer_id: Uuid) -> EngineResult<()>;
    async fn list_timers(&self) -> EngineResult<Vec<PendingTimer>>;
    async fn save_message_catch(&self, catch: &PendingMessageCatch) -> EngineResult<()>;
    async fn delete_message_catch(&self, catch_id: Uuid) -> EngineResult<()>;
    async fn list_message_catches(&self) -> EngineResult<Vec<PendingMessageCatch>>;
    
    // File Storage (Object Store)
    async fn save_file(&self, object_key: &str, data: &[u8]) -> EngineResult<()>;
    async fn load_file(&self, object_key: &str) -> EngineResult<Vec<u8>>;
    async fn delete_file(&self, object_key: &str) -> EngineResult<()>;
    
    // BPMN XML Storage
    async fn save_bpmn_xml(&self, key: &str, xml: &str) -> EngineResult<()>;
    async fn load_bpmn_xml(&self, key: &str) -> EngineResult<String>;
    async fn list_bpmn_xml_ids(&self) -> EngineResult<Vec<String>>;
    
    // History
    async fn append_history_entry(&self, entry: &HistoryEntry) -> EngineResult<()>;
    async fn append_history_entries(&self, entries: &[HistoryEntry]) -> EngineResult<()>; // default: loop
    async fn query_history(&self, query: HistoryQuery) -> EngineResult<Vec<HistoryEntry>>;
    
    // Archive
    async fn save_completed_instance(&self, instance: &ProcessInstance) -> EngineResult<()>;
    async fn query_completed_instances(&self, query: CompletedInstanceQuery) -> EngineResult<Vec<ProcessInstance>>;
    async fn get_completed_instance(&self, id: &str) -> EngineResult<Option<ProcessInstance>>;
    
    // Monitoring
    async fn get_storage_info(&self) -> EngineResult<Option<StorageInfo>>;
    async fn get_bucket_entries(&self, bucket: &str, offset: usize, limit: usize) -> EngineResult<Vec<BucketEntry>>;
    async fn get_bucket_entry_detail(&self, bucket: &str, key: &str) -> EngineResult<BucketEntryDetail>;
}
```

### EngineEvent (broadcast channel)

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    InstanceChanged,   // Instance started, advanced, or completed
    TaskChanged,       // User/Service task created or completed
    DefinitionChanged, // Definition deployed or deleted
}
```

### Scripting (`scripting/`)

| Type / Function | Purpose |
|-----------------|---------|
| `ScriptConfig` | `max_operations`, `max_memory` (budget), `timeout_ms`; `from_env()`, `build_engine()`, `derived_collection_limits()` |
| `ScriptRunner` | Shared engine + compiled-script cache: `new(config)`, `execute(script, vars)`; used by `WorkflowEngine` |
| `execute_script_safe(config, script, vars)` | One-off hardened entry (fresh engine per call): `spawn_blocking` + wall-clock timeout + Rhai limits |
| Env | `RHAI_MAX_OPERATIONS` (50 000), `RHAI_MAX_MEMORY_BYTES` (2 MiB), `RHAI_TIMEOUT_MS` (1000) |

`max_memory` does **not** call a Rhai heap API (none exists). It scales `set_max_string_size` / `set_max_array_size` / `set_max_map_size` relative to the 2 MiB default.

### In-Process-Task-Handler (`engine/task_handler.rs`)

| Item | Detail |
|------|--------|
| `TaskHandler` | `#[async_trait]`-Trait: `async fn handle(&self, task: &PendingServiceTask) -> TaskOutcome` |
| `TaskOutcome` | `Complete(vars)`, `Fail { message, details, retries, retry_timeout_ms }`, `BpmnError { code, message, variables }` |
| `register_task_handler(topic, Arc<dyn TaskHandler>)` / `unregister_task_handler(topic)` | Handler pro Topic (ersetzt vorhandenen) |
| `spawn_task_handler_dispatcher(self: &Arc<Self>) -> JoinHandle<()>` | Long-Poll über `fetch_and_lock_service_tasks` mit Worker-ID `IN_PROCESS_WORKER_ID`, Lock 300 s, max. 64 parallele Handler; Ergebnis über `complete_service_task` / `fail_service_task_with_retry_timeout` / `handle_bpmn_error_with_variables` |

Locks, Retries, Incidents, Error-Boundaries und Persistenz verhalten sich wie bei externen Workern; Topics ohne Handler bleiben für REST-/Camunda-Worker abrufbar. Panik im Handler = `Fail` mit `retries: None`. CPU-lastige Handler gehören in `spawn_blocking`. Der Server registriert keine Handler (Library-Feature für eingebettete Nutzung).

### Signale (`engine/signal.rs`, X1)

| Item | Detail |
|------|--------|
| Elemente | `SignalStartEvent`, `SignalCatchEvent`, `SignalThrowEvent`, `SignalEndEvent`, `BoundarySignalEvent { cancel_activity }` |
| `broadcast_signal(name, vars) -> Vec<Uuid>` | Setzt **alle** wartenden Signal-Catches/-Boundaries fort (ohne Business-Key) und startet die jeweils neueste Definition mit passendem Signal-Start; Fehler einzelner Instanzen werden geloggt, nicht propagiert |
| Wartezustand | `PendingMessageCatch` mit `kind: CatchKind::Signal` (`#[serde(default)]` = `Message` für Altdaten); `correlate_message` ignoriert Signal-Catches |
| Throw/End | Signal landet in der `SignalOutbox` und wird nach `run_instance_batch` zugestellt (keine Locks, kein Re-Entry); nur der äußerste Aufrufer liefert aus. Keine Variablen-Weitergabe. Signal-Start, der dasselbe Signal wirft, erzeugt eine Endlosschleife |

### Loops / Multi-Instance (`engine/loops.rs`, X4/X5)

| Item | Detail |
|------|--------|
| Entry | `execute_step` → `enter_loop_activity` when a token reaches a node in `def.loops` without a matching `Token.loop_instance` marker. Parallel → `NextAction::MultiInstanceFork` (children registered as active tokens); sequential / standard loop → `Continue` with marker; 0 instances or false `testBefore` → skip to successor |
| Exit | `execute_step` → `leave_loop_instance` when a marked token is no longer at its activity. Counting happens under one instance write lock; non-final parallel children end via `NextAction::Complete`; the final step continues a merged token (`is_merged`) with outgoing flows evaluated on the final variables |
| Boundary | Boundary paths call `interrupt_loop_activity` before moving on (message/signal, timer, service-task `bpmnError`); call-activity errors stop at the boundary node and are caught in the exit hook. Remaining children: wait states, user/service tasks and tokens are removed (child processes of call activities keep running) |
| Variables | `loopCounter`, `nrOfInstances`, `nrOfCompletedInstances`, `nrOfActiveInstances`, element and output element are local (previous values restored). Expressions accept `${…}` |

### Retry queue (internal)

| Item | Detail |
|------|--------|
| Channel | Bounded `mpsc` (default 10 000; `PERSISTENCE_RETRY_QUEUE_CAPACITY`) |
| Enqueue | Non-blocking `try_send`; on `Full` → drop + `bpmn_persistence_retry_dropped_total` |
| Exhausted | After 50 background retries → `bpmn_persistence_retry_exhausted_total` |
| Shutdown | Async `send(Shutdown)` then await worker join |

### Public Functions

| Function | Location | Purpose |
|----------|----------|---------|
| `evaluate_condition(expr: &str, variables: &HashMap<String, Value>) -> bool` | `condition.rs` | Evaluate gateway conditions |
| `parse_iso8601_duration(s: &str) -> EngineResult<Duration>` | (via bpmn-parser) | Parse ISO 8601 duration strings |
