---
name: engine-server
description: Skill for the engine-server crate — building the Axum REST API adapter for the workflow engine.
version: 2.0
triggers: ["server", "api", "rest", "axum", "http", "engine-server"]
author: Antigravity
tags: [rust, axum, rest-api]
---

# ENGINE SERVER SKILL
## Crate: `engine-server`

Axum-based HTTP REST API adapter. All business logic lives in `engine-core`; this crate is purely the HTTP layer.

## REST API Endpoints

### Definitions
- `POST /api/deploy` — Deploy a BPMN definition (XML body)
- `GET /api/definitions` — List all deployed definitions
- `GET /api/definitions/:id/xml` — Get original BPMN XML for a definition
- `DELETE /api/definitions/:id` — Delete a definition (`?cascade=true` to also delete instances)

### Instances
- `POST /api/start` — Start a new process instance
- `GET /api/instances` — List all process instances
- `GET /api/instances/:id` — Get details of a single instance
- `PUT /api/instances/:id/variables` — Update instance variables at runtime
- `DELETE /api/instances/:id` — Delete a process instance

### User Tasks
- `GET /api/tasks` — List all pending user tasks
- `POST /api/complete/:id` — Complete a user task

### Service Tasks (Camunda-style external tasks)
- `POST /api/service-task/fetchAndLock` — Fetch and lock tasks for a worker (supports long-polling)
- `POST /api/service-task/:id/complete` — Complete a service task with result variables
- `POST /api/service-task/:id/failure` — Report task failure (with retries)
- `POST /api/service-task/:id/extendLock` — Extend lock duration
- `POST /api/service-task/:id/bpmnError` — Report a BPMN error

### History
- `GET /api/instances/:id/history` — Query instance history (filterable by event_types, actor_types)

### Monitoring
- `GET /api/info` — Engine stats and backend info

## Error Handling
- `AppError` enum maps `EngineError` variants to HTTP status codes:
  - `400` — Invalid input (bad XML, invalid variables)
  - `404` — `NoSuchInstance`, `NoSuchDefinition`, `ServiceTaskNotFound`
  - `409` — `DefinitionHasInstances`, `ServiceTaskLocked`
  - `500` — Internal / persistence errors

## Rules
- Keep business logic in `engine-core`. The server is an adapter only.
- Map `EngineError` to appropriate HTTP status codes via `AppError`.
- Use `serde_json` for request/response serialization.
- All handlers are async and use `State<Arc<Mutex<WorkflowEngine>>>`.
