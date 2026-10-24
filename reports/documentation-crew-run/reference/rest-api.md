---
title: REST-API
description: Routengruppen der BPMNinja-HTTP-API mit Methode, Pfad, Auth-Rolle und Statuscodes; einheitliches Fehlerformat; Verweis auf docs/openapi.yaml als einzige Quelle der Wahrheit.
audience:
  - Entwickler und Integratoren
  - Betriebsteams und Plattform-Administratoren
prerequisites:
  - Laufende BPMNinja-Engine (Default http://localhost:8081)
  - Vertrautheit mit HTTP, JSON und curl
  - Kenntnis des Auth-Modells (siehe how-to/konfiguration-anpassen.md, Schritt 3)
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/service-task-api.md
  - how-to/konfiguration-anpassen.md
  - how-to/external-task-worker-schreiben.md
  - tutorials/erster-prozess.md
  - _review/glossary.md
---

# REST-API

Die Engine stellt eine HTTP-JSON-API unter dem Präfix `/api` bereit. Zusätzlich existiert der Prometheus-Endpunkt `/metrics` außerhalb dieses Präfixes.

**Verbindliche Quelle der Wahrheit ist `docs/openapi.yaml`.** Diese Seite beschreibt die Routengruppen, die Authentifizierungsrollen und das Fehlerformat. Sie ersetzt die OpenAPI-Spezifikation nicht und nennt bewusst keine Gesamtzahl der Endpunkte.

## Basis-URL und Content-Typen

| Eigenschaft | Wert | Beleg |
|---|---|---|
| Default-Port | `8081` (`PORT`) | `bpmninja/README.md` Env-Tabelle |
| Default-Bind-Adresse | `0.0.0.0` (`BIND_ADDR`, Alias `BIND_ADDRESS`) | `docs/knowledge-base/modules/engine-server/interfaces.md` |
| JSON-Body-Limit | 10 MiB (`MAX_XML_BYTES` = `10485760`) | `docs/knowledge-base/modules/engine-server/gotchas.md` §„Body size limit" |
| Multipart-Upload-Limit | 5 MiB (`MAX_UPLOAD_BYTES` = `5242880`) | ebd. |
| Fehlerformat | `{ "error": "Human-readable error message" }` | `docs/knowledge-base/modules/engine-server/gotchas.md` §„Alle Fehlerantworten sind `{ "error": ... }`" |

## Authentifizierung

Die Engine kennt zwei Rollen. Das Gate ist aktiv, sobald `BPMNINJA_API_KEY` (Admin) oder `BPMNINJA_WORKER_API_KEY` (Worker) gesetzt ist. Zusätzlich gilt **fail-closed**: ohne jeden Key und mit Nicht-Loopback-Bind antworten alle `/api`-Routen mit `401`.

| Modus | Header | Ergebnis |
|---|---|---|
| Admin-Key | `X-API-Key: <key>` oder `Authorization: Bearer <key>` | Zugriff auf alle Routen |
| Worker-Key | `X-API-Key: <key>` oder `Authorization: Bearer <key>` | Nur Service-Task-Routen; alles andere `403` |
| Kein Key, Loopback-Bind | – | Zugriff erlaubt |
| Kein Key, Nicht-Loopback-Bind | – | `401` auf allen `/api`-Routen außer `/api/health` und `/api/ready` |
| `BPMNINJA_ALLOW_UNAUTHENTICATED=true` | – | Gate aus (nur Entwicklung) |

Immer öffentlich: `GET /api/health`, `GET /api/ready`; CORS-Preflight (`OPTIONS`) läuft immer durch.

**Beispiel — Admin-Zugriff:**

```bash
curl -s -H "X-API-Key: $BPMNINJA_API_KEY" http://localhost:8081/api/definitions
```

**Beispiel — Worker-Key auf einer Admin-Route (erwartet 403):**

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -H "X-API-Key: $BPMNINJA_WORKER_API_KEY" \
  http://localhost:8081/api/definitions
# 403
```

## Rollenmatrix für Service-Task-Routen

| Route | Admin | Worker |
|---|---|---|
| `POST /api/service-task/fetchAndLock` | erlaubt | erlaubt |
| `POST /api/service-task/{id}/complete` | erlaubt | erlaubt |
| `POST /api/service-task/{id}/failure` | erlaubt | erlaubt |
| `POST /api/service-task/{id}/extendLock` | erlaubt | erlaubt |
| `POST /api/service-task/{id}/bpmnError` | erlaubt | erlaubt |
| `POST /api/service-task/{id}/retry`, `/resolve` | erlaubt | **403** (Worker-Rolle nicht zugelassen) |
| alles andere (`deploy`, `start`, `instances`, `definitions`, `tasks`, `monitoring`, `logs`, `events`, `timers`, `message`, `/metrics`) | erlaubt | **403** (Worker-Rolle nicht zugelassen) |

Beleg: `docs/knowledge-base/modules/engine-server/interfaces.md` §„Rollenmatrix"; `docs/knowledge-base/modules/engine-server/gotchas.md` §„API-Auth ist fail-closed".

Fehlversuche werden mit Methode und Pfad als `tracing::warn!` geloggt (ohne Secrets) und als `bpmn_auth_failures_total{reason}` gezählt mit `reason` ∈ {`invalid_or_missing_key`, `worker_role_forbidden`}.

---

## Routengruppen

Die folgenden Tabellen nennen Methode, Pfad, Auth-Rolle und die belegten Statuscodes. Wo ein Statuscode nicht belegt ist, steht „Offen".

### Definitions

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `POST` | `/api/deploy` | Admin | `200`, `400` (`InvalidDefinition`), `413`, `415`, `422`, `500` | Deploy mit JSON-Body `{ "name": …, "xml": … }` (max. 10 MiB XML) |
| `GET` | `/api/definitions` | Admin | `200` | Alle Definitionen auflisten |
| `GET` | `/api/definitions/{id}/xml` | Admin | `200` (`text/plain`), `404` | BPMN-XML einer Definition abrufen |
| `DELETE` | `/api/definitions/{id}` | Admin | `200`, `404`, `409` (`DefinitionHasInstances`) | Definition löschen; `?cascade=true` löscht auch Instanzen |
| `DELETE` | `/api/definitions/bpmn/{bpmn_id}` | Admin | `200`, `404` | Alle Versionen einer BPMN-ID löschen |

**Medientypen von `/api/deploy`:** Der Endpunkt akzeptiert ausschließlich `application/json` mit den Pflichtfeldern `name` und `xml` (max. 10 MiB XML). Das XML wird als String eingebettet; ein roher XML-Body wird mit `415` abgelehnt, ein fehlendes Pflichtfeld mit `422`. Beleg: `engine-server/src/server/deploy.rs`; `docs/openapi.yaml` beschreibt denselben Vertrag.

**Beispiel — Deploy mit JSON-Body:**

```bash
jq -Rs '{name:"erster-prozess", xml:.}' erster-prozess.bpmn > deploy.json
curl -s -X POST http://localhost:8081/api/deploy \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  --data-binary @deploy.json
```

```json
{"definition_key":"3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90","version":1}
```

**Beispiel — XML abrufen (roher Text, kein JSON-Objekt):**

```bash
curl -s http://localhost:8081/api/definitions/3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90/xml
```

```xml
<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions ...>
```

**Beispiel — Löschen mit laufenden Instanzen (erwartet 409):**

```bash
curl -s -o /dev/null -w "%{http_code}\n" -X DELETE \
  http://localhost:8081/api/definitions/3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90
# 409
```

### Instances

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `POST` | `/api/start` | Admin | `200`, `400`, `404` (`NoSuchDefinition`) | Instanz per `definition_key` starten |
| `POST` | `/api/start/latest` | Admin | `200`, `400`, `404` | Instanz der neuesten Version per `bpmn_process_id` starten |
| `POST` | `/api/start/timer` | Admin | `200`, `400` | Timer-Start-Event-Instanz starten (wiederholende Intervalle) |
| `GET` | `/api/instances` | Admin | `200` | Alle Instanzen auflisten (Pagination: siehe unten) |
| `GET` | `/api/instances/{id}` | Admin | `200`, `404` (`NoSuchInstance`) | Instanzdetails lesen |
| `DELETE` | `/api/instances/{id}` | Admin | `200`, `404` | Instanz löschen |
| `POST` | `/api/instances/{id}/suspend` | Admin | `200`, `404`, `409` (`InstanceSuspended`) | Instanz pausieren (Timer feuern nicht, Tasks nicht abschließbar) |
| `POST` | `/api/instances/{id}/resume` | Admin | `200`, `404` | Pausierte Instanz fortsetzen |
| `PUT` | `/api/instances/{id}/variables` | Admin | `200`, `404` | Variablen aktualisieren |
| `POST` | `/api/instances/{id}/move-token` | Admin | `200`, `400`, `404` (`NoSuchNode`) | Token auf einen anderen Knoten verschieben (Modification) |
| `POST` | `/api/instances/{id}/migrate` | Admin | `200`, `404`, `422` (`OrphanedToken`) | Instanz auf andere Definitionsversion migrieren |
| `GET` | `/api/instances/{id}/incidents` | Admin | `200`, `404` | Offene und aufgelöste Incidents lesen |
| `POST` | `/api/instances/{id}/incidents/{incident_id}/resolve` | Admin | `200`, `404` (`IncidentNotFound`), `409` (`IncidentAlreadyResolved`) | Incident auflösen |

**Pagination von `GET /api/instances`:** Ohne Query-Parameter liefert der Endpunkt das vollständige Array. Mit `limit` und `offset` liefert der Body eine Seite und der Response-Header `X-Total-Count` die Gesamtzahl. `limit` ist auf `1..=1000` begrenzt; sortiert wird nach `started_at` absteigend. Beleg: `docs/knowledge-base/modules/engine-core/gotchas.md`.

**Beispiel — Instanz starten:**

```bash
curl -s -X POST http://localhost:8081/api/start \
  -H "Content-Type: application/json" \
  -d '{"definition_key":"3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90","variables":{"kunde":"Beispiel GmbH"}}'
```

```json
{"instance_id":"a1b2c3d4-0000-4000-8000-000000000001"}
```

Die Antwort enthält ausschließlich `instance_id`. Den Zustand und den aktuellen Knoten liest du anschließend über `GET /api/instances/{id}`:

```bash
curl -s http://localhost:8081/api/instances/a1b2c3d4-0000-4000-8000-000000000001
```

```json
{"id":"a1b2c3d4-0000-4000-8000-000000000001","state":{"WaitingOnUserTask":{"task_id":"b2c3d4e5-0000-4000-8000-000000000002"}},"current_node":"pruefen","definition_key":"3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90"}
```

**Beispiel — Incident auflösen:**

```bash
curl -s -X POST \
  http://localhost:8081/api/instances/a1b2c3d4-0000-4000-8000-000000000001/incidents/inc-001/resolve \
  -H "Content-Type: application/json" \
  -d '{"resolvedBy":"admin"}'
```

Der Body ist optional. Ein bereits aufgelöster Incident liefert `409`.

**Feldhinweis `bpmn_process_id`:** `POST /api/start/latest` erwartet `bpmn_process_id`; `bpmn_id` bleibt per `#[serde(alias)]` gültig. `business_key` ist auf `StartRequest` und `StartLatestRequest` top-level erlaubt und hat Vorrang vor einem `business_key`-Eintrag in `variables`; die Engine entfernt den Schlüssel dann aus den Variablen. Beleg: `docs/knowledge-base/modules/engine-server/gotchas.md` §„Start-Felder".

### User Tasks

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `GET` | `/api/tasks` | Admin | `200` | Alle offenen User Tasks auflisten |
| `POST` | `/api/complete/{id}` | Admin | `204`, `400`, `404`, `409` (`TaskNotPending`) | User Task abschließen |

**Beispiel:**

```bash
curl -s -X POST http://localhost:8081/api/complete/b2c3d4e5-0000-4000-8000-000000000002 \
  -H "Content-Type: application/json" \
  -d '{"variables":{"bestaetigt":true}}'
```

### Service Tasks

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `GET` | `/api/service-tasks` | Admin | `200` | Alle Service Tasks auflisten |
| `POST` | `/api/service-task/fetchAndLock` | Admin, Worker | `200`, `400`, `401` | Tasks per Long-Polling holen und sperren |
| `POST` | `/api/service-task/{id}/complete` | Admin, Worker | `204`, `404`, `409` (`ServiceTaskLocked`, `ServiceTaskNotLocked`) | Task erfolgreich abschließen |
| `POST` | `/api/service-task/{id}/failure` | Admin, Worker | `204`, `404`, `409` | Task als fehlgeschlagen melden (Retry oder Incident) |
| `POST` | `/api/service-task/{id}/extendLock` | Admin, Worker | `204`, `404`, `409` | Sperre verlängern (`newDuration` in ms) |
| `POST` | `/api/service-task/{id}/bpmnError` | Admin, Worker | `204`, `404`, `409` | Fachlichen Fehler melden (Boundary Error Event) |
| `POST` | `/api/service-task/{id}/retry` | Admin | `204`, `404` | Retries zurücksetzen (Incident-Behandlung) |
| `POST` | `/api/service-task/{id}/resolve` | Admin | `204`, `404` | Task ohne Worker manuell abschließen |

**Warum `fetchAndLock` kein `403` liefert:** Die Route ist für die Worker-Rolle zugelassen; `403` entsteht nur, wenn ein Worker-Key eine **Admin-Route** aufruft. Ein `401` bleibt möglich, wenn das Gate aktiv ist und der Key fehlt oder falsch ist. Vertrag: [`reference/service-task-api.md`](service-task-api.md) §3.

**Beispiel — `fetchAndLock` mit gültiger Lock-Dauer:**

```bash
curl -s -X POST http://localhost:8081/api/service-task/fetchAndLock \
  -H "Content-Type: application/json" \
  -d '{"workerId":"worker-01","maxTasks":1,"topics":[{"topicName":"send-email","lockDuration":30000}]}'
```

**Beispiel — `fetchAndLock` mit Sekunden statt Millisekunden (erwartet 400):**

```bash
curl -s -o /dev/null -w "%{http_code}\n" -X POST http://localhost:8081/api/service-task/fetchAndLock \
  -H "Content-Type: application/json" \
  -d '{"workerId":"worker-01","maxTasks":1,"topics":[{"topicName":"send-email","lockDuration":30}]}'
# 400
```

Validierungsgrenzen und Lock-Semantik: [`reference/service-task-api.md`](service-task-api.md).

### Files

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `POST` | `/api/instances/{id}/files/{var}` | Admin | `200`, `404`, `413` | Datei als Prozessvariable hochladen (Multipart, max. 5 MiB) |
| `GET` | `/api/instances/{id}/files/{var}` | Admin | `200`, `404` | Datei herunterladen |
| `DELETE` | `/api/instances/{id}/files/{var}` | Admin | `200`, `404` | Dateivariable löschen |

### Events, Messages und Timer

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `POST` | `/api/message` | Admin | `200`, `400`, `404` | Message korrelieren |
| `GET` | `/api/messages` | Admin | `200` | Alle offenen Message-Subscriptions auflisten |
| `GET` | `/api/timers` | Admin | `200` | Alle aktiven Timer auflisten |
| `POST` | `/api/timers/process` | Admin | `200` | Abgelaufene Timer manuell verarbeiten |

### History und Archiv

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `GET` | `/api/instances/{id}/history` | Admin | `200`, `404` | Ereignis-Historie einer Instanz (Filter als Query-Parameter) |
| `GET` | `/api/instances/{id}/history/{event_id}` | Admin | `200`, `404` | Ein einzelnes Historien-Ereignis |
| `GET` | `/api/history/instances` | Admin | `200` | Archivierte Instanzen auflisten; Filter `definition_key`, `business_key`, `from`, `to`, `state`, `limit`, `offset` |
| `GET` | `/api/history/instances/{id}` | Admin | `200`, `404` | Einzelne archivierte oder aktive Instanz laden |

**Beispiel — archivierte Instanzen filtern:**

```bash
curl -s "http://localhost:8081/api/history/instances?definition_key=3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90&limit=20&offset=0"
```

### Monitoring, Health und Observability

| Methode | Pfad | Rolle | Statuscodes | Zweck |
|---|---|---|---|---|
| `GET` | `/api/health` | öffentlich | `200` | Liveness — immer `200`, solange der Prozess HTTP annimmt |
| `GET` | `/api/ready` | öffentlich | `200`, `503` | Readiness — `503`, wenn `require_nats && persistence.is_none()` oder `get_storage_info()` fehlschlägt |
| `GET` | `/api/info` | Admin | `200` | Backend-Info (Typ, NATS-URL, Status) |
| `GET` | `/api/monitoring` | Admin | `200` | Engine-Statistiken (Instanzen, Tasks, Storage, Fehler) |
| `GET` | `/api/monitoring/buckets/{bucket}/entries` | Admin | `200`, `404` | KV-Bucket-Einträge auflisten |
| `GET` | `/api/monitoring/buckets/{bucket}/entries/{key}` | Admin | `200`, `404` | Einzelnen KV-Eintrag laden |
| `GET` | `/metrics` | Admin | `200`, `403` | Prometheus-Metriken; **nur gemountet, wenn `prometheus_handle` gesetzt ist** — sonst existiert die Route nicht (nicht einmal `404`) |
| `GET` | `/api/logs` | Admin | `200` | Rollierender Log-Buffer; Query `?level=info&search=error&limit=500` |
| `GET` | `/api/events` | Admin | `200` (SSE-Stream) | Server-Sent Events: `instance_changed`, `task_changed`, `definition_changed` |

**Beispiel — Readiness prüfen:**

```bash
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/ready
# 200 bei funktionierender Persistenz, 503 sonst
```

**SSE-Hinweis:** `GET /api/events` nutzt einen `tokio::sync::broadcast`-Kanal mit Kapazität 256. Ein langsamer Consumer **verliert Ereignisse**; es gibt keinen Catch-up. Das Protokoll ist: Ereignis empfangen → per REST nachladen. Beleg: `docs/knowledge-base/modules/engine-server/gotchas.md` §„SSE is broadcast, NOT queue".

---

## Fehlerantworten

Alle Fehlerantworten sind JSON mit genau einem Feld:

```json
{ "error": "Human-readable error message" }
```

Die Zuordnung von `EngineError` und `AppError` zu HTTP-Status ist in `docs/knowledge-base/modules/engine-server/interfaces.md` §„AppError → HTTP Status Mapping" belegt. Die folgende Tabelle nennt die Status, die ein Client unterscheiden muss.

| Status | Bedeutung | Typischer Auslöser |
|---|---|---|
| `400` | Fehlerhafte Anfrage | Ungültiges XML, ungültige UUID, fehlende Pflichtfelder, Validierungsgrenze verletzt |
| `401` | Nicht authentifiziert | Kein oder falscher Key bei aktivem Gate; fail-closed ohne Key und Nicht-Loopback-Bind |
| `403` | Rolle nicht zugelassen | Gültiger Worker-Key auf einer Admin-Route |
| `404` | Ressource nicht gefunden | Unbekannte Definition, Instanz, Node, Incident-ID; kein gespeichertes BPMN-XML |
| `409` | Konflikt mit dem aktuellen Zustand | Task nicht im erwarteten Zustand, Sperre von einem anderen Worker gehalten, Definition hat noch Instanzen, Incident bereits aufgelöst |
| `413` | Nutzlast zu groß | Body über `MAX_XML_BYTES` (10 MiB) oder Multipart-Upload über `MAX_UPLOAD_BYTES` (5 MiB) |
| `415` | Medientyp nicht unterstützt | Fehlender oder falscher `Content-Type` |
| `422` | Inhaltlich nicht verarbeitbar | Gültiges JSON, aber Schema-Mismatch; Migration mit `OrphanedToken` (ein Token findet im Zielmodell keinen Knoten) |
| `500` | Interner Fehler | Alle übrigen `EngineError`-Varianten; fehlgeschlagenes Speichern des BPMN-XML beim Deploy |

**Abgrenzung der vier Ablehnungsstatus:** `400` ist ein Syntax- oder Validierungsfehler im Inhalt, `413` eine Größenüberschreitung, `415` ein falscher Medientyp, `422` ein formal gültiger, aber inhaltlich nicht passender Body. Beleg: `docs/knowledge-base/modules/engine-server/gotchas.md` §„Alle Fehlerantworten sind `{ "error": ... }`".

---

## Offen

- **Abgleich mit OpenAPI:** Die Tabellen sind gegen die Knowledge Base belegt, aber nicht automatisiert gegen `docs/openapi.yaml` geprüft (Audit AUD-006, AUD-018, AUD-019). **Offen:** CI-Abgleich.
- **Statuscodes für `POST /api/message`, `POST /api/timers/process` und `PUT /api/instances/{id}/variables`:** Nur teilweise belegt.
- **Vollständige Liste der akzeptierten Medientypen:** Für `/api/deploy` ist belegt, dass ausschließlich `application/json` akzeptiert wird (Abschnitt „Definitions"). **Offen:** ob weitere Routen abweichende Medientypen akzeptieren, ist nicht systematisch geprüft.
- **`reference/fehlercodes.md` und `reference/metriken.md`:** Im Dateiplan geführt, aber nicht geschrieben. Die vollständige `EngineError`-Zuordnung und die Metrikliste fehlen deshalb. Beide Seiten sind hier **nicht verlinkt**.
- **Konfiguration:** Die kanonische Referenz für Umgebungsvariablen, Limits und Retention steht in [`reference/konfiguration.md`](konfiguration.md).
- **Versionsstand:** Diese Seite nennt keinen Produktversionsstand („gültig ab"). `bpmninja/README.md` nennt `0.7.24` für Rust/Desktop und `1.0.0` für den npm-Client; die Zuordnung steht aus.
- **Ankerlinks:** Die internen Verweise auf `#fehlerantworten`, `#authentifizierung` und `#rollenmatrix-für-service-task-routen` zeigen auf Überschriften mit Umlauten. Die Linkprüfung prüft Anker nicht. **Offen:** Ankerauflösung im Zielsystem einmalig prüfen.
