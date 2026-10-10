# engine-server — Responsibilities

## What engine-server Owns

1. **@tag:rest-api** — Complete REST API surface: 46 endpoints across definitions, instances, tasks, files, events, messages, history, monitoring, and health.
2. **@tag:http-error-mapping** — `AppError` enum maps `EngineError` variants to HTTP status codes with JSON error bodies.
3. **@tag:sse-endpoint** — `GET /api/events` endpoint bridging `EngineEvent` broadcast channel to Server-Sent Events for push-based UI updates.
4. **@tag:timer-scheduler** — Background Tokio task that periodically calls `engine.process_timers()` (interval configurable via `TIMER_INTERVAL_MS`, default 1000ms).
5. **@tag:startup-restore** — `StartupCoordinator` restores all state from NATS persistence on server start: definitions, instances, user tasks, service tasks, timers, message catches.
6. **@tag:log-buffer** — Rolling in-memory log buffer (5,000 entries) captured via custom `tracing` layer, with optional NATS JetStream persistence (`ENGINE_LOGS` stream, 50,000 entries) and file fallback (`engine_logs.jsonl`).
7. **@tag:prometheus-metrics** — `/metrics` endpoint exposing engine counters and gauges via `metrics-exporter-prometheus`.
8. **@tag:health-endpoints** — `/api/health` (liveness, always 200) and `/api/ready` (readiness: required durability + live NATS storage check).
9. **@tag:cors** — CORS allowlist via `CORS_ORIGINS` (default Tauri/localhost origins; `*` is the only Allow-Any escape). Methods GET/POST/PUT/DELETE/OPTIONS; headers Authorization, Content-Type, Accept, X-API-Key.
10. **@tag:request-size-limit** — JSON/XML body limit via `DefaultBodyLimit` = `MAX_XML_BYTES` (10 MiB, OpenAPI). Multipart instance files remain `MAX_UPLOAD_BYTES` (5 MiB).
11. **@tag:api-auth** — Optional shared API key (`BPMNINJA_API_KEY`): Bearer or `X-API-Key`. Public: `GET /api/health`, `GET /api/ready`.
12. **@tag:lock-duration-ms** — HTTP `lockDuration`/`newDuration` are milliseconds; converted to seconds at the engine-server boundary.

## Invariants

1. **Never hold engine lock across `.await`**: Route handlers must scope all lock access before any async call.
2. **All errors return JSON**: `{ "error": "Human-readable message" }` — no HTML error pages.
3. **UUID validation at boundary**: Path parameters are parsed to `Uuid` immediately, returning 400 on failure.
4. **SSE events are fire-and-forget**: Server doesn't care if SSE clients miss events (channel capacity 256).
5. **Graceful shutdown**: `Ctrl+C` / `SIGTERM` shuts down timer scheduler, flushes persistence queue, and stops Axum.
6. **Deployment size limit**: BPMN XML/JSON bodies capped at 10 MiB (`MAX_XML_BYTES`, same as `DefaultBodyLimit`). Multipart instance files capped via `MAX_UPLOAD_BYTES` (default 5 MiB).
7. **NATS required by default**: `REQUIRE_NATS` defaults to true — startup fails if NATS is unavailable. Set `REQUIRE_NATS=false` only for ephemeral local runs (in-memory, state lost on restart).
8. **Prometheus optional**: `/metrics` only mounted if `prometheus_handle` is `Some`. Protected by API-key auth when `BPMNINJA_API_KEY` is set.
9. **Auth off unless configured**: Unset `BPMNINJA_API_KEY` means no auth. Tests use `AppBuildConfig.api_key` (never process env).
10. **Bind address**: `BIND_ADDR` / `BIND_ADDRESS` default `0.0.0.0` (required for Docker).

## Internal Module Responsibilities

| Module | Path | Purpose |
|--------|------|---------|
| `main.rs` | `src/main.rs` | Entry point: tracing setup, NATS connect, engine init, timer scheduler, Axum serve (`BIND_ADDR`) |
| `startup.rs` | `src/startup.rs` | `StartupCoordinator`: restores definitions, instances, tasks, timers, messages from NATS |
| `log_buffer.rs` | `src/log_buffer.rs` | `LogBuffer`: rolling 5000-entry in-memory log + file persistence + NATS sync |
| `log_nats.rs` | `src/log_nats.rs` | `NatsLogSink`: syncs log buffer to NATS JetStream `ENGINE_LOGS` stream |
| `observability.rs` | `src/observability.rs` | Prometheus recorder setup, metrics handler, HTTP metrics middleware |
| `server/state.rs` | `src/server/state.rs` | `AppState`, `AppError` → HTTP status mapping, env parsers, `lock_duration_ms_to_secs` |
| `server/auth.rs` | `src/server/auth.rs` | Optional API-key middleware (Bearer / `X-API-Key`) |
| `server/mod.rs` | `src/server/mod.rs` | Route registration, CORS layer, `DefaultBodyLimit`, `build_app*` |
| `server/deploy.rs` | `src/server/deploy.rs` | Deploy, list, get XML, delete definitions |
| `server/instances.rs` | `src/server/instances.rs` | Start, list, get, delete, suspend, resume, move token, migrate, update variables |
| `server/tasks.rs` | `src/server/tasks.rs` | User tasks + service tasks (fetchAndLock, complete, failure, retry, resolve, bpmnError, extendLock) |
| `server/files.rs` | `src/server/files.rs` | File upload, download, delete for instance variables |
| `server/history.rs` | `src/server/history.rs` | Instance history + completed instance archive search |
| `server/events.rs` | `src/server/events.rs` | SSE endpoint: subscribe to broadcast channel, stream to client |
| `server/messages.rs` | `src/server/messages.rs` | Message correlation + list pending messages |
| `server/timers.rs` | `src/server/timers.rs` | List pending timers + manual timer processing |
| `server/monitoring.rs` | `src/server/monitoring.rs` | Health, ready, info, monitoring stats, KV bucket browser |
| `server/logs.rs` | `src/server/logs.rs` | Query log buffer with level/text filters |
