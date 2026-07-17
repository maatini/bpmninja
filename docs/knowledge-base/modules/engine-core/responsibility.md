# engine-core — Responsibilities

## What engine-core Owns

1. **@tag:bpmn-domain-model** — All BPMN domain types: `BpmnElement` (29 variants), `ProcessDefinition`, `Token`, `SequenceFlow`, `TimerDefinition`, `MultiInstanceDef`, execution listeners.
2. **@tag:engine-state** — `WorkflowEngine` and all runtime state: definitions (DefinitionRegistry), instances (InstanceStore), four wait-state queues (DashMaps), event broadcast channel.
3. **@tag:token-execution** — Token-based execution loop: `run_instance_batch`, `execute_step`, `NextAction` dispatch logic.
4. **@tag:gateway-routing** — XOR, AND, OR, EventBased, and Complex gateway evaluation and token fork/join.
5. **@tag:element-handlers** — Per-`BpmnElement` execution: tasks (User, Service, Script, Send), events (Start, End, Timer, Message, Error, Escalation, Compensation), boundaries, call activities, sub-processes.
6. **@tag:rhai-scripting** / **@tag:rhai-sandbox** — Sandboxed Rhai script execution for execution listeners and ScriptTasks. Limits: max operations, max_memory budget (derives string/array/map caps; Rhai has no heap API), wall-clock timeout.
7. **@tag:history-audit** — Audit trail generation with automatic diff calculation, snapshots every 8 entries, actor tracking.
8. **@tag:persistence-port** — `WorkflowPersistence` trait defining the persistence contract.
9. **@tag:retry-queue** / **@tag:fault-tolerant-retry** — Two-stage persistence retry: inline (2× ~50ms) + **bounded** background worker (`mpsc::channel`, default 10 000; drop + `bpmn_persistence_retry_dropped_total` when full; max 50 retries with exponential backoff).
10. **@tag:engine-events** — Broadcast channel for state-change events (`EngineEvent` enum).

## Invariants (Must Always Be True)

1. **No I/O in engine-core**: engine-core does not import any network, filesystem, or database crates. Only `async-nats` types appear indirectly through the `WorkflowPersistence` trait.
2. **No `.unwrap()` in production code**: All fallible operations return `EngineResult<T>`. Only tests use `.unwrap()`.
3. **`ProcessDefinition` is immutable after deploy**: Stored as `Arc<ProcessDefinition>` in `DefinitionRegistry`.
4. **Tokens live in one place**: `ProcessInstance.tokens: HashMap<Uuid, Token>`. Pending tasks hold only `token_id: Uuid`.
5. **No lock held across `.await`**: All `RwLock` and `DashMap` access must be scoped before any `.await` call.
6. **`BpmnElement` exhaustive matching**: All 29 variants must be handled in `execute_step` and handler functions.
7. **All mutations via `&self`**: `WorkflowEngine` methods use `&self` (interior mutability via Arc + DashMap/RwLock).
8. **History generated for every state transition**: Every token move, task completion, variable update, error must produce a `HistoryEntry`.
9. **Persistence operations go through retry**: Direct NATS calls only in `persistence_ops.rs`; engine logic dispatches `PersistJob` to retry queue.

## Key Entry Points

| Entry Point | Returns | Description |
|------------|---------|-------------|
| `WorkflowEngine::new()` | `Self` | Creates empty engine with script config from env |
| `.with_persistence(p)` | `Self` | Attaches persistence + starts retry worker |
| `.deploy_definition(def)` | `(Uuid, i32)` | Register a `ProcessDefinition` → versioning |
| `.start_instance(key, vars, biz_key, parent)` | `ProcessInstance` | Create and run a new instance |
| `.start_instance_latest(bpmn_id, vars, biz_key)` | `ProcessInstance` | Start latest version by BPMN process ID |
| `.start_timer_instance(key, vars, biz_key)` | `ProcessInstance` | Start a timer-triggered instance |
| `.complete_user_task(task_id, variables)` | `()` | Complete a user task → resume instance |
| `.fetch_and_lock_service_tasks(req)` | `Vec<PendingServiceTask>` | Fetch and lock service tasks (long polling) |
| `.complete_service_task(id, vars)` | `()` | Complete a service task → resume instance |
| `.fail_service_task(id, msg, details, retries)` | `()` | Mark a task as failed / create incident |
| `.bpmn_error(task_id, error_code)` | `()` | Route to BoundaryErrorEvent |
| `.process_timers()` | `usize` | Process all expired timers (called by server background task) |
| `.correlate_message(name, biz_key, vars)` | `Vec<Uuid>` | Correlate an incoming message with waiting catches |
| `.suspend_instance(id)` / `.resume_instance(id)` | `()` | Pause/resume an instance |
| `.move_token(id, token_id, target_node)` | `()` | Manually move a token to a different node |
| `.migrate_instance(id, new_key, mapping)` | `()` | Migrate to a different definition version |
| `.update_variables(id, vars)` | `()` | Update instance variables at runtime |
| `.shutdown()` | `()` | Gracefully shut down retry worker |
| `.subscribe_events()` | `broadcast::Receiver` | Subscribe to engine event stream |
| `.get_engine_stats()` | `EngineStats` | Monitoring stats (instances, tasks, timers, etc.) |

## Internal Module Responsibilities

| Module | Path | Responsibility |
|--------|------|---------------|
| `domain` | `src/domain/` | All data types — definitions, elements, tokens, errors, timers, flows, listeners |
| `engine/mod.rs` | `src/engine/mod.rs` | `WorkflowEngine` struct definition, constructor, persistence setup, shutdown |
| `executor/` | `src/engine/executor/` | Core execution loop: `run_instance_batch`, `execute_step`, `handle_next_action`, completion |
| `handlers/` | `src/engine/handlers/` | Element-specific logic: `handle_service_task`, `handle_boundary_timer`, etc. |
| `gateway.rs` | `src/engine/gateway.rs` | Gateway evaluation: XOR (conditions), AND (fork/join), OR (multi-fork), EventBased, Complex |
| `boundary.rs` | `src/engine/boundary.rs` | Boundary event setup and registration |
| `user_task.rs` | `src/engine/user_task.rs` | User task creation, completion, listing |
| `service_task.rs` | `src/engine/service_task.rs` | Service task fetch-and-lock, complete, failure, bpmnError, lock extension |
| `timer_processor.rs` | `src/engine/timer_processor.rs` | Timer scanning, expiry processing, repeating timer setup |
| `message_processor.rs` | `src/engine/message_processor.rs` | Message correlation logic |
| `process_start.rs` | `src/engine/process_start.rs` | Instance creation from various start triggers |
| `definition_ops.rs` | `src/engine/definition_ops.rs` | Definition CRUD, list, versioning, deploy |
| `instance_ops.rs` | `src/engine/instance_ops.rs` | Instance CRUD, suspend, resume, migrate, move token |
| `persistence_ops.rs` | `src/engine/persistence_ops.rs` | Save/restore orchestrator (dispatches PersistJobs) |
| `registry.rs` | `src/engine/registry.rs` | `DefinitionRegistry` — immutable definition store (`Arc<RwLock<HashMap>>`) |
| `instance_store.rs` | `src/engine/instance_store.rs` | `InstanceStore` — per-instance locking (`Arc<RwLock<HashMap>>`) |
| `retry_queue.rs` | `src/engine/retry_queue.rs` | Bounded retry: `PersistJob`, `create_retry_queue_with_capacity`, background worker, drop metrics |
| `events.rs` | `src/engine/events.rs` | `EngineEvent` enum + broadcast sender |
| `history/mod.rs` | `src/history/mod.rs` | `HistoryEntry`, `HistoryDiff`, `VariableDiff`, `calculate_diff`, `calculate_diff_from_snapshot` |
| `scripting/` | `src/scripting/` | `ScriptConfig`, `build_engine`, `derived_collection_limits`, `execute_script_safe` / listeners |
| `runtime/` | `src/runtime/` | `ProcessInstance`, `InstanceState`, `NextAction`, pending types, `ActiveToken`, `JoinBarrier` |
| `condition.rs` | `src/condition.rs` | Gateway condition evaluation: `evaluate_condition(expr, vars) -> bool` |
