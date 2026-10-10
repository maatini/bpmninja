# engine-server — Gotchas

### ⚠️ Route handlers MUST NOT hold engine locks across `.await`

The `AppState.engine` is `Arc<WorkflowEngine>`. All engine methods take `&self`. However, if a route handler acquires a lock from engine internals (e.g., `instances.get(&id).await`), it must drop that lock before any other `.await` point.

This is especially relevant in:
- `tasks.rs` — fetchAndLock via long polling loops
- `instances.rs` — multi-step operations (suspend + persist)
- `history.rs` — querying multiple persistence operations

### ⚠️ SSE is broadcast, NOT queue

`GET /api/events` uses `tokio::sync::broadcast::Receiver`. If a client is slow (channel capacity 256), it **will miss events**. The protocol is: event fires → client re-fetches via REST. No retry or catch-up mechanism.

### ⚠️ CORS is an allowlist (not `Any` by default)

`CORS_ORIGINS` is a comma-separated origin list. **Unset** → `http://localhost:1420,http://127.0.0.1:1420,tauri://localhost,https://tauri.localhost`. Explicit `CORS_ORIGINS=*` is the only way to allow any origin (dev escape hatch). `build_app()` uses the default allowlist, never silent `Any`.

Allowed methods: GET, POST, PUT, DELETE, OPTIONS. Allowed headers: `Authorization`, `Content-Type`, `Accept`, `X-API-Key`.

### ⚠️ Optional API-key auth (`BPMNINJA_API_KEY`)

When `BPMNINJA_API_KEY` is set, every route except `GET /api/health` and `GET /api/ready` requires `Authorization: Bearer <key>` **or** `X-API-Key: <key>`. Missing/wrong key → **401** `{ "error": "Unauthorized" }`. Unset → no auth (E2E/`build_app()` stay green). `/metrics` and `/api/events` are protected when a key is configured. Tests must inject the key via `AppBuildConfig.api_key`, never via process env (parallel-test races).

### ⚠️ Body size limit is 10 MiB for XML/JSON

`DefaultBodyLimit` and the deploy-path cap share `MAX_XML_BYTES` (**10 MiB** = `10 * 1024 * 1024`, OpenAPI contract). Override with env `MAX_XML_BYTES`. Multipart instance-file uploads stay at `MAX_UPLOAD_BYTES` (default **5 MiB**) in `files.rs` — oversized uploads return **413 Payload Too Large**.

### ⚠️ `lockDuration` / `newDuration` are milliseconds at the HTTP boundary

Camunda/OpenAPI send milliseconds. `engine-server` converts to seconds for `engine-core` (`TimeDelta::seconds`): `ms <= 0 → 0`, otherwise `(ms + 999) / 1000` (ceil). Empty `topics` defaults to **30_000 ms** (30 s), not 30. Do not change engine-core to milliseconds.

### ⚠️ Startup restore prefers Definitions-KV

`StartupCoordinator.restore_definitions` loads `list_definitions()` (parsed JSON) first and deploys without XML reparse. BPMN XML is loaded only into `deployed_xml`. Empty or failed Definitions-KV falls back to XML parse. Persistence `keys()` errors propagate as `PersistenceError`, not as an empty list.

### ⚠️ Deploy XML save failure is HTTP 500

If persistence is configured and `save_bpmn_xml` fails, the handler rolls back the in-memory definition (`delete_definition`) and returns 500. `deployed_xml` is updated only after a successful save. Without persistence the RAM deploy still returns 200.

### ⚠️ Startup restore can take time

On server start with large NATS state, `StartupCoordinator.restore()` loads all definitions and instances synchronously (within the async context). Very large deployments may delay server readiness. The `/api/ready` endpoint returns `503` until NATS connection is verified.

### ⚠️ Log buffer is rolling (5,000 entries)

Oldest entries are dropped when the buffer exceeds 5,000. NATS persistence (`ENGINE_LOGS` stream) stores 50,000 entries. The file fallback (`engine_logs.jsonl`) is NOT truncated — it grows indefinitely (mitigated by `LOG_FILE=off` or external rotation). The file is gitignored; never commit runtime logs.

### ⚠️ In-memory fallback is opt-in (dev)

`REQUIRE_NATS` defaults to **true**. If NATS is unavailable the process refuses to start. Set `REQUIRE_NATS=false` only for ephemeral local runs (warning log; all state is lost on restart). `/api/ready` returns **503** when NATS is required but no persistence is configured.

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
