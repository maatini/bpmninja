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
You are now an expert in building safe, idiomatic Rust workflow engines. Use token-based execution, async Tokio, and strict compile-time safety.

## Required Patterns (use exactly these)
1. **Model Layer**
   - Enum `BpmnElement` with variants: StartEvent, TimerStartEvent(Duration), EndEvent, ServiceTask, UserTask
   - Struct `Token { id: Uuid, current_node: String, variables: HashMap<String, serde_json::Value> }`
   - Struct `ProcessDefinition` with nodes + outgoing flows (HashMap)

2. **Engine Core**
   - `WorkflowEngine` with `HashMap<ProcessInstanceId, ProcessInstance>`
   - `execute(token: Token) -> Result<NextAction>` method
   - Use `tokio::sync::mpsc` for task queue

3. **TimerStartEvent**
   - Spawn `tokio::task` with `tokio::time::interval` or `sleep`
   - On trigger: create new ProcessInstance and start token

4. **ServiceTask**
   - Registry of async handlers: `HashMap<String, Arc<dyn ServiceHandler>>`
   - Call with `handler.execute(variables).await`

5. **UserTask**
   - Store pending tasks in `Vec<PendingUserTask>`
   - Method `complete_user_task(id: String, variables: HashMap)` that resumes the token

## Best Practices (mandatory)
- Rust 2024 edition
- `anyhow` + `thiserror` for errors (no unwrap in engine)
- `tracing::info!` / `debug!` for every step
- Unit tests with `#[tokio::test]` for every element
- After every change: `cargo clippy --all-targets -- -D warnings`
- Keep state in-memory first (later add persistence)

## Example Implementation Snippet (copy this style)
```rust
#[derive(Debug)]
enum NextAction {
    Continue(Token),
    WaitForUser(String),
    Complete,
}

async fn execute_step(&self, token: Token) -> Result<NextAction> {
    match &self.definition.get_node(&token.current_node) {
        BpmnElement::TimerStartEvent(dur) => { /* timer logic */ }
        BpmnElement::ServiceTask(name) => { self.call_service(name, &token).await }
        BpmnElement::UserTask => { Ok(NextAction::WaitForUser(token.id.to_string())) }
        _ => { /* token flow */ }
    }
}
