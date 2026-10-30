# engine-server — Gotchas

### ⚠️ Incident-Routen sind Admin-only

`GET /api/instances/{id}/incidents` und `POST /api/instances/{id}/incidents/{incident_id}/resolve` stehen nicht auf der Worker-Whitelist (`auth.rs`), ein Worker-Key erhält also **403**. Der Resolve-Body ist optional (`{ "resolvedBy": "..." }`); ein bereits aufgelöster Incident liefert **409**, eine unbekannte Incident-ID **404**. Die Incident-Liste lebt im `ProcessInstance`-Dokument und wird mit diesem persistiert — es gibt keine separate Persistence-Trait-Methode.

### ⚠️ Route handlers MUST NOT hold engine locks across `.await`

The `AppState.engine` is `Arc<WorkflowEngine>`. All engine methods take `&self`. However, if a route handler acquires a lock from engine internals (e.g., `instances.get(&id).await`), it must drop that lock before any other `.await` point.

This is especially relevant in:
- `tasks.rs` — `fetchAndLock` awaits one long-poll; the engine wakes it when a task becomes fetchable (no lock held across that wait)
- `instances.rs` — multi-step operations (suspend + persist)
- `history.rs` — querying multiple persistence operations

### ⚠️ SSE is broadcast, NOT queue

`GET /api/events` uses `tokio::sync::broadcast::Receiver`. If a client is slow (channel capacity 256), it **will miss events**. The protocol is: event fires → client re-fetches via REST. No retry or catch-up mechanism.

### ⚠️ CORS is an allowlist (not `Any` by default)

`CORS_ORIGINS` is a comma-separated origin list. **Unset** → `http://localhost:1420,http://127.0.0.1:1420,tauri://localhost,https://tauri.localhost`. Explicit `CORS_ORIGINS=*` is the only way to allow any origin (dev escape hatch). `build_app()` uses the default allowlist, never silent `Any`.

Allowed methods: GET, POST, PUT, DELETE, OPTIONS. Allowed headers: `Authorization`, `Content-Type`, `Accept`, `X-API-Key`.

### ⚠️ API-Auth ist fail-closed, sobald nicht an Loopback gebunden wird

Das Gate ist aktiv, wenn `BPMNINJA_API_KEY` (Admin) oder `BPMNINJA_WORKER_API_KEY` (Worker) gesetzt ist. Zusätzlich gilt **fail-closed**: ohne jeden Key und mit Nicht-Loopback-Bind (`BIND_ADDR` ≠ `127.0.0.1`/`::1`/`localhost`) antworten alle `/api`-Routen mit **401**, ausgenommen `GET /api/health` und `GET /api/ready`. Ein bewusster Dev-Opt-out ist `BPMNINJA_ALLOW_UNAUTHENTICATED=true`; `AppBuildConfig { api_key: Some(None) }` schaltet das Gate nur explizit für Tests/Helfer aus.

Rollen: Admin-Key darf alles, Worker-Key nur `fetchAndLock`, `complete`, `failure`, `extendLock`, `bpmnError` — alles andere (inkl. `retry`/`resolve`, `/metrics`) ist **403**. Ist kein Worker-Key gesetzt, darf der Admin-Key auch Worker-Routen nutzen. Fehlversuche werden mit Methode + Pfad geloggt (keine Secrets) und als `bpmn_auth_failures_total{reason}` gezählt. Tests müssen Keys über `AppBuildConfig` injizieren, nie über Prozess-Env (parallele Testläufe).

### ⚠️ Body size limit is 10 MiB for XML/JSON

`DefaultBodyLimit` and the deploy-path cap share `MAX_XML_BYTES` (**10 MiB** = `10 * 1024 * 1024`, OpenAPI contract). Override with env `MAX_XML_BYTES`. Multipart instance-file uploads stay at `MAX_UPLOAD_BYTES` (default **5 MiB**) in `files.rs` — oversized uploads return **413 Payload Too Large**.

### ⚠️ Deploy-Model-Checking ist opt-in (Feature und Env)

`MODEL_CHECK` steht standardmäßig auf `off`. Wirksam wird die Prüfung nur, wenn das Binary zusätzlich mit `--features model-check` gebaut wurde (`rust_bpmn_analyzer`, MIT, Git-Rev `3d3eede32409d70e6818d8344d480de8d76a6d8b`, nicht auf crates.io).

| Modus | Deploy | Antwort |
|---|---|---|
| `off` | immer 200 | kein Feld `model_check` |
| `warn` | immer 200, auch bei Befund, Timeout, Panic, belegter Analyse oder fehlendem Feature | `model_check` plus Log bei Befunden |
| `enforce` | 400 bei bestätigtem Verstoß oder wenn das Feature fehlt | `{ "error", "model_check" }` mit den Befunden |

Timeout (`inconclusive`), Panic (`failed`) und „eine Analyse gleichzeitig“ (`skipped`) lehnen auch in `enforce` nicht ab — sonst würde ein hängender oder abstürzender Analyzer Deploys blockieren. Die Analyse läuft in `spawn_blocking`, das Budget ist `MODEL_CHECK_TIMEOUT_MS` (Default 2000). `build_app()` erzwingt `off`, damit parallele Tests nicht von der Prozess-Env abhängen.

### ⚠️ Alle Fehlerantworten sind `{ "error": ... }`

Ausnahme: `MODEL_CHECK=enforce` ergänzt bei Ablehnung das Feld `model_check` (Status, Befunde, Grund). `error` bleibt ein String und enthält die Befunde im Klartext.

JSON-Handler nutzen `ApiJson<T>` statt `axum::Json<T>` (definiert in `state.rs`). Dessen `FromRequest`/`OptionalFromRequest` mappen `JsonRejection` auf `AppError::Rejected` und rendern damit dieselbe Fehlerform wie alle anderen Fehler:

| Ursache | Status |
|---|---|
| JSON-Syntaxfehler | 400 |
| `EngineError::DataMappingError` | 400 |
| Body über `DefaultBodyLimit` | 413 |
| fehlender/falscher `Content-Type` | 415 |
| gültiges JSON, aber Schema-Mismatch | 422 |

Der Deploy-Handler prüft `payload.xml.len() > max_xml_bytes` zusätzlich und liefert 413. `DataMappingError` (fehlende Output-Variable beim Abschluss, Fehler beim Betreten einer gemappten Aktivität) nutzt dieselbe 400-Form wie `InvalidDefinition`. Neue JSON-Handler müssen `ApiJson` verwenden, sonst antwortet Axum wieder mit Plain-Text.

### ⚠️ `GET /api/definitions/{id}/xml` liefert `text/plain`

Erfolg ist der rohe XML-String (`content-type: text/plain`), kein `{ "xml": ... }`-Objekt. Fehlt das XML in Cache und Persistence, antwortet der Endpunkt mit **404** und `{ "error": "No XML found for definition '…'" }`.

### ⚠️ Start-Felder: `bpmn_process_id` mit Legacy-Alias

`POST /api/start/latest` erwartet `bpmn_process_id`; `bpmn_id` bleibt per `#[serde(alias)]` gültig. `business_key` ist auf `StartRequest` und `StartLatestRequest` top-level erlaubt und hat Vorrang vor einem `business_key`-Eintrag in `variables` (engine-core entfernt den Schlüssel aus den Variablen und setzt `ProcessInstance.business_key`).

### ⚠️ `lockDuration` / `newDuration` sind Millisekunden an der HTTP-Grenze

Camunda/OpenAPI senden Millisekunden. `engine-server` validiert `topics[].lockDuration` auf **1_000..=3_600_000 ms**, `maxTasks` auf **1..=100** und `topics` auf **1..=32** (sonst `400`). Danach rechnet es in Sekunden um (`(ms + 999) / 1000`). Jedes Topic behält seine eigene Dauer (`fetch_and_lock_service_tasks_with_durations`). engine-core clampt Sekunden defensiv auf `1..=3600` und nutzt `TimeDelta::try_seconds` — extreme Werte (z. B. `i64::MAX`, negativ) dürfen keine Panic auslösen.

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
