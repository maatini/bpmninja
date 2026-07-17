# Tag Registry

All `@tag:xxx` references used across the knowledge base. Tags identify cross-cutting concepts and responsibilities.

| Tag | Meaning |
|-----|---------|
| `@tag:bpmn-domain-model` | BPMN domain types: BpmnElement, ProcessDefinition, Token |
| `@tag:bpmn-xml-parsing` | Parsing BPMN 2.0 XML → typed structs |
| `@tag:bpmn-js-modeler` | bpmn-js integration in the desktop app |
| `@tag:camunda-compatible` | Camunda-style fetch-and-lock service task API |
| `@tag:condition-extraction` | Extracting gateway conditions from sequence flows |
| `@tag:cors` | CORS middleware in engine-server |
| `@tag:element-handlers` | Per-BpmnElement-variant execution logic |
| `@tag:element-mapping` | XML element → BpmnElement variant mapping |
| `@tag:engine-events` | Broadcast engine state changes via SSE |
| `@tag:engine-state` | Runtime state: definitions, instances, wait-state queues |
| `@tag:fault-tolerant-retry` | Two-stage persistence retry (inline + background worker) |
| `@tag:gateway-routing` | XOR/AND/OR/EventBased/Complex gateway evaluation |
| `@tag:graceful-shutdown` | Clean shutdown with queue flush |
| `@tag:health-endpoints` | /api/health (liveness) and /api/ready (readiness) |
| `@tag:hexagonal-architecture` | WorkflowPersistence trait as port; NATS + in-memory as adapters |
| `@tag:history-audit` | Audit trail: HistoryEntry, HistoryDiff, snapshots |
| `@tag:http-error-mapping` | EngineError → HTTP status code mapping in AppError |
| `@tag:incident-creation` | Creating incidents when retries exhausted |
| `@tag:incident-management` | Retry/resolve incidents from UI |
| `@tag:instance-tracking` | Live instance tracking in desktop app |
| `@tag:iso8601-timer-parsing` | Parsing Duration, AbsoluteDate, CronCycle, RepeatingInterval |
| `@tag:listener-extraction` | Extracting execution listeners from Camunda extension elements |
| `@tag:lock-extension` | Automatic lock extension for long-running service tasks |
| `@tag:lock-free-concurrency` | DashMap for wait-state queues; per-instance RwLock |
| `@tag:log-buffer` | Rolling 5000-entry in-memory log buffer |
| `@tag:long-polling` | fetchAndLock with asyncResponseTimeout |
| `@tag:migration-dialog` | Instance migration UI with node mapping |
| `@tag:monitoring-dashboard` | Storage info, KV browser, engine stats, log stream |
| `@tag:multi-topic-subscription` | Multiple topic handlers in parallel |
| `@tag:namespace-handling` | Camunda namespace prefix on BPMN extension elements |
| `@tag:persistence-port` | WorkflowPersistence trait in engine-core/src/port |
| `@tag:prometheus-metrics` | /metrics endpoint with counters and gauges |
| `@tag:request-size-limit` | 5 MB body limit for BPMN XML deployment |
| `@tag:rest-api` | Complete REST API surface: 38 endpoints |
| `@tag:retry-backoff` | Exponential backoff (1s → 2s → 4s → ..., max 30s) |
| `@tag:retry-queue` | Background persistence retry worker |
| `@tag:rhai-scripting` | Sandboxed Rhai script execution with resource limits |
| `@tag:sse-client` | SSE subscription in desktop app (Tauri background task) |
| `@tag:sse-endpoint` | GET /api/events — push-based state updates |
| `@tag:sse-push` | Push-based UI updates via SSE (no polling) |
| `@tag:startup-restore` | State restoration from NATS on server start |
| `@tag:subprocess-flattening` | Flattening embedded sub-processes at parse time |
| `@tag:tauri-commands` | Tauri Rust backend — HTTP proxy layer |
| `@tag:timer-scheduler` | Background task: process_timers() every N ms |
| `@tag:token-execution` | Token-based execution model: NextAction drive loop |
| `@tag:ui-components` | shadcn/ui + Tailwind CSS for desktop app |
| `@tag:ui-shell` | Tauri app shell with 7 tabs |
| `@tag:worker-client` | TypeScript external task worker client |
