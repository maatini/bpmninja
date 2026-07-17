# engine-server — Dependencies

## Outbound

| Dependency | Type | Purpose |
|-----------|------|---------|
| engine-core | Rust crate (direct) | All engine operations: `WorkflowEngine`, domain types, persistence trait |
| bpmn-parser | Rust crate (direct) | Parsing BPMN XML on deploy |
| persistence-nats | Rust crate (direct) | NATS connection and persistence backend |
| persistence-memory | Rust crate (indirect via engine-core tests) | Dev in-memory only when `REQUIRE_NATS` is false and NATS is down |
| axum | External crate | HTTP framework: routing, extractors, middleware |
| tokio | External crate | Async runtime, signal handling, timers, spawn |
| tracing / tracing-subscriber | External crate | Structured logging with env-filter, JSON + text formats |
| tower-http | External crate | CORS middleware, body limit middleware |
| uuid | External crate | UUID parsing in route handlers |
| metrics / metrics-exporter-prometheus | External crate | Prometheus metrics endpoint |
| serde / serde_json | External crate | JSON request/response serialization |
| futures | External crate | Stream extensions for SSE |

## Inbound

| Caller | How | For |
|--------|-----|-----|
| desktop-tauri | HTTP REST + SSE | All BPMN operations, real-time updates |
| external-task-client | HTTP REST | fetchAndLock, complete, failure, bpmnError |
| External systems | HTTP REST | Message correlation (`POST /api/message`) |
| Load balancers / K8s | HTTP | `/api/health` (liveness), `/api/ready` (readiness) |
| Prometheus | HTTP | `/metrics` endpoint |
| Docker Compose | HTTP | Service orchestration via health checks |

## Key Environment Variables

| Variable | Default | Used In |
|----------|---------|---------|
| `NATS_URL` | `nats://localhost:4222` | main.rs: NATS connection |
| `REQUIRE_NATS` | `false` | main.rs: fail-fast if NATS unavailable; ready fails without persistence |
| `MAX_UPLOAD_BYTES` | `5242880` (5 MiB) | files.rs: multipart upload size limit |
| `PORT` | `8081` | main.rs: HTTP listen port |
| `TIMER_INTERVAL_MS` | `1000` | main.rs: timer scheduler interval |
| `RUST_LOG` | `info` | main.rs: log level filter |
| `LOG_FORMAT` | `text` | main.rs: `text` or `json` |
| `LOG_FILE` | `engine_logs.jsonl` | main.rs: log file path, `off` to disable |

