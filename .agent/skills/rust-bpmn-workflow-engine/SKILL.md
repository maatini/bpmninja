---
name: rust-bpmn-workflow-engine
description: Expert skill for building a minimal, production-ready BPMN 2.0 Workflow Engine in Rust with StartEvent, TimerStartEvent, EndEvent, ServiceTask and UserTask.
version: 1.0
triggers: ["bpmn", "workflow engine", "token execution", "timer task", "user task"]
author: Grok + Antigravity
tags: [rust, tokio, bpmn, state-machine, concurrency]
---

# RUST BPMN WORKFLOW ENGINE SKILL

## Core Expertise (always apply)
You are now an expert in building safe, idiomatic Rust workflow engines. Use token-based execution, async Tokio, strict compile-time safety, and follow all persistence rules when NATS is requested.

## Required Patterns (use exactly these)
1. **Model Layer**
   - Enum `BpmnElement` with variants: StartEvent, TimerStartEvent(Duration), EndEvent, ServiceTask, UserTask
   - Struct `Token { id: Uuid, current_node: String, variables: HashMap<String, serde_json::Value> }`
   - Struct `ProcessDefinition` with nodes + outgoing flows (HashMap)

2. **Engine Core**
   - `WorkflowEngine` with `HashMap<ProcessInstanceId, ProcessInstance>`
   - `execute(token: Token) -> Result<NextAction, EngineError>` method
   - Use `tokio::sync::mpsc` for task queue

3. **TimerStartEvent & Background Tasks**
   - Always use `tokio::spawn` with `tokio::time::interval` or `tokio::time::sleep` for background tasks (timers, NATS watchers).
   - Never block the Tokio runtime. On trigger: create ProcessInstance and start token.

4. **ServiceTask**
   - Registry of async handlers: `HashMap<String, Arc<dyn ServiceHandler>>`
   - Call with `handler.execute(variables).await`

5. **UserTask**
   - Store pending tasks in `Vec<PendingUserTask>` (or NATS KV `user_tasks` if persistence active)
   - Method `complete_user_task(id: String, variables: HashMap)` that resumes the token

## Persistence & NATS (When Requested)
If NATS/persistence is mentioned, you MUST strictly follow this architecture (`async-nats` with JetStream):
- **Object Store (`bpmn_xml`)**: Store original BPMN 2.0 XML (immutable).
- **KV Store (`definitions`)**: Store `ProcessDefinition` (JSON).
- **KV Store (`instances`)**: Store `ProcessInstance`, `Token`, and variables.
- **KV Store (`user_tasks`)**: Store pending UserTasks.
- **JetStream (`WORKFLOW_EVENTS`)**: Publish state changes atomically with KV writes (Subjects: `workflow.deploy`, `workflow.start`, `workflow.complete`, `workflow.timer`).

## Best Practices (mandatory)
- Rust 2024 edition. Keep state in-memory first, unless NATS is requested.
- Error Handling: Use `anyhow` + `thiserror`. Define `EngineError` enum (with `NatsError` variants if applicable).
- Anti-Patterns: NEVER use `.unwrap()`, `.expect()`, or `panic!` in production library code. Never ignore `cargo clippy` warnings.
- `tracing::info!` / `debug!` for every step.
- Unit tests with `#[tokio::test]` for every element. Minimum 85% coverage on core logic.

## Example Implementation Snippet (copy this style)
```rust
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Persistence error: {0}")]
    NatsError(String),
}

#[derive(Debug)]
pub enum NextAction {
    Continue(Token),
    WaitForUser(String),
    Complete,
}

pub async fn execute_step(&self, token: Token) -> Result<NextAction, EngineError> {
    let node = self.definition.get_node(&token.current_node)
        .ok_or_else(|| EngineError::NodeNotFound(token.current_node.clone()))?;

    tracing::debug!(node_id = %token.current_node, "Executing step");

    match node {
        BpmnElement::TimerStartEvent(dur) => { /* timer logic */ }
        BpmnElement::ServiceTask(name) => { self.call_service(name, &token).await }
        BpmnElement::UserTask => { Ok(NextAction::WaitForUser(token.id.to_string())) }
        _ => { /* default token flow logic */ }
    }
}
```
