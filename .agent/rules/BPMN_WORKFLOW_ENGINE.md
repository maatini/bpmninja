---
trigger: always_on
---

# BPMN_WORKFLOW_ENGINE.md - Project Specification

## Minimal Requirements
Implement exactly these elements:
- StartEvent (plain)
- TimerStartEvent (with duration or cron-like)
- EndEvent
- ServiceTask (executes registered handler function)
- UserTask (creates pending task, waits for external complete call)

## Architecture (must follow)
1. **Model**
   - BpmnElement enum (Start, End, ServiceTask, UserTask, etc.)
   - ProcessDefinition (parsed graph)
   - Token { id, current_node, variables: HashMap<String, Value> }

2. **Engine**
   - WorkflowEngine struct with active ProcessInstances
   - execute_step(token) method
   - Timer scheduler (tokio task)

3. **Tasks**
   - ServiceTask: async fn call
   - UserTask: store in pending_tasks queue, support complete_task(id, variables)

Start simple (in-memory). Later add persistence.

Prioritize correctness of token flow and timer triggering first.
