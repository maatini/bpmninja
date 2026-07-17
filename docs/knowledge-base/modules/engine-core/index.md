# engine-core

The heart of BPMNinja — a **pure state machine** with no network or I/O code. Implements BPMN 2.0 token-based execution, gateways, scripting, and history.

**Crate path:** `engine-core/`  
**Source:** `engine-core/src/` (25+ lib files, ~7,000 LoC)  
**Tests:** ~217 lib tests + 5 integration (BPMN compliance / complex gateways); includes Rhai memory + bounded retry tests

- [responsibility.md](responsibility.md) — What engine-core owns, invariants, entry points
- [dependencies.md](dependencies.md) — Inbound/outbound dependencies, trait relationships
- [interfaces.md](interfaces.md) — Public API surface: structs, enums, traits, functions
- [gotchas.md](gotchas.md) — Known pitfalls, concurrency rules, common mistakes

## Quick Reference

| Module | Path | Purpose |
|--------|------|---------|
| `domain/` | `src/domain/` | Core types: `ProcessDefinition`, `BpmnElement`, `Token`, `EngineError`, `TimerDefinition` |
| `engine/` | `src/engine/` | `WorkflowEngine` + all sub-components (executor, gateways, handlers, retry queue) |
| `engine/executor/` | `src/engine/executor/` | Execution loop: `run_instance_batch`, `execute_step`, `handle_next_action` |
| `engine/handlers/` | `src/engine/handlers/` | Per-BpmnElement-variant logic (tasks, events, gateways, sub-processes) |
| `port/` | `src/port/` | `WorkflowPersistence` trait (hexagonal port) |
| `history/` | `src/history/` | Audit trail, diff calculation, `HistoryEntry`, `HistoryDiff` |
| `scripting/` | `src/scripting/` | Rhai sandbox: ops / memory budget (collection caps) / timeout |
| `runtime/` | `src/runtime/` | `ProcessInstance`, `InstanceState`, `PendingUserTask`, `PendingServiceTask`, `NextAction` |
| `adapter/` | `src/adapter/` | In-memory persistence implementation for testing |
| `condition.rs` | `src/condition.rs` | Expression evaluator for gateway conditions |
