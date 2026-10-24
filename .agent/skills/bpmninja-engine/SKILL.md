---
name: bpmninja-engine
description: Skill for the engine-core crate covering hexagonal architecture, token-based BPMN execution, gateway routing, scripting, and BPMN 2.0 compliance.
version: 2.0.0
tags: [rust, engine-core, bpmn, token-engine, concurrency, hexagonal]
requires: [cargo]
---

# BPMNinja Engine Skill

## When to Activate
Activate whenever you work on the `engine-core` crate:
- Adding or modifying BPMN elements (gateways, events, tasks, sub-processes)
- Token execution, state machine, or concurrency changes
- Rhai scripting or condition evaluator improvements
- History tracking or persistence trait modifications
- Performance benchmarks or test coverage improvements

## Scope & File Map

```
engine-core/src/
├── lib.rs                    # Public API, re-exports, backward-compat aliases
├── condition.rs              # Condition evaluator (expressions for gateways)
├── domain/                   # Pure domain models (no I/O)
│   ├── mod.rs
│   ├── definition.rs         # ProcessDefinition
│   ├── element.rs            # BpmnElement (geschlossener Enum)
│   ├── error.rs              # EngineError
│   ├── file_ref.rs           # File reference model
│   ├── flow.rs               # SequenceFlow
│   ├── incident.rs           # Incident model
│   ├── listener.rs           # Execution listeners
│   ├── multi_instance.rs     # Multi-instance config
│   ├── timer.rs              # Timer definitions (ISO 8601, cron)
│   ├── token.rs              # Token model
│   └── tests.rs              # Domain model tests
├── port/                     # Trait boundaries (hexagonal ports)
│   ├── mod.rs
│   └── persistence.rs        # WorkflowPersistence trait
├── adapter/                  # Trait implementations
│   ├── mod.rs
│   └── in_memory.rs          # InMemoryPersistence (for tests)
├── engine/                   # Application services
│   ├── mod.rs                # WorkflowEngine main struct
│   ├── boundary.rs           # Boundary event handling
│   ├── definition_ops.rs     # Definition management operations
│   ├── events.rs             # Event handling
│   ├── gateway.rs            # Gateway routing logic
│   ├── incidents.rs          # Incident handling
│   ├── instance_ops.rs       # Instance lifecycle operations
│   ├── instance_store.rs     # DashMap-based concurrent store
│   ├── message_processor.rs  # Message event processing
│   ├── persistence_ops.rs    # Persistence integration
│   ├── process_start.rs      # Process instantiation logic
│   ├── registry.rs           # Definition registry
│   ├── retry_queue.rs        # Retry queue for failed operations
│   ├── service_task.rs       # Service task execution
│   ├── timer_index.rs        # Timer due index
│   ├── timer_processor.rs    # Timer event processing
│   ├── topic_index.rs        # Service-task topic index
│   ├── user_task.rs          # User task handling
│   ├── executor/             # Token execution engine
│   ├── handlers/             # Element-specific handlers
│   │   ├── events.rs         # Event handlers
│   │   ├── gateways.rs       # Gateway handlers
│   │   ├── sub_processes.rs  # Sub-process handlers
│   │   └── tasks.rs          # Task handlers
│   └── tests/                # Engine integration tests
├── runtime/                  # Runtime infrastructure
│   ├── mod.rs
│   ├── constants.rs          # Runtime constants
│   ├── instance.rs           # RuntimeInstance (DashMap-based)
│   ├── pending.rs            # Pending token management
│   └── stats.rs              # Runtime statistics
├── history/                  # Audit trail
│   └── (HistoryEntry, HistoryDiff, VariableDiff)
└── scripting/                # Rhai script engine
    └── mod.rs                # ScriptEngine, evaluate_script
```

## Domain Rules & Patterns

1. **Hexagonal Architecture**: Domain models in `domain/` MUST NOT depend on adapters or engine. Communication across boundaries only via traits in `port/`.
2. **Backward Compatibility**: `lib.rs` re-exports all public types. Legacy aliases (`model`, `persistence`, `timer_definition`) must be preserved until downstream crates migrate.
3. **Concurrency**: `instance_store.rs` uses `DashMap` for lock-free concurrent access. Never introduce global `Mutex`/`RwLock` on the instance store.
4. **Token Semantics**: Every BPMN element consumes input tokens and produces output tokens. Gateway routing is deterministic for the same input state.
5. **Error Handling**: Use `EngineError` from `domain/error.rs`. Never unwrap in production code – use `?` or explicit error mapping.
6. **Cross-Crate**: Follow the implementation order in `AGENTS.md` – engine-core changes come FIRST, then parser, persistence, server, desktop.

## Verifikation

```bash
cargo clippy -p engine-core --all-targets -- -D warnings
cargo test -p engine-core
```

## Common Pitfalls
- Only theoretical improvements without running clippy and tests
- Ignoring multi-threading edge cases in DashMap (e.g., deadlocks with nested entries)
- Rhai scripts without error handling (script panics crash the engine)
- Breaking backward-compat re-exports when moving modules
- Performance regressions in token execution hot path

