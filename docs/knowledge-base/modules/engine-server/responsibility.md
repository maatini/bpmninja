# engine-server — Responsibilities

## What engine-server Owns

1. **@tag:rest-api** — Complete REST API surface: 46 endpoints across definitions, instances, tasks, files, events, messages, history, monitoring, and health.
2. **@tag:http-error-mapping** — `AppError` enum maps `EngineError` variants to HTTP status codes with JSON error bodies.
3. **@tag:sse-endpoint** — `GET /api/events` endpoint bridging `EngineEvent` broadcast channel to Server-Sent Events for push-based UI updates.
4. **@tag:timer-scheduler** — Background Tokio task that calls `engine.process_timers()` when the next timer is due or the schedule changes. `TIMER_INTERVAL_MS` (default 1000ms) is the maximum idle wait. Shutdown cancels the wait.
5. **@tag:startup-restore** — `StartupCoordinator` restores all state from NATS persistence on server start: definitions, instances, user tasks, service tasks, timers, message catches.
6. **@tag:log-buffer** — Rolling in-memory log buffer (5,000 entries) captured via custom `tracing` layer, with optional NATS JetStream persistence (`ENGINE_LOGS` stream, 50,000 entries) and file fallback (`engine_logs.jsonl`).
7. **@tag:prometheus-metrics** — `/metrics` endpoint exposing engine counters and gauges via `metrics-exporter-prometheus`.
8. **@tag:health-endpoints** — `/api/health` (liveness, always 200) and `/api/ready` (readiness: required durability + live NATS storage check).
9. **@tag:cors** — CORS allowlist via `CORS_ORIGINS` (default Tauri/localhost origins; `*` is the only Allow-Any escape). Methods GET/POST/PUT/DELETE/OPTIONS; headers Authorization, Content-Type, Accept, X-API-Key.
10. **@tag:request-size-limit** — JSON/XML body limit via `DefaultBodyLimit` = `MAX_XML_BYTES` (10 MiB, OpenAPI). Multipart instance files remain `MAX_UPLOAD_BYTES` (5 MiB).
11. **@tag:api-auth** — API-Key-Gate mit Rollen: `BPMNINJA_API_KEY` (Admin, alles) und optional `BPMNINJA_WORKER_API_KEY` (nur Service-Task-Routen, sonst 403). Fail-closed ohne Key bei Nicht-Loopback-Bind (401). Öffentlich: `GET /api/health`, `GET /api/ready`.
12. **@tag:lock-duration-ms** — HTTP `lockDuration`/`newDuration` sind Millisekunden; Validierung (1_000..=3_600_000 ms, `maxTasks` 1..=100, 1..=32 Topics) und Umrechnung in Sekunden an der Server-Grenze. Jedes Topic behält seine eigene Lock-Dauer.

## Invariants

1. **Never hold engine lock across `.await`**: Route handlers must scope all lock access before any async call.
2. **All errors return JSON**: `{ "error": "Human-readable message" }` — no HTML error pages.
3. **UUID validation at boundary**: Path parameters are parsed to `Uuid` immediately, returning 400 on failure.
4. **SSE events are fire-and-forget**: Server doesn't care if SSE clients miss events (channel capacity 256).
5. **Graceful shutdown**: `Ctrl+C` / `SIGTERM` shuts down timer scheduler, flushes persistence queue, and stops Axum.
6. **Deployment size limit**: BPMN XML/JSON bodies capped at 10 MiB (`MAX_XML_BYTES`, same as `DefaultBodyLimit`). Multipart instance files capped via `MAX_UPLOAD_BYTES` (default 5 MiB).
7. **NATS required by default**: `REQUIRE_NATS` defaults to true — startup fails if NATS is unavailable. Set `REQUIRE_NATS=false` only for ephemeral local runs (in-memory, state lost on restart).
8. **Prometheus optional**: `/metrics` only mounted if `prometheus_handle` is `Some`. Protected by the API-key gate (Worker-Key → 403).
9. **Auth ist fail-closed**: kein Key gesetzt + Nicht-Loopback-Bind → 401 auf allen `/api`-Routen außer health/ready. `BPMNINJA_ALLOW_UNAUTHENTICATED=true` ist der einzige bewusste Opt-out; Tests deaktivieren das Gate über `AppBuildConfig { api_key: Some(None), worker_api_key: Some(None) }` (nie über Prozess-Env).
10. **Bind address**: `BIND_ADDR` / `BIND_ADDRESS` default `0.0.0.0` (required for Docker). Ohne Key ist dieser Default nur noch mit explizitem Opt-out ungeschützt nutzbar; lokal `BIND_ADDR=127.0.0.1` setzen.

## Internal Module Responsibilities

| Module | Path | Purpose |
|--------|------|---------|
| `main.rs` | `src/main.rs` | Entry point: tracing setup, NATS connect, engine init, timer scheduler, Axum serve (`BIND_ADDR`) |
| `startup.rs` | `src/startup.rs` | `StartupCoordinator`: restores definitions, instances, tasks, timers, messages from NATS |
| `log_buffer.rs` | `src/log_buffer.rs` | `LogBuffer`: rolling 5000-entry in-memory log + file persistence + NATS sync |
| `log_nats.rs` | `src/log_nats.rs` | `NatsLogSink`: syncs log buffer to NATS JetStream `ENGINE_LOGS` stream |
| `observability.rs` | `src/observability.rs` | Prometheus recorder setup, metrics handler, HTTP metrics middleware |
| `server/state.rs` | `src/server/state.rs` | `AppState`, `AppError` → HTTP status mapping, env parsers, `lock_duration_ms_to_secs` |
| `server/auth.rs` | `src/server/auth.rs` | API-Key-Gate (fail-closed), Rollen (Admin/Worker), 401/403, `bpmn_auth_failures_total` |
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
