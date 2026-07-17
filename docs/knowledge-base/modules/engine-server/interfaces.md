# engine-server — Interfaces

## AppState (shared state for all route handlers)

```rust
pub struct AppState {
    pub engine: Arc<WorkflowEngine>,
    pub persistence: Option<Arc<dyn WorkflowPersistence>>,
    pub deployed_xml: Arc<RwLock<HashMap<String, String>>>,  // uuid → BPMN XML
    pub nats_url: String,
    pub log_buffer: Arc<LogBuffer>,
    pub require_nats: bool,           // REQUIRE_NATS → readiness / durability
    pub max_upload_bytes: usize,      // MAX_UPLOAD_BYTES (default 5 MiB)
}
```

## AppError → HTTP Status Mapping

| EngineError Variant | HTTP Status | Example |
|-------------------|-------------|---------|
| `InvalidDefinition(msg)` | 400 | Bad XML, missing fields |
| `NoMatchingCondition(msg)` | 400 | Gateway condition mismatch |
| `AppError::BadRequest(msg)` | 400 | Invalid UUID, bad JSON |
| `AppError::PayloadTooLarge(msg)` | 413 | Multipart upload exceeds `max_upload_bytes` |
| `NoSuchDefinition(id)` | 404 | Definition not found |
| `NoSuchInstance(id)` | 404 | Instance not found |
| `NoSuchNode(id)` | 404 | Node not found |
| `ServiceTaskNotFound(id)` | 404 | Service task not found |
| `TaskNotPending { task_id, actual_state }` | 409 | Task not in pending state |
| `ServiceTaskLocked { task_id, worker_id }` | 409 | Task locked by another worker |
| `ServiceTaskNotLocked(id)` | 409 | Complete called on unlocked task |
| `AlreadyCompleted` | 409 | Instance already in terminal state |
| `DefinitionHasInstances(count)` | 409 | Cannot delete definition with running instances |
| `InstanceSuspended(id)` | 409 | Operation blocked while suspended |
| `OrphanedToken(node)` | 422 | Migration: node missing in target definition |
| All other `EngineError` | 500 | Internal server error |

## Route Map (38 endpoints)

See [modules/engine-server/interfaces.md] for the complete table. Key groups:

| Group | Count | Key Endpoints |
|-------|-------|---------------|
| Definitions | 5 | `POST /api/deploy`, `GET /api/definitions`, `GET /api/definitions/:id/xml`, `DELETE /api/definitions/:id`, `DELETE /api/definitions/bpmn/:bpmn_id` |
| Instances | 9 | `POST /api/start`, `POST /api/start/latest`, `POST /api/start/timer`, `GET/PUT/DELETE /api/instances...`, `suspend`, `resume`, `move-token`, `migrate` |
| User Tasks | 2 | `GET /api/tasks`, `POST /api/complete/:id` |
| Service Tasks | 8 | `GET /api/service-tasks`, `fetchAndLock`, `complete`, `failure`, `bpmnError`, `extendLock`, `retry`, `resolve` |
| Files | 3 | `POST/GET/DELETE /api/instances/:id/files/:var` |
| Events & Messages | 4 | `POST /api/message`, `GET /api/messages`, `GET /api/timers`, `POST /api/timers/process` |
| History & Archive | 4 | `GET /api/instances/:id/history`, `GET /api/history/instances`, `GET /api/history/instances/:id` |
| Monitoring & Health | 7 | `GET /api/health`, `/api/ready`, `/api/info`, `/api/monitoring`, bucket entries, `/metrics` |
| Observability | 2 | `GET /api/logs`, `GET /api/events` (SSE) |

## SSE Event Format

```
event: instance_changed
data: {"type":"instance_changed"}
```

Events are coarse-grained (`InstanceChanged`, `TaskChanged`, `DefinitionChanged`). The client must re-fetch data via REST to get details.

## Log Buffer Query

```
GET /api/logs?level=info&search=error&limit=500
```

Returns JSON array of log entries with `timestamp`, `level`, `target`, `message`, `fields`.

## Health vs Readiness

| Endpoint | Semantics |
|----------|-----------|
| `GET /api/health` | Liveness — always `200` if process accepts HTTP |
| `GET /api/ready` | Readiness — `503` if `require_nats && persistence.is_none()`, or if configured persistence fails `get_storage_info()`; else `200` |

## Builder Functions

```rust
// Public API for tests and server binary
pub struct AppBuildConfig {
    pub require_nats: Option<bool>,      // None → env REQUIRE_NATS
    pub max_upload_bytes: Option<usize>, // None → env MAX_UPLOAD_BYTES
}

pub fn build_app() -> Router
// Test-friendly: require_nats = false (avoids env races under parallel tests)

pub fn build_app_with_options(config: AppBuildConfig) -> Router

pub fn build_app_with_engine(
    engine: Arc<WorkflowEngine>,
    persistence: Option<Arc<dyn WorkflowPersistence>>,
    xml_cache: HashMap<String, String>,
    prometheus_handle: Option<PrometheusHandle>,
    log_buffer: Arc<LogBuffer>,
) -> Router
// Uses AppBuildConfig::default() → env for require_nats / max_upload_bytes

pub fn build_app_with_config(..., config: AppBuildConfig) -> Router

// Public exports
pub struct StartupCoordinator { ... }
impl StartupCoordinator {
    pub fn new(nats: Arc<NatsPersistence>) -> Self
    pub async fn restore(&self, engine: &mut WorkflowEngine, deployed_xml: &mut HashMap<String, String>) -> RestoreStats
}
pub struct RestoreStats { definitions, instances, user_tasks, service_tasks, timers, message_catches: usize }
```
