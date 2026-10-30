# Environment Variables

Complete list of environment variables read by Rust code in this repository
(`std::env::var` / `env::var`). Defaults are taken from the source, not guessed.

Snapshot: 2026-10-24.

Sources: `engine-server/src/{main.rs,server/state.rs,server/mod.rs}`,
`engine-core/src/{scripting/runner.rs,engine/retry_queue.rs}`,
`persistence-nats/src/{client.rs,tests.rs}`,
`agent-orchestrator/src/main.rs`, `desktop-tauri/src-tauri/src/main.rs`.

## engine-server

| Variable | Default | Meaning | Scope |
|----------|---------|---------|-------|
| `NATS_URL` | `nats://localhost:4222` | JetStream server URL | `main.rs` (startup), `server/mod.rs` (state) |
| `REQUIRE_NATS` | `true` | Fail-closed: without NATS the server refuses to start (no silent in-memory fallback). `false`/`0`/`no`/`off` opts into in-memory mode; `/api/ready` then mirrors the weaker durability | `server/state.rs` |
| `MAX_UPLOAD_BYTES` | `5242880` (5 MiB) | Max multipart size for instance file uploads → HTTP 413. Non-numeric or `0` falls back to the default | `server/state.rs` |
| `MAX_XML_BYTES` | `10485760` (10 MiB) | Cap for BPMN XML payloads on deploy → HTTP 413; also sets the axum `DefaultBodyLimit` for JSON bodies | `server/state.rs`, `server/deploy.rs`, `server/mod.rs` |
| `BIND_ADDR` | `0.0.0.0` | Listen host. Alias: `BIND_ADDRESS` (used when `BIND_ADDR` is unset) | `main.rs`, `server/state.rs` |
| `PORT` | `8081` | HTTP listen port | `main.rs` |
| `CORS_ORIGINS` | `http://localhost:1420`, `http://127.0.0.1:1420`, `tauri://localhost`, `https://tauri.localhost` | Comma-separated CORS allowlist. Empty value → default list. `*` = allow any origin (dev escape hatch only) | `server/state.rs` |
| `BPMNINJA_API_KEY` | unset (auth off) | Admin API key, accepted as `Authorization: Bearer …` or `X-Api-Key`; grants full access. Empty string counts as unset | `server/state.rs` |
| `BPMNINJA_WORKER_API_KEY` | unset | Worker key, limited to service-task routes (fetch/lock/complete/failure/extend) | `server/state.rs` |
| `BPMNINJA_ALLOW_UNAUTHENTICATED` | `false` | Explicit escape hatch: keeps the API-key gate off even on a non-loopback bind. Only `1`/`true`/`yes`/`on` enable it | `server/state.rs` |
| `TIMER_INTERVAL_MS` | `1000` | Maximum idle wait of the timer scheduler (ms). The scheduler sleeps until the next due timer or a schedule change, and at most this long | `main.rs` |
| `RUST_LOG` | `info` | Log filter, read via `tracing_subscriber::EnvFilter::try_from_default_env()` | `main.rs` |
| `LOG_FORMAT` | `text` | `text` (human-readable) or `json` (structured) | `main.rs` |
| `LOG_FILE` | `engine_logs.jsonl` | Path of the file-backed log buffer; `off` keeps logs in memory only | `main.rs` |

Auth gating (`server/state.rs`): the gate is active when any key is set. Without
keys it stays active unless the server binds to loopback, auth was disabled via
`AppBuildConfig`, or `BPMNINJA_ALLOW_UNAUTHENTICATED` is set.

## engine-core

| Variable | Default | Meaning | Scope |
|----------|---------|---------|-------|
| `RHAI_MAX_OPERATIONS` | `50000` | Rhai operation budget per script evaluation | `scripting/runner.rs` |
| `RHAI_MAX_MEMORY_BYTES` | `2097152` (2 MiB) | Script memory budget; mapped onto Rhai's string/array/map size caps | `scripting/runner.rs` |
| `RHAI_TIMEOUT_MS` | `1000` | Hard wall-clock timeout per script (ms) | `scripting/runner.rs` |
| `PERSISTENCE_RETRY_QUEUE_CAPACITY` | `10000` | Capacity of the bounded background persistence retry queue. `0` or non-numeric → default | `engine/retry_queue.rs` |
| `PERSISTENCE_SHUTDOWN_DRAIN_MS` | `5000` | Time budget for draining queued persistence jobs during shutdown (ms) | `engine/retry_queue.rs` |

## persistence-nats

Applied when the JetStream streams are created or updated. Invalid values log a
warning and fall back to the default.

| Variable | Default | Meaning | Scope |
|----------|---------|---------|-------|
| `BPMNINJA_EVENT_RETENTION_DAYS` | `7` | `max_age` of the event stream in days; `0` = unlimited | `client.rs` |
| `BPMNINJA_HISTORY_RETENTION_DAYS` | `30` | `max_age` of the history stream in days; `0` = unlimited | `client.rs` |
| `BPMNINJA_STREAM_MAX_BYTES` | `1073741824` (1 GiB) | `max_bytes` per stream; `0` = unlimited, negative/invalid → default | `client.rs` |
| `BPMNINJA_STREAM_REPLICAS` | `1` | Stream replica count; capped at the NATS limit of `5`, `0`/invalid → default | `client.rs` |

## agent-orchestrator

| Variable | Default | Meaning | Scope |
|----------|---------|---------|-------|
| `ENGINE_API_URL` | `http://localhost:8081` | Base URL of the engine REST API | `main.rs` |
| `BPMNINJA_API_KEY` | unset | If set and non-empty, sent as `Authorization: Bearer …` on every request | `main.rs` |
| `LOG_FORMAT` | `text` | `text` or `json` logging | `main.rs` |
| `RUST_LOG` | `info` | Log filter via `EnvFilter::try_from_default_env()` | `main.rs` |

## desktop-tauri (Rust backend)

| Variable | Default | Meaning | Scope |
|----------|---------|---------|-------|
| `ENGINE_API_URL` | `http://localhost:8081` | Initial base URL of the engine REST API (can be changed in the UI) | `src-tauri/src/main.rs` |

## Tests only

| Variable | Default | Meaning | Scope |
|----------|---------|---------|-------|
| `NATS_URL` | `nats://localhost:4222` | NATS URL used by the persistence integration tests | `persistence-nats/src/tests.rs` |
| `REQUIRE_NATS_TESTS` | unset | When set, NATS-backed tests fail instead of being skipped | `persistence-nats/src/tests.rs` |
| `CI` | unset | When set, NATS-backed tests are treated as required (same effect as `REQUIRE_NATS_TESTS`) | `persistence-nats/src/tests.rs` |
