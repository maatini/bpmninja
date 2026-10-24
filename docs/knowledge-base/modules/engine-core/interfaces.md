# engine-core — Interfaces

## Public API Surface

Stable module aliases (not deprecated): `engine_core::model` → `domain`, `engine_core::persistence` → `port`, `engine_core::timer_definition` → `domain::timer`.

### Core Structs (re-exported via `engine-core::*`)

| Type | Location | Purpose |
|------|----------|---------|
| `WorkflowEngine` | `engine/mod.rs` | Central engine — all operations |
| `ProcessDefinition` | `domain/definition.rs` | Immutable BPMN definition with nodes, edges, listeners |
| `ProcessDefinitionBuilder` | `domain/definition.rs` | Builder pattern for constructing definitions (used in tests) |
| `BpmnElement` | `domain/element.rs` | Enum: 37 BPMN element variants (inkl. Signal- und Conditional-Events, LinkThrowEvent) |
| `Token` | `domain/token.rs` | Execution token with id, current_node, variables |
| `SequenceFlow` | `domain/flow.rs` | Edge between nodes with optional condition |
| `TimerDefinition` | `domain/timer.rs` | ISO 8601 timer: Duration, AbsoluteDate, CronCycle, RepeatingInterval |
| `ExecutionListener` | `domain/listener.rs` | Script attached to a node (start/end event) |
| `ListenerEvent` | `domain/listener.rs` | Enum: Start, End |
| `ScopeEventListener` | `domain/listener.rs` | Timer/Message/Error event sub-process listener |
| `MultiInstanceDef` | `domain/multi_instance.rs` | Multi-instance configuration |
| `FileReference` | `domain/file_ref.rs` | File variable metadata (object_key, filename, mime_type, size) |
| `Incident` | `domain/incident.rs` | Incident an einer Instanz: kind, node_id, task_id, message, details, occurrences, resolved_at/by |
| `IncidentKind` | `domain/incident.rs` | Enum: `ServiceTask`, `Timer`, `Persistence`, `BpmnError` (Serde snake_case) |

### Runtime Types

| Type | Location | Purpose |
|------|----------|---------|
| `ProcessInstance` | `runtime/instance.rs` | Live process instance with tokens, state, variables |
| `ProcessInstance.incidents` | `runtime/instance.rs` | `Vec<Incident>`, `#[serde(default)]`, max. 100 pro Instanz |
| `InstanceState` | `runtime/instance.rs` | Enum: Running, `WaitingOnSignal`, `WaitingOnCondition`, WaitingOn*, Completed, Suspended, etc. |
| `NextAction` | `runtime/instance.rs` | Execution loop dispatch: Continue, `WaitForSignal`, `WaitForCondition`, WaitFor*, Complete, Terminate, etc. |
| `ActiveToken` | `runtime/instance.rs` | Token traveling through graph with fork_id and branch_index |
| `JoinBarrier` | `runtime/instance.rs` | Synchronization barrier at converging gateway |
| `MultiInstanceProgress` | `runtime/instance.rs` | Progress tracking for multi-instance tasks |
| `CompensationRecord` | `runtime/instance.rs` | Completed compensatable activity + handler node |
| `PendingUserTask` | `runtime/pending.rs` | User task awaiting completion (has token_id ref) |
| `PendingServiceTask` | `runtime/pending.rs` | Service task with lock, retries, variables_snapshot |
| `PendingTimer` | `runtime/pending.rs` | Timer with expiry and optional repeat config |
| `PendingMessageCatch` | `runtime/pending.rs` | Message subscription awaiting correlation |
| `PendingSignalCatch` | `runtime/pending.rs` | Signal subscription awaiting broadcast |
| `PendingConditionalCatch` | `runtime/pending.rs` | Conditional catch with `last_result` for `false → true` edge detection |
| `EngineStats` | `runtime/stats.rs` | Monitoring statistics (instance/task/timer/message/signal/conditional counts) |

### Error Types

| Type | Purpose |
|------|---------|
| `EngineError` | Enum: all engine errors (NoSuchDefinition, NoSuchInstance, TaskNotPending, ServiceTaskLocked, IncidentNotFound, IncidentAlreadyResolved, etc.) |
| `EngineResult<T>` | Type alias: `Result<T, EngineError>` |

### History Types

| Type | Purpose |
|------|---------|
| `HistoryEntry` | Single audit event with timestamp, type, diff, actor, snapshot |
| `HistoryEventType` | Enum: 23 event types (InstanceStarted, TaskCompleted, TokenForked, SignalCaught, ConditionalCaught, etc.) |
| `HistoryDiff` | Diff between two states (variables, status, node) |
| `VariableDiff` | Added/removed/changed variables |
| `ActorType` | Who caused the change: Engine, User, ServiceWorker, Timer, Listener |

### Persistence Trait

41 async methods. Wait-state methods are grouped per event type; a future consolidation into a generic wait-state port is planned.

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
    async fn save_signal_catch(&self, catch: &PendingSignalCatch) -> EngineResult<()>;
    async fn delete_signal_catch(&self, catch_id: Uuid) -> EngineResult<()>;
    async fn list_signal_catches(&self) -> EngineResult<Vec<PendingSignalCatch>>;
    async fn save_conditional_catch(&self, catch: &PendingConditionalCatch) -> EngineResult<()>;
    async fn delete_conditional_catch(&self, catch_id: Uuid) -> EngineResult<()>;
    async fn list_conditional_catches(&self) -> EngineResult<Vec<PendingConditionalCatch>>;
    
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

### Incident-Modell und Lebenszyklus

```rust
pub struct Incident {
    pub id: Uuid,
    pub kind: IncidentKind,          // service_task | timer | persistence | bpmn_error
    pub node_id: Option<String>,
    pub task_id: Option<Uuid>,
    pub message: String,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub occurrences: u32,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}
```

| Phase | Verhalten |
|-------|-----------|
| Anlage | `ProcessInstance::push_or_touch_incident(kind, node_id, task_id, message, details)` |
| Dedup | Offener Incident mit gleichem `(kind, node_id, task_id)` → `occurrences += 1`, `last_seen_at`/`message` aktualisiert |
| Limit | Max. `MAX_INCIDENTS_PER_INSTANCE = 100`; beim Trimmen weichen aufgelöste vor offenen, jeweils die ältesten |
| Auflösen | `WorkflowEngine::resolve_instance_incident(instance_id, incident_id, resolved_by)` setzt `resolved_at`/`resolved_by` und persistiert die Instanz |
| Persistenz | Incidents sind Teil des `ProcessInstance`-Dokuments (`#[serde(default)]`), keine eigene `WorkflowPersistence`-Methode |
| Metrik | `bpmn_incidents_total{kind="service_task" / "timer" / "persistence" / "bpmn_error"}` |

Quellen: Service-Task mit `retries <= 0` bzw. unbehandeltem `bpmnError`; fehlschlagende Timer-Auswertung (dedupliziert trotz 2-s-Retry); Persistenz-Job nach 50 Retries (Instanz-Zuordnung über `SaveInstance`, `AppendHistoryEntry` bzw. Pending-Maps); ungefangener `ErrorEndEvent` am Prozessscope sowie unbehandelter Child-Error einer Call Activity.

### Retry queue (internal)

| Item | Detail |
|------|--------|
| Channel | Bounded `mpsc` (default 10 000; `PERSISTENCE_RETRY_QUEUE_CAPACITY`) |
| Enqueue | Non-blocking `try_send`; on `Full` → drop + `bpmn_persistence_retry_dropped_total` |
| Exhausted | After 50 background retries → `bpmn_persistence_retry_exhausted_total` + Persistence-Incident an der zuordenbaren Instanz |
| Shutdown | Async `send(Shutdown)` then await worker join |

### Public Functions

| Function | Location | Purpose |
|----------|----------|---------|
| `evaluate_condition(expr: &str, variables: &HashMap<String, Value>) -> bool` | `condition.rs` | Wertet Gateway-Bedingungen aus (siehe unten) |
| `validate_condition(expr: &str) -> Result<(), String>` | `condition.rs` | Prüft die Syntax; Fehler enthalten die 1-basierte Zeichenposition |
| `parse_iso8601_duration(s: &str) -> EngineResult<Duration>` | (via bpmn-parser) | Parse ISO 8601 duration strings |

### Bedingungssprache (`condition.rs`)

Eigener Lexer + Recursive-Descent-Parser, keine Rhai-/`eval`-Abhängigkeit. Grammatik (Kurzschluss bei `&&`/`||`):

```
or      := and (("||" | "or") and)*
and     := not (("&&" | "and") not)*
not     := ("!" | "not") not | comparison
compare := additive (("==" | "!=" | ">" | "<" | ">=" | "<=") additive)?
additive:= multiplicative (("+" | "-") multiplicative)*
mult    := unary (("*" | "/" | "%") unary)*
unary   := "-" unary | primary
primary := Zahl | String | true | false | null | "(" or ")" | Pfad
Pfad    := Ident (("." Ident) | ("[" Ganzzahl "]"))*
```

| Aspekt | Verhalten |
|--------|-----------|
| Wrapper | Nur eine vollständige `${…}`-/`#{…}`-Umhüllung wird entfernt; mitten im Ausdruck oder ohne `}` → Syntaxfehler |
| Fehlende Variablen/Pfade | Internes `undefined`: jeder Vergleich ergibt `false` (auch `!=`); `undefined` ist nicht `null` |
| Truthiness | Bool direkt; `null`/`undefined`/`0`/leerer String → `false`, Arrays und Objekte → `true` |
| Zahlenvergleich | `f64`-Epsilon (`< f64::EPSILON`), wie zuvor |
| Arithmetik | Nur Zahlen; `undefined` propagiert, Division/Modulo durch Null und Typfehler → `false` + `tracing::warn!` |
| Deploy-Validierung | `ProcessDefinitionBuilder::build` prüft alle `SequenceFlow.condition` und `ComplexGateway.join_condition` → `EngineError::InvalidDefinition` mit Quelle/Ziel bzw. Gateway-ID |
