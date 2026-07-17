# engine-core — Interfaces

## Public API Surface

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
| `MultiInstanceDef` | `domain/multi_instance.rs` | Multi-instance configuration |
| `FileReference` | `domain/file_ref.rs` | File variable metadata (object_key, filename, mime_type, size) |

### Runtime Types

| Type | Location | Purpose |
|------|----------|---------|
| `ProcessInstance` | `runtime/instance.rs` | Live process instance with tokens, state, variables |
| `InstanceState` | `runtime/instance.rs` | Enum: Running, WaitingOn*, Completed, Suspended, etc. |
| `NextAction` | `runtime/instance.rs` | Execution loop dispatch: Continue, WaitFor*, Complete, Terminate, etc. |
| `ActiveToken` | `runtime/instance.rs` | Token traveling through graph with fork_id and branch_index |
| `JoinBarrier` | `runtime/instance.rs` | Synchronization barrier at converging gateway |
| `MultiInstanceProgress` | `runtime/instance.rs` | Progress tracking for multi-instance tasks |
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
| `execute_script_safe(config, script, vars)` | Hardened entry: `spawn_blocking` + wall-clock timeout + Rhai limits |
| Env | `RHAI_MAX_OPERATIONS` (50 000), `RHAI_MAX_MEMORY_BYTES` (2 MiB), `RHAI_TIMEOUT_MS` (1000) |

`max_memory` does **not** call a Rhai heap API (none exists). It scales `set_max_string_size` / `set_max_array_size` / `set_max_map_size` relative to the 2 MiB default.

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
