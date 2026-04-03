---
name: engine-core
description: Skill for the engine-core crate — token-based BPMN execution, gateway routing, condition evaluation, and Rhai script execution.
version: 2.0
triggers: ["engine", "token", "gateway", "condition", "script", "execute_step", "bpmn", "workflow engine"]
author: Antigravity
tags: [rust, bpmn, state-machine, execution]
---

# ENGINE CORE SKILL

## Crate: `engine-core`
Pure state machine — no network code, no NATS, no HTTP. Built with Tokio.

## Module Structure
| File | Purpose |
|---|---|
| `model.rs` | `BpmnElement`, `Token`, `ProcessDefinition`, `SequenceFlow`, `ExecutionListener` |
| `engine.rs` | `WorkflowEngine`, `ProcessInstance`, `InstanceState`, `NextAction` |
| `engine/service_task.rs` | External task ops (fetch-and-lock, complete, fail, BPMN error) |
| `condition.rs` | `evaluate_condition()` — condition evaluator for gateway routing |
| `script_runner.rs` | Rhai execution listeners (start/end scripts) |
| `persistence.rs` | `WorkflowPersistence` trait definition |
| `history.rs` | `HistoryEntry`, `HistoryEventType`, `calculate_diff()` |
| `error.rs` | `EngineError` enum, `EngineResult<T>` alias |
| `engine/tests.rs`| Comprehensive integration tests |
| `lib.rs` | Public re-exports (including `EngineStats`) |

## Supported BPMN Elements
- **StartEvent** / **TimerStartEvent(Duration)**
- **EndEvent**
- **ServiceTask** (Camunda-style fetch-and-lock)
- **UserTask**
- **ExclusiveGateway** (XOR split)
- **InclusiveGateway** (OR split)

## Key Design Decisions
- `Arc<ProcessDefinition>` for shared definitions
- Token-based execution (`execute_step()` -> `NextAction`)
- Script listeners embedded via `rhai::Engine`
- `thiserror` for `EngineError` (no unwraps in lib code!)
