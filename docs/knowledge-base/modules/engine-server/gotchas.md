# engine-server — Gotchas

### ⚠️ Route handlers MUST NOT hold engine locks across `.await`

The `AppState.engine` is `Arc<WorkflowEngine>`. All engine methods take `&self`. However, if a route handler acquires a lock from engine internals (e.g., `instances.get(&id).await`), it must drop that lock before any other `.await` point.

This is especially relevant in:
- `tasks.rs` — fetchAndLock via long polling loops
- `instances.rs` — multi-step operations (suspend + persist)
- `history.rs` — querying multiple persistence operations

### ⚠️ SSE is broadcast, NOT queue

`GET /api/events` uses `tokio::sync::broadcast::Receiver`. If a client is slow (channel capacity 256), it **will miss events**. The protocol is: event fires → client re-fetches via REST. No retry or catch-up mechanism.

### ⚠️ CORS is wide open (`Any` origin)

The development setup allows all origins. For production deployment behind a specific domain, replace with a restrictive CORS policy.

### ⚠️ Body size limit is 5 MB

BPMN XML deployments are capped at 5 MB. File uploads for instance variables go through multipart and are NOT subject to this limit (files route handles `multipart/form-data` separately).

### ⚠️ Startup restore can take time

On server start with large NATS state, `StartupCoordinator.restore()` loads all definitions and instances synchronously (within the async context). Very large deployments may delay server readiness. The `/api/ready` endpoint returns `503` until NATS connection is verified.

### ⚠️ Log buffer is rolling (5,000 entries)

Oldest entries are dropped when the buffer exceeds 5,000. NATS persistence (`ENGINE_LOGS` stream) stores 50,000 entries. The file fallback (`engine_logs.jsonl`) is NOT truncated — it grows indefinitely (mitigated by `LOG_FILE=off` option).

### ⚠️ In-memory fallback is silent

If NATS is unavailable, the server starts in in-memory mode with just a warning log. All state is lost on restart. This is by design for development, but prevent this in production.

### ⚠️ Timer scheduler uses `tokio::spawn`

The timer background task is spawned via `tokio::spawn`. If the main event loop panics, timer processing stops. The task is gracefully shut down on `Ctrl+C` via a `watch::channel`.

### ⚠️ Prometheus handle is optional

The `/metrics` endpoint is only mounted if `prometheus_handle` is `Some`. No handle = no `/metrics` route at all (not even a 404).

### ⚠️ Adding a new route

1. Add the handler function in the appropriate `server/` module
2. Register the route in `server/mod.rs` → `build_app_with_engine`
3. Update `AppError::IntoResponse` if new error variants need HTTP mapping
4. Add an E2E test in `engine-server/tests/`
5. Update `docs/openapi.yaml` (API spec)
