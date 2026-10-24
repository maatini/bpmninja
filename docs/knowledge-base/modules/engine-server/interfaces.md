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
    pub max_xml_bytes: usize,         // MAX_XML_BYTES (default 10 MiB)
    pub api_key: Option<String>,      // BPMNINJA_API_KEY (admin); None = kein Admin-Key
    pub worker_api_key: Option<String>, // BPMNINJA_WORKER_API_KEY; None = nur Admin
    pub auth_required: bool,          // Gate aktiv (Key konfiguriert ODER fail-closed)
    pub cors_origins: Vec<String>,    // CORS_ORIGINS; `*` = Allow Any
}
```

## AppError → HTTP Status Mapping

| EngineError Variant | HTTP Status | Example |
|-------------------|-------------|---------|
| `InvalidDefinition(msg)` | 400 | Bad XML, missing fields |
| `NoMatchingCondition(msg)` | 400 | Gateway condition mismatch |
| `AppError::Unauthorized` | 401 | Fehlender/falscher API-Key, Gate aktiv (Key gesetzt oder fail-closed) |
| `AppError::Forbidden` | 403 | Gültiger Worker-Key auf einer Admin-Route |
| `AppError::BadRequest(msg)` | 400 | Invalid UUID, invalid BPMN XML |
| `AppError::PayloadTooLarge(msg)` | 413 | Multipart upload or XML/JSON body exceeds the configured limit |
| `AppError::Rejected { status, message }` | 400/413/415/422 | Axum-Extractor-Ablehnung (JSON-Syntax, Typfehler, Content-Type, Body-Limit) |
| `AppError::NotFound(msg)` | 404 | Fehlende Ressource, z. B. kein gespeichertes BPMN-XML |
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
| `IncidentNotFound(id)` | 404 | Incident-ID existiert nicht an der Instanz |
| `IncidentAlreadyResolved(id)` | 409 | Incident wurde bereits aufgelöst |
| `OrphanedToken(node)` | 422 | Migration: node missing in target definition |
| All other `EngineError` | 500 | Internal server error |

## Route Map (48 endpoints)

See groups below. Key groups:

| Group | Count | Key Endpoints |
|-------|-------|---------------|
| Definitions | 5 | `POST /api/deploy`, `GET /api/definitions`, `GET /api/definitions/:id/xml`, `DELETE /api/definitions/:id`, `DELETE /api/definitions/bpmn/:bpmn_id` |
| Instances | 13 | `POST /api/start`, `POST /api/start/latest`, `POST /api/start/timer`, `GET/DELETE /api/instances...`, `variables`, `suspend`, `resume`, `move-token`, `migrate`, `incidents`, `incidents/:incident_id/resolve` |
| User Tasks | 2 | `GET /api/tasks`, `POST /api/complete/:id` |
| Service Tasks | 8 | `GET /api/service-tasks`, `fetchAndLock`, `complete`, `failure`, `bpmnError`, `extendLock`, `retry`, `resolve` |
| Files | 3 | `POST/GET/DELETE /api/instances/:id/files/:var` |
| Events & Messages | 4 | `POST /api/message`, `GET /api/messages`, `GET /api/timers`, `POST /api/timers/process` |
| History & Archive | 4 | `GET /api/instances/:id/history`, `GET /api/instances/:id/history/:event_id`, `GET /api/history/instances`, `GET /api/history/instances/:id` |
| Monitoring & Health | 7 | `GET /api/health`, `/api/ready`, `/api/info`, `/api/monitoring`, bucket entries, `/metrics` |
| Observability | 2 | `GET /api/logs`, `GET /api/events` (SSE) |

## Incident-API

Admin-only (Worker-Key → 403). Incidents werden mit der Instanz persistiert; die Endpunkte lesen/schreiben direkt das `ProcessInstance`-Dokument.

| Methode & Pfad | Body | Antwort |
|----------------|------|---------|
| `GET /api/instances/{id}/incidents` | – | `200` mit `Incident[]` (offen + aufgelöst), `404` wenn Instanz unbekannt |
| `POST /api/instances/{id}/incidents/{incident_id}/resolve` | optional `{ "resolvedBy": "admin" }` | `200` mit aufgelöstem `Incident`; `404` bei unbekanntem Incident, `409` wenn bereits aufgelöst |

`Incident`-JSON: `id`, `kind` (`service_task`/`timer`/`persistence`/`bpmn_error`), `node_id`, `task_id`, `message`, `details`, `created_at`, `last_seen_at`, `occurrences`, `resolved_at`, `resolved_by`.

Die bestehenden Service-Task-Endpunkte `/api/service-task/{id}/retry` und `/api/service-task/{id}/resolve` bleiben unverändert und arbeiten weiter auf `retries <= 0`; der neue Resolve-Pfad setzt zusätzlich `resolved_at`/`resolved_by` am Incident.

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
    pub require_nats: Option<bool>,           // None → env REQUIRE_NATS (default true)
    pub max_upload_bytes: Option<usize>,      // None → env MAX_UPLOAD_BYTES
    pub max_xml_bytes: Option<usize>,         // None → env MAX_XML_BYTES (default 10 MiB)
    pub api_key: Option<Option<String>>,      // None → env BPMNINJA_API_KEY; Some(None)=explizit aus; Some(Some(k))=an
    pub worker_api_key: Option<Option<String>>, // None → env BPMNINJA_WORKER_API_KEY; Some(None)=kein Worker-Key
    pub bind_addr: Option<String>,            // None → env BIND_ADDR/BIND_ADDRESS; steuert fail-closed
    pub allow_unauthenticated: Option<bool>,  // None → env BPMNINJA_ALLOW_UNAUTHENTICATED (default false)
    pub cors_origins: Option<Vec<String>>,    // None → env CORS_ORIGINS (default allowlist, never Any)
}

pub const MAX_XML_BYTES: usize = 10 * 1024 * 1024; // DefaultBodyLimit == deploy cap

pub fn build_app() -> Router
// Test-friendly: require_nats = false, API-Key-Gate explizit aus, Default-CORS-Allowlist

pub fn build_app_with_options(config: AppBuildConfig) -> Router

pub fn build_app_with_engine(
    engine: Arc<WorkflowEngine>,
    persistence: Option<Arc<dyn WorkflowPersistence>>,
    xml_cache: HashMap<String, String>,
    prometheus_handle: Option<PrometheusHandle>,
    log_buffer: Arc<LogBuffer>,
) -> Router
// Uses AppBuildConfig::default() → env for require_nats / limits / api_key / worker_api_key / bind_addr / CORS

pub fn build_app_with_config(..., config: AppBuildConfig) -> Router
```

## HTTP lock duration

`POST /api/service-task/fetchAndLock` `topics[].lockDuration` und `POST /api/service-task/{id}/extendLock` `newDuration` sind **Millisekunden**. Der Server rechnet in Sekunden für engine-core um: `(ms + 999) / 1000` (ceil).

fetchAndLock-Validierung (sonst `400 Bad Request`, kein Long-Poll):

| Feld | Erlaubt |
|------|---------|
| `topics` | 1..=32 Einträge |
| `topics[].topicName` | nicht leer (getrimmt) |
| `topics[].lockDuration` | 1_000..=3_600_000 ms |
| `maxTasks` | 1..=100 |

Jedes Topic behält seine eigene Lock-Dauer: `WorkflowEngine::fetch_and_lock_service_tasks_with_durations(worker_id, max_tasks, &[(topic, secs)])`. engine-core clampt Sekunden defensiv auf `1..=3600` (`TimeDelta::try_seconds`), auch bei `extend_lock` — Werte außerhalb führen also nie zu einer Panic. Die alte Signatur `fetch_and_lock_service_tasks(..., lock_duration)` bleibt als Wrapper erhalten.

## Auth-Modi & Rollenmatrix

Das Gate ist aktiv (`AppState.auth_required`), sobald ein Key konfiguriert ist — oder fail-closed, wenn **kein** Key gesetzt ist und der Server **nicht** an Loopback bindet.

```text
kein Key + Loopback-Bind (127.0.0.1, ::1, localhost)  → Auth aus
kein Key + Nicht-Loopback-Bind                         → fail-closed: 401 auf allen /api-Routen
nur Admin-Key (BPMNINJA_API_KEY)                       → Admin darf alles (auch Worker-Routen)
Admin-Key + Worker-Key                                 → Rollentrennung: Worker nur Service-Tasks
BPMNINJA_ALLOW_UNAUTHENTICATED=true                    → Auth aus (expliziter Dev-Opt-out)
```

Immer öffentlich: `GET /api/health`, `GET /api/ready`; CORS-Preflight (`OPTIONS`) läuft immer durch.

| Route | Admin | Worker |
|-------|-------|--------|
| `POST /api/service-task/fetchAndLock` | ✅ | ✅ |
| `POST /api/service-task/{id}/complete` | ✅ | ✅ |
| `POST /api/service-task/{id}/failure` | ✅ | ✅ |
| `POST /api/service-task/{id}/extendLock` | ✅ | ✅ |
| `POST /api/service-task/{id}/bpmnError` | ✅ | ✅ |
| `POST /api/service-task/{id}/retry`, `/resolve` | ✅ | 403 |
| alles andere (`deploy`, `start`, `instances`, `definitions`, `tasks`, `monitoring`, `logs`, `events`, `timers`, `message`, `/metrics`) | ✅ | 403 |

Fehlversuche (fehlender/falscher Key → 401, Worker auf Admin-Route → 403) werden mit Methode + Pfad als `tracing::warn!` geloggt (ohne Secrets) und als `bpmn_auth_failures_total{reason}` gezählt (`invalid_or_missing_key`, `worker_role_forbidden`).

## Auth & CORS env

| Variable | Default | Notes |
|----------|---------|-------|
| `CORS_ORIGINS` | localhost:1420 + Tauri origins | Comma-separated. `*` = Allow Any (dev only). |
| `BIND_ADDR` (alias `BIND_ADDRESS`) | `0.0.0.0` | Listen host; Docker braucht `0.0.0.0`. Lokal ohne Key `127.0.0.1` setzen, sonst fail-closed (401). |
| `BPMNINJA_API_KEY` | unset | Admin-Key. Bearer oder `X-API-Key`. Health/ready bleiben öffentlich. |
| `BPMNINJA_WORKER_API_KEY` | unset | Worker-Key; nur Service-Task-Routen, sonst 403. |
| `BPMNINJA_ALLOW_UNAUTHENTICATED` | `false` | Bewusster Opt-out aus dem fail-closed-Gate (nur Dev). |
| `MAX_XML_BYTES` | `10485760` (10 MiB) | JSON/XML body + deploy cap. |

// Public exports
pub struct StartupCoordinator { ... }
impl StartupCoordinator {
    pub fn new(nats: Arc<NatsPersistence>) -> Self
    pub async fn restore(&self, engine: &mut WorkflowEngine, deployed_xml: &mut HashMap<String, String>) -> RestoreStats
}
pub struct RestoreStats { definitions, instances, user_tasks, service_tasks, timers, message_catches: usize }
```
