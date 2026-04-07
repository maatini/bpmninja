---
trigger: file_match
file_patterns: ["engine-core/**"]
---

# Execution Engine Agent
- **Domain:** `engine-core/`
- **Role:** Pure state machine — token advancement, `BpmnElement` dispatch, gateway routing, condition evaluation, script execution, timer/message/error boundary events.
- **Specification:** See `BPMN_WORKFLOW_ENGINE.md` for the full element specification, architecture, and module reference.

## Essential Rules
- No network code (no NATS, no HTTP). All I/O goes through the `WorkflowPersistence` trait.
- Run timers via `tokio::time`. Define traits for everything external.
- The engine uses internal DashMap-based concurrency — no global `RwLock` on the instance store.
- `ProcessDefinition` is immutable after construction and shared via `Arc<ProcessDefinition>`.
- `EngineError` (in `error.rs`) is the single error type — extend it for new failure modes.
