# engine-server — Responsibilities

## What engine-server Owns

1. **@tag:rest-api** — Complete REST API surface: 38 endpoints across definitions, instances, tasks, files, events, messages, history, monitoring, and health.
2. **@tag:http-error-mapping** — `AppError` enum maps `EngineError` variants to HTTP status codes with JSON error bodies.
3. **@tag:sse-endpoint** — `GET /api/events` endpoint bridging `EngineEvent` broadcast channel to Server-Sent Events for push-based UI updates.
4. **@tag:timer-scheduler** — Background Tokio task that periodically calls `engine.process_timers()` (interval configurable via `TIMER_INTERVAL_MS`, default 1000ms).
5. **@tag:startup-restore** — `StartupCoordinator` restores all state from NATS persistence on server start: definitions, instances, user tasks, service tasks, timers, message catches.
6. **@tag:log-buffer** — Rolling in-memory log buffer (5,000 entries) captured via custom `tracing` layer, with optional NATS JetStream persistence (`ENGINE_LOGS` stream, 50,000 entries) and file fallback (`engine_logs.jsonl`).
7. **@tag:prometheus-metrics** — `/metrics` endpoint exposing engine counters and gauges via `metrics-exporter-prometheus`.
8. **@tag:health-endpoints** — `/api/health` (liveness) and `/api/ready` (readiness via NATS connection check).
9. **@tag:cors** — CORS middleware allowing all origins (development-friendly; configure for production).
10. **@tag:request-size-limit** — 5 MB body limit via `DefaultBodyLimit` middleware (for BPMN XML deployment).

## Invariants

1. **Never hold engine lock across `.await`**: Route handlers must scope all lock access before any async call.
2. **All errors return JSON**: `{ "error": "Human-readable message" }` — no HTML error pages.
3. **UUID validation at boundary**: Path parameters are parsed to `Uuid` immediately, returning 400 on failure.
4. **SSE events are fire-and-forget**: Server doesn't care if SSE clients miss events (channel capacity 256).
5. **Graceful shutdown**: `Ctrl+C` / `SIGTERM` shuts down timer scheduler, flushes persistence queue, and stops Axum.
6. **Deployment size limit**: BPMN XML uploads capped at 5 MB (configurable in `build_app_with_engine`). Multipart instance files capped via `MAX_UPLOAD_BYTES` (default 5 MiB).
7. **NATS optional in dev**: Without `REQUIRE_NATS`, server starts in in-memory mode if NATS is unavailable (warning log). With `REQUIRE_NATS=true` (docker-compose default), startup fails instead of silent data-loss fallback.
8. **Prometheus optional**: `/metrics` only mounted if `prometheus_handle` is `Some`.

## Internal Module Responsibilities

| Module | Path | Purpose |
|--------|------|---------|
| `main.rs` | `src/main.rs` | Entry point: tracing setup, NATS connect, engine init, timer scheduler, Axum serve |
| `startup.rs` | `src/startup.rs` | `StartupCoordinator`: restores definitions, instances, tasks, timers, messages from NATS |
| `log_buffer.rs` | `src/log_buffer.rs` | `LogBuffer`: rolling 5000-entry in-memory log + file persistence + NATS sync |
| `log_nats.rs` | `src/log_nats.rs` | `NatsLogSink`: syncs log buffer to NATS JetStream `ENGINE_LOGS` stream |
| `observability.rs` | `src/observability.rs` | Prometheus recorder setup, metrics handler, HTTP metrics middleware |
| `server/state.rs` | `src/server/state.rs` | `AppState`, `AppError` → HTTP status mapping, `parse_uuid` helper |
| `server/mod.rs` | `src/server/mod.rs` | Route registration: `build_app_with_engine` → Axum Router |
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
