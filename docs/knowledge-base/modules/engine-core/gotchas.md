# engine-core — Gotchas

## Critical Rules

### ⚠️ Never hold a lock across `.await`

```rust
// ❌ FORBIDDEN — will deadlock
let inst = instance_arc.write().await;
self.some_async_persistence_call().await; // DEADLOCK!

// ✅ CORRECT
let persistence_call;
{
    let mut inst = instance_arc.write().await;
    persistence_call = self.persistence.save_instance(&inst).await; // ALSO WRONG — still inside lock
}
// MUST drop lock first, THEN await:
let persistence_call;
{
    let mut inst = instance_arc.write().await;
    inst.state = InstanceState::Running;
} // Lock dropped here
self.persistence.save_instance(&updated_inst).await; // SAFE
```

### ⚠️ Tokens live in ONE place

Pending tasks hold `token_id: Uuid`, NOT copies of `Token`. When resuming:
1. Remove token from `instance.tokens.get(&token_id)`
2. Pass it to `run_instance_batch`
3. When pausing again, store it back in `instance.tokens`

Never clone a token and hold two copies — they'll diverge.

### ⚠️ `BpmnElement` is a closed enum — must be exhaustive

All 29 variants must be handled. When adding a new element:
1. Add variant to `BpmnElement` in `domain/element.rs`
2. Handle in `engine/executor/mod.rs` → `execute_step`
3. Handle in `engine/handlers/` → appropriate handler file
4. Update `bpmn-parser/src/parser.rs` to parse the XML into the new variant

### ⚠️ Script execution limits

Rhai scripts are sandboxed with limits:
- `max_operations: 50,000` (default) — configurable via `RHAI_MAX_OPERATIONS`
- `max_memory: 2 MiB` (default) — configurable via `RHAI_MAX_MEMORY_BYTES`; Rhai has no total-heap API, so this budget derives `set_max_string_size` / `set_max_array_size` / `set_max_map_size`
- `timeout_ms: 1,000` (default) — configurable via `RHAI_TIMEOUT_MS`

Heavy scripts will be killed by timeout or collection-size limits. The error is recoverable — just means the script didn't complete.

### ⚠️ History snapshots

Snapshots are taken every 8 audit entries. The `calculate_diff` function compares previous and current `ProcessInstance` state. For large variable maps, values > 1KB are truncated in the diff.

### ⚠️ Persistence is optional in the engine; required in production server

`WorkflowEngine.persistence` is `Option<Arc<dyn WorkflowPersistence>>`. The engine runs fine without persistence (in-memory / unit tests). Server code in `engine-server/main.rs`:
- NATS connect OK → attach `NatsPersistence` + restore
- NATS fail + `REQUIRE_NATS=false` → in-memory with error log (dev only)
- NATS fail + `REQUIRE_NATS=true` → **process exits** (docker-compose default)

### ⚠️ Retry queue is bounded

Failed persistence ops are enqueued on a bounded channel (default capacity 10 000, `PERSISTENCE_RETRY_QUEUE_CAPACITY`). When full, jobs are **dropped** and counted (`bpmn_persistence_retry_dropped_total`) instead of growing memory unboundedly. Call `engine.shutdown()` before dropping the engine so the worker can flush remaining jobs.

### ⚠️ Event channel capacity

The broadcast channel has capacity 256. Slow SSE consumers may miss events if they don't keep up — this is by design (no backpressure on engine execution).

### ⚠️ Instance suspend/resume

When suspended:
- Timers don't fire (filtered out in `process_timers`)
- Tasks can't be completed (blocked in `complete_user_task`, `complete_service_task`)
- Variables CAN still be updated
- Token CAN still be moved

The previous state is stored in `InstanceState::Suspended { previous_state: Box<InstanceState> }`.

### ⚠️ Parallel gateway joins

`JoinBarrier` stores arriving tokens. When the last token arrives:
- Variables from all tokens are merged (later tokens override earlier ones)
- The merged token gets `is_merged = true` flag
- The barrier is removed from `instance.join_barriers`

### ⚠️ Instance migration

Migration changes the `definition_key` and optionally remaps node IDs. The `node_mapping: HashMap<String, String>` provides old→new node translations. If a node ID exists in the old definition but is missing in the new one (and no mapping exists), migration fails with `EngineError::OrphanedToken`.

### ⚠️ Backward-compatible re-exports

`engine-core/src/lib.rs` has legacy re-exports:
- `use domain as model`
- `use domain::timer as timer_definition`
- `use port as persistence`

These exist for backward compatibility. New code should use the canonical paths (`engine_core::domain::*`, `engine_core::port::*`).
