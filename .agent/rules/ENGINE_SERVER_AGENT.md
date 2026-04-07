---
trigger: file_match
file_patterns: ["engine-server/**"]
---

# Server Agent
- **Domain:** `engine-server/`
- **Role:** Implements the Axum HTTP REST API for the engine.

## Architecture
- `engine-server` is a thin HTTP adapter. Keep all business logic in `engine-core`.
- Use `axum::State<Arc<WorkflowEngine>>` for shared state.
- Serialize/Deserialize via `serde_json`.

## API Conventions
- RESTful endpoints under `/api/v1/` prefix.
- Response format: JSON. Successful responses return the resource directly. Errors return `{ "error": "<message>" }`.
- HTTP status codes: `200` success, `201` created, `404` not found, `400` bad request, `409` conflict, `500` internal error.
- Use `axum::extract::Path` for resource IDs, `axum::extract::Json` for request bodies.

## Middleware
- CORS: Allow all origins in dev, configurable in production.
- Logging: Use `tower_http::trace::TraceLayer` for request/response tracing.

## Rules
- One handler function per endpoint, grouped in separate modules by resource (definitions, instances, tasks).
- Error mapping: Convert `EngineError` variants to appropriate HTTP status codes.
- Do NOT implement background workers or timers here — those belong in `engine-core`.
