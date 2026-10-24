---
title: Verifikationsbericht — Live-Lauf und Rendering
description: Belegt die Abnahme von B-06 (Code-Beispiele) und B-09 (Mermaid-Rendering) mit Datum, Build, Befehlen, tatsächlichen Antworten und den daraus korrigierten Vertragsabweichungen.
audience:
  - Beitragende und Maintainer
  - Stakeholder und Entscheider
prerequisites:
  - Laufende BPMNinja-Engine mit NATS JetStream
  - Mermaid CLI (`mmdc`)
content_type: reference
status: reviewed
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _qa/qa-report.md
  - _qa/content-manifest.md
  - reference/rest-api.md
  - reference/service-task-api.md
  - how-to/external-task-worker-schreiben.md
---

# Verifikationsbericht — Live-Lauf und Rendering

Datum: **2026-10-24**. Zweck: die beiden offenen Nachweise aus dem QA-Lauf schließen — **B-06** (kein Code-Beispiel gegen eine laufende Engine geprüft) und **B-09** (kein Rendering-Nachweis für die Mermaid-Diagramme).

## 1. Umgebung

| Element | Wert |
|---|---|
| Engine-Binary | `target/debug/engine-server`, gebaut mit `devbox run -- cargo build -p engine-server` (rustc 1.99.0) |
| Start | `BIND_ADDR=127.0.0.1 PORT=8091 REQUIRE_NATS=true NATS_URL=nats://localhost:4222 LOG_FILE=off RUST_LOG=warn target/debug/engine-server` |
| Broker | NATS JetStream, `nats://localhost:4222` |
| Health | `GET /api/health` → `200`, leerer Body |
| Mermaid | `/Users/martinrichardt/.devbox/bin/mmdc` (Mermaid CLI 11.x) |

Alle Aufrufe unten liefen gegen `http://127.0.0.1:8091`. Die Dokumentation verwendet `localhost:8081` als Default; Pfad- und Statusverhalten sind identisch.

## 2. REST-Vertrag (B-06)

### 2.1 Deploy

```bash
jq -Rs '{name:"worker-prozess", xml:.}' worker-prozess.bpmn > deploy.json
curl -s -X POST http://127.0.0.1:8091/api/deploy \
  -H "Content-Type: application/json" --data-binary @deploy.json
```

| Variante | Ergebnis |
|---|---|
| JSON `{name, xml}` | `200` — `{"definition_key":"39613d30-4179-4069-9b8c-ae9b79771d0b","version":1}` |
| Rohes XML (`Content-Type: application/xml`) | `415` |

**Konsequenz für die Doku:** Alle Deploy-Beispiele nutzen JSON. Das frühere Beispiel mit rohem XML war falsch. Beleg im Code: `engine-server/src/server/deploy.rs` (`ApiJson<DeployRequest>`).

### 2.2 Start und Instanz

| Aufruf | Tatsächliche Antwort |
|---|---|
| `POST /api/start` | `200`, `{"instance_id":"616a5c9f-b555-455d-aad6-b18ed9e4a997"}` — **nur** die ID |
| `GET /api/instances/{id}` | `id`, `definition_key`, `business_key`, `state`, `current_node`, `variables`, `tokens`, `incidents`, `started_at`, `completed_at`, … |
| `GET /api/instances/{id}` im Wartezustand | `"state":{"WaitingOnServiceTask":{"task_id":"…"}}` |
| `GET /api/tasks` | `task_id`, `instance_id`, `node_id`, `assignee`, `token_id`, `created_at`, `business_key` |
| `POST /api/complete/{task_id}` | `204`, leerer Body |

### 2.3 Service-Task-Worker

Prozess: `send-email` mit Boundary Error Event (`INVALID_ADDRESS`) auf `fix-address`.

| Schritt | Ergebnis |
|---|---|
| `fetchAndLock` mit `lockDuration: 30000` | `200`, Task mit `retries: 3`, `worker_id`, `lock_expiration`, `variables_snapshot` |
| `complete` auf gesperrtem Task | `204` |
| `complete` auf ungesperrtem Task | `409` — `{"error":"Service task '…' is not locked"}` |
| `failure` mit `retries: 0` | `204`; Incident `kind: "service_task"`, `message: "SMTP nicht erreichbar"`; Instanz bleibt `WaitingOnServiceTask` |
| `retry` | `204`; `retries` wieder `3`, `error_message` geleert; Incident bleibt offen |
| `POST /api/instances/{id}/incidents/{incident_id}/resolve` | `200`; `resolved_at` und `resolved_by` gesetzt |
| `bpmnError` mit passendem Code | `204`; Instanz läuft über `adresse-korrigieren` nach `ende-fehler`, kein Incident |
| `bpmnError` ohne passendes Boundary Event | `204`; Incident `kind: "service_task"`, `message: "Unhandled BPMN error 'NICHT_GEFANGEN'"` |
| `extendLock` mit `newDuration: 60000` | `204` |
| `extendLock` mit `newDuration: 30` | `204` — **keine** 400-Validierung; der Handler rundet auf 1 s auf |
| `fetchAndLock` mit `lockDuration: 30` | `400` — `topics[0].lockDuration must be between 1000 and 3600000 ms, got 30` |
| `fetchAndLock` mit `maxTasks: 0` | `400` — `maxTasks must be between 1 and 100, got 0` |
| `resolve` auf ungesperrtem Task | `204`; Task abgeschlossen, Incident bleibt offen |

**Konsequenzen für die Doku:** `extendLock` hat keinen 400-Pfad (Tabelle korrigiert); ein ungefangener `bpmnError` erzeugt `kind: "service_task"` statt `bpmn_error` (How-to und Referenz korrigiert); `retries` startet bei `3` (neu dokumentiert); `resolve` schließt den Task, löst den Incident aber nicht automatisch auf.

### 2.4 Body-Limit

```bash
python3 -c 'import json; open("zu-gross.json","w").write(json.dumps({"name":"zu-gross","xml":"x"*11000000}))'
curl -s -X POST http://127.0.0.1:8091/api/deploy \
  -H "Content-Type: application/json" --data-binary @zu-gross.json
```

Ergebnis: `413` — `{"error":"Failed to buffer the request body: length limit exceeded"}`. Der Fehler kommt aus dem Body-Limit (10 MiB), bevor der Handler die XML-Länge prüft.

### 2.5 Abweichung zur OpenAPI-Spezifikation

`docs/openapi.yaml` nennt für `POST /api/service-task/{id}/bpmnError` den Status `400`, wenn kein Boundary Event existiert. Der Live-Lauf und `engine-server/src/server/tasks.rs` zeigen `204` mit anschließendem Incident. Die Dokumentation folgt dem Verhalten der Engine und markiert die Abweichung in `reference/service-task-api.md`. Die OpenAPI-Datei wurde in diesem Lauf **nicht** geändert.

## 3. Mermaid-Rendering (B-09)

Alle Mermaid-Blöcke wurden aus den Markdown-Dateien extrahiert und einzeln gerendert:

```bash
for f in *.mmd; do mmdc -i "$f" -o "${f%.mmd}.svg"; done
```

| Quelle | Blöcke | Ergebnis |
|---|---:|---|
| `_architecture/diagram-plan.md` | 5 | 5/5 SVG |
| `explanation/architektur.md` | 3 | 3/3 SVG |
| `explanation/konzepte.md` | 1 | 1/1 SVG |
| `reference/service-task-api.md` | 1 | 1/1 SVG |
| **Summe** | **10** | **10/10 SVG** |

Zusätzlich geprüft: `verify_mermaid` (Fenced-Block-Typ, Tabulatoren) meldet keine Befunde. Die Diagramme D1–D5 sind in den Zielseiten eingefügt.

## 4. Link- und Ankerprüfung

Eigenes Prüfskript über alle 27 Markdown-Dateien: relative Dateilinks und Anker, GitHub-Slug-Regeln, ohne Fenced Code und Inline-Code.

- Geprüfte interne Links: 57 Anker
- Defekte: **0**
- Zuvor defekt und korrigiert: `#fehler-2-apiready-…` (README, Tutorial), `#fehler-3-bpmnerror-…` (README), `..//explanation/konzepte.md` (Service-Task-API)

## 5. Was dieser Bericht nicht belegt

- Den TypeScript-Client `@bpmninja/external-task-client` — kein veröffentlichtes Paket installiert; der How-to-Code folgt der Knowledge-Base-API.
- Das `autoExtendLock`-Verhalten bei 80 % — Client-Feature, nicht serverseitig geprüft.
- Die OpenAPI-Spezifikation selbst — sie bleibt in vier Punkten unverändert abweichend (siehe §2.5).
- Last-, Timeout- und Fehlerpfade jenseits der oben genannten Statuscodes.
