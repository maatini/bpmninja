---
name: engine-server
description: Skill for the engine-server crate — building the Axum REST API adapter for the workflow engine.
version: 1.0
triggers: ["server", "api", "rest", "axum", "http", "engine-server"]
author: Antigravity
tags: [rust, axum]
---

# ENGINE SERVER SKILL
## Crate: `engine-server`

## Known Endpoints
- `POST /deploy`, `GET /definitions`
- `POST /instances`, `GET /instances`, `GET /instances/:id`
- `GET /tasks/user`, `POST /tasks/user/:id/complete`
- `GET /tasks/service`, `POST /tasks/service/:id/complete`
- `GET /info`

## Rules
- Keep business logic in `engine-core`. The server is an adapter.
- Map `EngineError` to appropriate HTTP status codes (e.g. 404 for `NoSuchInstance`).
