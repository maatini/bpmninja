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
- NATS fail + `REQUIRE_NATS=false` → in-memory with error log (explicit local opt-in)
- NATS fail + `REQUIRE_NATS=true` (default) → **process exits**

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

### ⚠️ Inclusive gateway join uses the split's taken-path count

`execute_inclusive_gateway` splits on every outgoing flow whose condition is true (BPMN OR). A join (`incoming_count >= 2`, `WaitForJoin`) still uses `JoinBarrier`. `ContinueMultiple` registers `expected_count = branch_count` on the downstream join (`find_downstream_join` + same gateway type). A 1-of-N *split* (`is_split_gateway`, outgoing ≥ 2) returns `Continue` and registers `expected_count = 1`. Join-only Inclusive nodes do not register, so a nested inner join cannot overwrite the outer join's count. `arrive_at_join` honors that count. Unstructured joins without a matching downstream join still fall back to structural `incoming_flow_count` and can stall.

Re-registering a barrier updates `expected_count` and keeps `arrived_tokens` (no wipe).

`ComplexGateway` uses the same barrier, plus an optional `join_condition` (`activationCondition`) evaluated on the merged variables of tokens that have already arrived — the join can fire early when that condition is true.

### ⚠️ Exclusive gateway takes unconditional non-default flows

XOR split: first true condition wins; then an unconditional flow whose target is not the default; then the default. A bpmn-js merge (one unconditional outgoing, no `@default`) therefore completes. Split with only failing conditions and no default still yields `NoMatchingCondition`.

### ⚠️ Call-Activity tracking is `outstanding_calls`, not only `WaitingOnCallActivity`

Parallel Call-Activities keep `InstanceState::ParallelExecution`. Resume looks up the child in `ProcessInstance.outstanding_calls` (keyed by child instance id). Spawn failure or missing called element marks the parent `CompletedWithError` (`CALL_ACTIVITY_SPAWN_FAILED` / `CALL_ACTIVITY_TARGET_NOT_FOUND`) instead of hanging.

### ⚠️ Service-task topic index is secondary

`pending_service_tasks` is SSOT. `service_task_topic_index` maps topic → task ids for `fetch_and_lock`. Insert into the map first, then the index. Orphan index entries are dropped at fetch. Incidents (`retries <= 0`) stay in the index but are not locked.

### ⚠️ Timer due-index is secondary

`pending_timers` is SSOT. `timer_due_index` is a `BTreeMap<(expires_at, id)>` so `process_timers` only walks due IDs. Insert/remove/retain go through `insert_pending_timer` / `remove_pending_timer` / `retain_pending_timers`. Changing `expires_at` in place without `set_timer_expiry` leaves the index stale and the timer will not fire.

### ⚠️ `list_instances_page` clones only the page

Live listing sorts by `started_at` desc, then instance id. `GET /api/instances` without query still returns the full JSON array. With `limit`/`offset`, the body is the page and `X-Total-Count` is set (`limit` clamped 1..=1000).

### ⚠️ Instance migration

Migration changes the `definition_key` and optionally remaps node IDs. The `node_mapping: HashMap<String, String>` provides old→new node translations. If a node ID exists in the old definition but is missing in the new one (and no mapping exists), migration fails with `EngineError::OrphanedToken`.

### ⚠️ Backward-compatible re-exports

`engine-core/src/lib.rs` has legacy re-exports:
- `use domain as model`
- `use domain::timer as timer_definition`
- `use port as persistence`

These exist for backward compatibility. New code should use the canonical paths (`engine_core::domain::*`, `engine_core::port::*`).
