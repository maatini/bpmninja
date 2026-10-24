---
title: Konfiguration und Umgebungsvariablen
description: Kanonische Referenz aller Umgebungsvariablen der BPMNinja-Engine — Port und Bind-Adresse, Authentifizierung, NATS-Durability, Body-Limits, CORS, Logging, Retry-Queue, Timer, Rhai-Grenzen sowie Stream-Retention und KV-Buckets — mit Default, Gültigkeitsbereich und Wirkung.
audience:
  - Betriebsteams und Plattform-Administratoren
  - Entwickler und Integratoren
  - Beitragende und Maintainer
prerequisites:
  - Laufende oder startbereite BPMNinja-Engine
  - Grundverständnis von Umgebungsvariablen und Prozessstarts
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/cli.md
  - reference/rest-api.md
  - how-to/konfiguration-anpassen.md
  - explanation/architektur.md
  - explanation/konzepte.md
---

# Konfiguration und Umgebungsvariablen

Diese Seite ist die kanonische Referenz für alle Umgebungsvariablen, die die BPMNinja-Engine beim Start liest. Sie beschreibt **was gilt** — Variablenname, Default, Gültigkeitsbereich und Wirkung. Die Schritt-für-Schritt-Einrichtung steht in [`how-to/konfiguration-anpassen.md`](../how-to/konfiguration-anpassen.md).

**Beleglage:** Alle Werte sind gegen den Quelltext geprüft (`engine-server/src/main.rs`, `engine-server/src/server/state.rs`, `engine-core/src/engine/retry_queue.rs`, `engine-core/src/scripting/runner.rs`, `persistence-nats/src/client.rs`, Stand Version 0.7.24). Ein Wert, der nur aus einer Knowledge-Base-Seite stammt, ist als solcher gekennzeichnet. Ausnahme: Für drei Namen der persistierten Strukturen (Abschnitt 9) widersprechen sich Quelltext und Knowledge Base; sie sind dort als offen markiert.

## Konventionen

- **Boolesche Variablen** akzeptieren je nach Variable unterschiedliche Schreibweisen; die jeweilige Tabelle nennt sie. Groß-/Kleinschreibung und umgebende Leerzeichen sind unerheblich.
- **Größenangaben** sind Bytes, sofern nicht anders angegeben.
- **Dauern** sind Millisekunden (`_MS`) oder Tage (`_DAYS`); die Einheit steht im Variablennamen.
- **Geheimnisse** sind mit dem Zeichen 🔑 in der Spalte `Typ` markiert. Sie gehören nicht in ein Repository und nicht in Logs.
- Ungültige Werte werden — außer bei den ausdrücklich genannten Klemmen — auf den Default zurückgesetzt; die Engine startet weiter.

Eine Variable ohne Eintrag in dieser Seite wird von der Engine nicht gelesen. Ausnahme: Variablen, die nur in Tests oder im Compose-Stack verwendet werden, sind am Ende separat gelistet.

---

## 1. HTTP-Server

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `PORT` | `8081` | Port | Listen-Port des Axum-Servers. Ungültige Werte werden nicht ersetzt, sondern führen beim Binden zu einem Startfehler. |
| `BIND_ADDR` | `0.0.0.0` | Host oder IP | Listen-Adresse. Ein leerer Wert wird wie „nicht gesetzt" behandelt. |
| `BIND_ADDRESS` | — | Host oder IP | Alias für `BIND_ADDR`. `BIND_ADDR` hat Vorrang, wenn beide gesetzt sind. |
| `TIMER_INTERVAL_MS` | `1000` | Millisekunden | Abstand, in dem der Hintergrund-Scheduler abgelaufene Timer verarbeitet. `0` ist kein sinnvoller Wert und wird nicht abgefangen. |

**Wirkung der Bind-Adresse auf die Authentifizierung:** `127.0.0.0/8`, `::1` und `localhost` gelten als Loopback. Bindet die Engine an eine andere Adresse — auch an `0.0.0.0` — und ist kein Key gesetzt, ist das Auth-Gate aktiv und alle `/api`-Routen außer `/api/health` und `/api/ready` antworten mit `401`. Das ist beabsichtigt (fail-closed).

```bash
# Nur lokal erreichbar
PORT=8081 BIND_ADDR=127.0.0.1 cargo run -p engine-server
```

---

## 2. Authentifizierung und Rollen

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `BPMNINJA_API_KEY` | nicht gesetzt | Zeichenkette 🔑 | Admin-Key. Erlaubt alle Routen inklusive `retry`, `resolve`, `/metrics`, Deploy, Instanzen und Definitionen. Leere oder nur aus Leerzeichen bestehende Werte gelten als „nicht gesetzt". |
| `BPMNINJA_WORKER_API_KEY` | nicht gesetzt | Zeichenkette 🔑 | Worker-Key. Erlaubt ausschließlich `fetchAndLock`, `complete`, `failure`, `extendLock` und `bpmnError`. Auf allen anderen Routen antwortet die Engine mit `403`. |
| `BPMNINJA_ALLOW_UNAUTHENTICATED` | `false` | `1`, `true`, `yes`, `on` | Bewusster Opt-out aus dem fail-closed-Gate. Nur für lokale Entwicklung. In einer von außen erreichbaren Umgebung ist das ein Sicherheitsfehler. |

**Rollenmatrix:**

| Route | ohne Key (Loopback) | Worker-Key | Admin-Key |
|---|---|---|---|
| `/api/health`, `/api/ready` | erlaubt | erlaubt | erlaubt |
| `/api/service-task/fetchAndLock` sowie `/api/service-task/{id}/complete`, `failure`, `extendLock`, `bpmnError` | erlaubt | erlaubt | erlaubt |
| alle übrigen `/api`-Routen | erlaubt | `403` | erlaubt |
| alle `/api`-Routen bei Nicht-Loopback-Bind | `401` | rollenabhängig | erlaubt |

Fehlversuche zählen in `bpmn_auth_failures_total{reason}` mit den Gründen `invalid_or_missing_key` und `worker_role_forbidden`.

---

## 3. NATS und Durability

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `NATS_URL` | `nats://localhost:4222` | URL | Broker-Adresse. Im Compose-Stack mit Credentials: `nats://bpmninja:bpmninja@nats:4222`. |
| `REQUIRE_NATS` | `true` | Boolesch (`0`, `false`, `no`, `off` schalten ab) | Fail-closed: Ohne erreichbaren NATS bricht der Start ab. Mit `false` startet die Engine im In-Memory-Modus; alle Daten gehen beim Neustart verloren, `/api/ready` bleibt `503`. |

**Warum der Default `true` ist:** Ein stiller Wechsel in den In-Memory-Modus würde einen Datenverlust wie einen erfolgreichen Start aussehen lassen. Die Begründung steht in [`explanation/architektur.md`](../explanation/architektur.md).

```bash
export NATS_URL="nats://bpmninja:bpmninja@nats:4222"
export REQUIRE_NATS=true
```

---

## 4. Body-Limits und CORS

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `MAX_XML_BYTES` | `10485760` (10 MiB) | Bytes, > 0 | Limit für JSON-/XML-Bodies und den Deploy-Cap. Überschreitung → `413`. Ungültige oder `0`-Werte fallen auf den Default zurück. |
| `MAX_UPLOAD_BYTES` | `5242880` (5 MiB) | Bytes, > 0 | Limit für Multipart-Datei-Uploads an Instanzen. Überschreitung → `413`. |
| `CORS_ORIGINS` | Tauri- und Lokal-Allowlist (siehe unten) | Kommagetrennte Origin-Liste | Erlaubte Browser-Origins. Ein leerer Wert fällt auf die Allowlist zurück. `*` erlaubt jeden Origin und ist ausschließlich für Entwicklung gedacht. |

**Default-Allowlist:** `http://localhost:1420`, `http://127.0.0.1:1420`, `tauri://localhost`, `https://tauri.localhost`.

```bash
export CORS_ORIGINS="https://bpmn.example.com"
export MAX_XML_BYTES=10485760
export MAX_UPLOAD_BYTES=5242880
```

---

## 5. Logging

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `RUST_LOG` | `info` | `tracing`-Filterausdruck | Filter für Konsole und Log-Buffer, z. B. `debug`, `warn`, `engine_server=trace`. |
| `LOG_FORMAT` | `text` | `text` oder `json` | Ausgabeformat der Konsolenzeilen. Andere Werte werden als `text` behandelt. |
| `LOG_FILE` | `engine_logs.jsonl` | Pfad oder `off` | Datei-Fallback für den Log-Buffer. `off` deaktiviert die Dateipersistenz; der In-Memory-Puffer bleibt aktiv. |

**Log-Buffer:** 5 000 Einträge im Speicher, 50 000 im NATS-Stream `ENGINE_LOGS`, zusätzlich die Datei aus `LOG_FILE`. Die Engine rotiert die Datei nicht selbst. `GET /api/logs` liest den Puffer.

```bash
export RUST_LOG=info
export LOG_FORMAT=json
export LOG_FILE=/var/log/bpmninja/engine_logs.jsonl
```

---

## 6. Persistenz-Retry und Shutdown

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `PERSISTENCE_RETRY_QUEUE_CAPACITY` | `10000` | Anzahl Jobs | Größe der begrenzten Hintergrund-Queue für fehlgeschlagene Persistenzschreibvorgänge. Ist sie voll, werden neue Jobs verworfen und als `bpmn_persistence_retry_dropped_total` gezählt. |
| `PERSISTENCE_SHUTDOWN_DRAIN_MS` | `5000` | Millisekunden | Zeitbudget, das die Engine beim Herunterfahren für das Leeren der Retry-Queue aufwendet. |

**Die Grenze ist bewusst:** Die Engine puffert nicht unbegrenzt. Ein verworfener Job ist ein verlorener Schreibversuch, kein verlorener Endzustand — der Retry-Worker liest den aktuellen Zustand vor dem Schreiben neu. Wer die Garantie für jeden Zwischenstand braucht, alarmiert den Zähler.

---

## 7. Rhai-Skripte

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `RHAI_MAX_OPERATIONS` | `50000` | Operationen | Maximale Anzahl ausgewerteter Rhai-Operationen pro Skriptlauf. Überschreitung bricht das Skript ab. |
| `RHAI_MAX_MEMORY_BYTES` | `2097152` (2 MiB) | Bytes | Speicherlimit der Rhai-Instanz. |
| `RHAI_TIMEOUT_MS` | `1000` | Millisekunden | Maximale Laufzeit eines Skripts. |

Die Limits gelten für Script Tasks und für Execution Listener. Ein Skript, das sie überschreitet, erzeugt einen Incident; die Engine bleibt ansprechbar.

---

## 8. Stream-Retention

| Variable | Default | Typ | Wirkung |
|---|---|---|---|
| `BPMNINJA_EVENT_RETENTION_DAYS` | `7` | Tage | `max_age` des Streams `WORKFLOW_EVENTS`. `0` bedeutet unbegrenzt. |
| `BPMNINJA_HISTORY_RETENTION_DAYS` | `30` | Tage | `max_age` des Streams `WORKFLOW_HISTORY`. `0` bedeutet unbegrenzt. |
| `BPMNINJA_STREAM_MAX_BYTES` | `1073741824` (1 GiB) | Bytes | `max_bytes` beider Workflow-Streams. `0` bedeutet unbegrenzt; negative Werte fallen auf den Default zurück. |
| `BPMNINJA_STREAM_REPLICAS` | `1` | Anzahl | `num_replicas` beider Streams. Werte über `5` werden auf `5` geklemmt, `0` wird auf `1` geklemmt. |

**Der Abgleich läuft bei jedem Start.** Die Engine vergleicht `max_age`, `max_bytes` und `num_replicas` mit den gewünschten Werten und ruft bei Abweichung `update_stream` auf. Subjects, Storage-Typ und Discard-Policy bleiben unverändert. Der **erste Start mit niedrigeren Limits wendet diese sofort an**; NATS verwirft dann ältere oder größere Nachrichten. Vor einem Upgrade mit bestehender Historie deshalb `0` setzen oder ausreichend große Werte wählen.

```bash
# Bestehende Historie beim ersten Start nicht beschneiden
export BPMNINJA_EVENT_RETENTION_DAYS=0
export BPMNINJA_HISTORY_RETENTION_DAYS=0
export BPMNINJA_STREAM_MAX_BYTES=0
export BPMNINJA_STREAM_REPLICAS=1
```

Ohne `STREAM.UPDATE`-Berechtigung schlägt der Start mit der Serverursache fehl, statt die alten Limits stillschweigend zu behalten.

---

## 9. Persistierte Strukturen (NATS JetStream)

Diese Namen sind der Vertrag für Betriebsprüfungen. Sie folgen `persistence-nats/src/client.rs` (Version 0.7.24). Ein `bpm_*`-Präfix existiert in dieser Version nicht.

**KV-Buckets (acht):**

| Bucket | Inhalt |
|---|---|
| `tokens` | Token-Zustand je Prozessinstanz (JSON) |
| `instances` | Aktive `ProcessInstance`-Zustände (JSON) |
| `definitions` | `ProcessDefinition`-Metadaten (JSON) |
| `user_tasks` | Offene User Tasks (JSON) |
| `service_tasks` | Offene Service Tasks (JSON) |
| `timers` | Offene Timer (JSON) |
| `messages` | Offene Message Catches (JSON) |
| `history_instances` | Archivierte abgeschlossene Instanzen (JSON) |

**Object Stores (zwei):**

| Store | Inhalt |
|---|---|
| `bpmn_xml` | Original-BPMN-2.0-XML der Deploys |
| `instance_files` | Binäre Datei-Anhänge an Instanzvariablen |

**Streams (drei):**

| Stream | Subjects | Zweck |
|---|---|---|
| `WORKFLOW_EVENTS` | `WORKFLOW_EVENTS.*` | Laufende Ereignisse, Retention über `BPMNINJA_EVENT_RETENTION_DAYS` |
| `WORKFLOW_HISTORY` | `history.instance.*` | Event-Sourcing der Instanz-Historie, Retention über `BPMNINJA_HISTORY_RETENTION_DAYS` |
| `ENGINE_LOGS` | (Log-Publishing) | Log-Buffer, maximal 50 000 Nachrichten |

**Abgrenzung:** Abgeschlossene Instanzen liegen in `history_instances`, nicht mehr in `instances`. Wer Buckets manuell prüft, verwendet die Namen aus dieser Tabelle.

> **Warnung — vor Betriebsnutzung bestätigen:** Die Knowledge Base (`persistence/gotchas.md`) nennt für drei Namen abweichende Bezeichnungen: `message_catches` statt `messages`, `history` statt `history_instances` und zusätzlich `history_completed_instances` als Archiv-Bucket. Diese Seite folgt dem Adapter (`client.rs`); ein Grep im Adapter (`create_key_value`, `create_object_store`) entscheidet. Ein falscher Bucket-Name ist still: Die Abfrage liefert „leer", nicht „Fehler". Siehe **Offen**.

---

## 10. Nur für Tests und Compose

| Variable | Verwendung |
|---|---|
| `REQUIRE_NATS_TESTS` | Test-Gate: NATS-Integrationstests laufen nur, wenn diese Variable gesetzt ist. Keine Laufzeitwirkung im Server. |
| `NATS_URL` im Compose-Stack | Der Stack verlangt Credentials (`nats://bpmninja:bpmninja@nats:4222`); ein unauthentifiziertes `nats://localhost:4222` schlägt gegen diesen Stack fehl. |

---

## Prüfung nach dem Start

| Prüfung | Befehl | Erwartet |
|---|---|---|
| Liveness | `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/health` | `200` (leerer Body) |
| Readiness | `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/ready` | `200` mit NATS, `503` ohne |
| Auth fail-closed | `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/definitions` | `401` bei Nicht-Loopback-Bind ohne Key |
| Worker-Rolle | derselbe Aufruf mit `X-API-Key: $BPMNINJA_WORKER_API_KEY` | `403` |
| Retention | `curl -s http://127.0.0.1:8222/jsz?streams=true` | Limits wie gesetzt |
| Buckets | `curl -s http://127.0.0.1:8222/jsz?kv=true` | acht Buckets aus Abschnitt 9 |

---

## Offen

- **Bucket-Namen (drei Punkte):** `messages` und `history_instances` folgen `persistence-nats/src/client.rs`. Die Knowledge Base nennt stattdessen `message_catches`, `history` und `history_completed_instances`. **Offen:** quellseitig bestätigen (technischer Review B-02). Bis dahin gilt der Warnkasten in Abschnitt 9.
- **Portvalidierung:** `PORT` wird nicht validiert. Ein ungültiger Wert führt erst beim Binden zu einem Startfehler; die Fehlermeldung stammt aus dem Betriebssystem, nicht aus der Engine.
- **`TIMER_INTERVAL_MS=0`:** Ein Wert von `0` wird nicht abgefangen. Der Scheduler läuft dann ohne Pause; das Verhalten ist nicht als Vertrag dokumentiert.
- **Replikation:** `BPMNINJA_STREAM_REPLICAS` beschreibt die Stream-Replikation. Eine Aussage zur Replikation der KV-Buckets ist im Quelltext nicht belegt und fehlt hier bewusst.
- **Metriken:** Eine vollständige Liste der Prometheus-Metriken (`reference/metriken.md`) ist nicht geschrieben. Genannt sind nur die Zähler, die in dieser Seite eine Rolle spielen.
