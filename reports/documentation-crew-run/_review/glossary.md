---
title: Glossar — BPMNinja-Dokumentation
description: Verbindliche Begriffsliste der BPMNinja-Dokumentation mit Definition, zulässigen Synonymen, abzugrenzenden Begriffen und einem Beispiel je Eintrag; Grundlage der Terminologieprüfung und öffentliche Nachschlageseite.
audience:
  - Einsteiger und Lernende
  - Entwickler und Integratoren
  - Betriebsteams und Plattform-Administratoren
  - Stakeholder und Entscheider
  - Beitragende und Maintainer
prerequisites:
  - Kenntnis der Informationsarchitektur unter _architecture/information-architecture.md
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _review/style-guide.md
  - _review/style-report.md
  - reference/README.md
  - explanation/konzepte.md
---

# Glossar — BPMNinja-Dokumentation

Dieses Glossar ist die **verbindliche** Begriffsliste für alle veröffentlichten Seiten. Es hat Vorrang vor Formulierungen in einzelnen Seiten. Wenn eine Seite einen Begriff anders verwendet als hier definiert, ist die Seite zu korrigieren — nicht das Glossar.

**Aufbau je Eintrag:** Definition · Synonyme (zulässig) · Abzugrenzen von · Beispiel.

**Regel:** Ein Konzept, ein Begriff. Stille Synonyme sind unzulässig. Ein Synonym ist nur zulässig, wenn es in der Zeile **Synonyme** steht.

**Sprungziele ohne Anker:** Die Übersicht nennt Abschnitte als Text, nicht als Link (Style Guide R6.7) — Anker auf Überschriften mit Umlauten sind nicht geprüft.

| Begriff | Abschnitt | Verwechslungsgefahr mit |
|---|---|---|
| Process Definition | 1. Ausführung und Zustand | Process Instance, BPMN-Datei |
| Process Instance | 1. Ausführung und Zustand | Process Definition, `instance_id` |
| Token | 1. Ausführung und Zustand | Task, `token_id` |
| Wait State | 1. Ausführung und Zustand | Incident, Suspend |
| User Task | 1. Ausführung und Zustand | Service Task, Manual Task |
| Call Activity | 1. Ausführung und Zustand | Embedded Sub-Process |
| `bpmn_process_id` / `definition_key` / `instance_id` | 1. Ausführung und Zustand | einander, „Prozess-ID“ |
| Service Task | 2. Service Task und External Task | External Task |
| External Task | 2. Service Task und External Task | Service Task |
| Fetch-and-Lock | 2. Service Task und External Task | Polling, Long-Polling |
| Topic | 2. Service Task und External Task | Queue-Name, `topicName` |
| `lockDuration` / `newDuration` | 2. Service Task und External Task | Timeout, `asyncResponseTimeout` |
| `bpmnError` / `failure` | 2. Service Task und External Task | einander, Incident |
| Idempotenz / At-least-once | 2. Service Task und External Task | Exactly-once, Retry |
| Incident | 3. Fehlerzustände und Reparatur | Fehler, Exception |
| Boundary Error Event | 3. Fehlerzustände und Reparatur | Error End Event, `bpmnError` |
| Error End Event | 3. Fehlerzustände und Reparatur | Boundary Error Event |
| `OrphanedToken` | 3. Fehlerzustände und Reparatur | Token, Incident |
| Migration / Modification | 3. Fehlerzustände und Reparatur | einander |
| Flattening / JoinBarrier | 3. Fehlerzustände und Reparatur | Call Activity, Gateway |
| Rhai | 3. Fehlerzustände und Reparatur | JavaScript-Engine |
| Fehlerformat | 4. HTTP-Vertrag | Erfolgsantwort |
| SSE-Ereignis / Catch-up | 4. HTTP-Vertrag | WebSocket, Polling |
| Front-Matter-Felder | 4. HTTP-Vertrag | HTTP-Header |
| fail-closed | 5. Konfiguration und Betrieb | fail-open, Fehler |
| Long-Polling / Backoff | 5. Konfiguration und Betrieb | Sperre, Retry |
| `REQUIRE_NATS` / `BIND_ADDR` | 5. Konfiguration und Betrieb | `NATS_URL`, `PORT` |
| KV-Bucket / Object Store / Stream | 5. Konfiguration und Betrieb | einander |
| Retention | 5. Konfiguration und Betrieb | Backup |
| Readiness / Health | 5. Konfiguration und Betrieb | einander |

---

## 1. Ausführung und Zustand

### Process Definition

Das deployte, **unveränderliche** Prozessmodell. Es entsteht aus einer BPMN-XML-Datei und trägt einen `definition_key` (UUID) und eine `version`.

- **Synonyme:** Definition (nach Erstnennung), Prozessdefinition.
- **Abzugrenzen von:** Process Instance (der Lauf), BPMN-Datei (die Quelle vor dem Deploy), `bpmn_process_id` (die ID im XML).
- **Beispiel:** „Ein erneuter Deploy derselben `bpmn_process_id` erzeugt `version: 2`, nicht eine Änderung an Version 1.“

### Process Instance

Ein konkreter Lauf einer Process Definition mit eigenem Variablenzustand und eigenen Tokens.

- **Synonyme:** Instanz (nach Erstnennung), Prozessinstanz.
- **Abzugrenzen von:** Process Definition, `instance_id` (der Bezeichner, nicht der Lauf).
- **Beispiel:** „Die Instanz bleibt im Zustand `WaitingOnServiceTask`.“

### Token

Die Ausführungsmarke, die genau eine Position im Prozessgraph repräsentiert. Ein linearer Prozess hat ein Token; ein Parallel-Gateway erzeugt mehrere.

- **Synonyme:** keines. „Ausführungsmarke“ ist eine Erläuterung, kein Synonym.
- **Abzugrenzen von:** Task (ein Knoten), `token_id` (der Bezeichner).
- **Beispiel:** „Die Engine setzt das Token hinter den User Task und erreicht den End Event.“

### Wait State

Ein Zustand, in dem die Engine anhält, den Zustand persistiert und auf ein externes Ereignis wartet (User Task, Service Task, Timer, Message, Call Activity).

- **Synonyme:** Wartezustand (nur als Erläuterung im Fließtext).
- **Abzugrenzen von:** Incident (Fehlerzustand), Suspend (administratives Anhalten).
- **Beispiel:** „Der Zustand `WaitingOnUserTask` ist das erwartete Ergebnis: Die Engine hat den Wait State persistiert.“

### User Task

Ein BPMN-Knoten, an dem die Engine auf einen **Menschen** wartet. Abschluss ausschließlich über `POST /api/complete/{id}`; es gibt keine Tasklist und keine Formular-Engine.

- **Synonyme:** keines. Nicht „Manual Task“ — der wird zwar als `UserTask` abgebildet, ist aber ein eigenes Element.
- **Abzugrenzen von:** Service Task (wartet auf einen Worker), Script Task (läuft in der Engine).
- **Beispiel:** `<bpmn:userTask id="bestaetigen" data-assignee="tutorial-leser" />`

### Call Activity

Ein BPMN-Knoten, der eine **global deployte Definition** als eigene Kind-Instanz aufruft. Sie ist der belegte Weg zu echtem Runtime-Scoping.

- **Synonyme:** keines.
- **Abzugrenzen von:** Embedded Sub-Process (wird beim Parsen geflattet, ohne eigenes Scoping).
- **Beispiel:** „Wer Runtime-Scoping braucht, verwendet eine Call Activity — nicht einen Embedded Sub-Process.“

### `bpmn_process_id` / `definition_key` / `instance_id`

Drei verschiedene Bezeichner: `bpmn_process_id` ist das `id`-Attribut im XML und bestimmt die Versionskette; `definition_key` ist die vom Server vergebene UUID **einer Version**; `instance_id` ist die UUID **eines Laufs**.

- **Synonyme:** keines. „Prozess-ID“ ist gesperrt, weil unklar bleibt, welcher der drei gemeint ist.
- **Abzugrenzen von:** einander. `token_id` ist ein vierter, eigener Bezeichner.
- **Beispiel:** „Zwei Deploys mit derselben `bpmn_process_id` ergeben Version 1 und Version 2 — zwei `definition_key`s.“

---

## 2. Service Task und External Task

### Service Task

Der **Knoten im BPMN-Modell**, der Arbeit repräsentiert, die nicht die Engine ausführt. Er trägt ein Topic.

- **Synonyme:** keines.
- **Abzugrenzen von:** External Task (die Arbeitseinheit in der Queue), Script Task (Rhai in der Engine), User Task (wartet auf einen Menschen).
- **Beispiel:** `<bpmn:serviceTask id="send-email" data-topic="send-email" />`

### External Task

Die **Arbeitseinheit in der Fetch-and-Lock-Queue**, die aus einem Service Task entsteht, sobald die Engine ihn erreicht. Ein Worker sperrt sie, arbeitet sie ab und meldet das Ergebnis zurück.

- **Synonyme:** keines. Nicht „Service Task“ — das ist der Modellknoten.
- **Abzugrenzen von:** Service Task.
- **Beispiel:** „Der Service Task ist der Knoten im Modell, der External Task ist der Eintrag, den ein Worker sperrt.“

### Fetch-and-Lock

Das Muster, mit dem ein Worker Arbeit holt: `fetchAndLock` liefert Tasks **und** setzt gleichzeitig die Sperre (`lockDuration`). Holen und Sperren sind eine Operation.

- **Synonyme:** keines. Nicht „Polling“ — Polling ohne Sperre gäbe denselben Task an mehrere Worker.
- **Abzugrenzen von:** Long-Polling (das Warteverhalten des Aufrufs), SSE (Server pusht).
- **Beispiel:** „Fetch-and-Lock ist der Grund, warum ein Task `worker_id` und `lock_expiration` trägt.“

### Topic

Der Name, unter dem ein Worker External Tasks eines Service Task abonniert. Auflösung: `data-topic` → `data-handler` → `camunda:topic` → Node-ID.

- **Synonyme:** keines. Nicht „Queue-Name“, nicht „Task-Typ“.
- **Abzugrenzen von:** `topicName` (der Feldname im `fetchAndLock`-Request).
- **Beispiel:** „Das Topic im Worker muss exakt dem `data-topic` im BPMN entsprechen.“

### `lockDuration` / `newDuration`

Die Sperrdauer eines External Task in **Millisekunden**. `lockDuration` steht im `fetchAndLock`-Request, `newDuration` im `extendLock`-Request. Nur `fetchAndLock` validiert 1 000 bis 3 600 000 ms; `extendLock` prüft nicht und klemmt die umgerechneten Sekunden auf 1..=3600.

- **Synonyme:** keines. Nicht „Lock-Zeit“, nicht „Timeout“.
- **Abzugrenzen von:** `asyncResponseTimeout` (Long-Poll-Dauer, keine Sperre).
- **Beispiel:** „`lockDuration: 30_000` ist die Sperre in Millisekunden — nicht `30`.“

### `bpmnError` / `failure`

Zwei Fehlerkanäle mit unterschiedlicher Folge. `bpmnError(errorCode)` meldet einen **fachlichen** Fehler; die Engine sucht ein passendes Boundary Error Event. `failure(message, details?, retries?)` meldet einen **technischen** Fehler; die Engine dekrementiert `retries`, bei `0` entsteht ein Incident.

- **Synonyme:** keines. Nicht „Business Error“ als Fachbegriff.
- **Abzugrenzen von:** einander; `retry`/`resolve` (Incident-Behandlung, Admin-only).
- **Beispiel:** `await service.bpmnError("INVALID_ADDRESS");` / `await service.failure(err.message, String(err.stack ?? ""), 0);`

> **Beleglage:** Ein ungefangener `bpmnError` erzeugt einen Incident der Art **`service_task`** mit der Meldung `Unhandled BPMN error '<CODE>'` (Live-Lauf 2026-10-24). Die frühere Angabe `bpmn_error` ist falsch; der Enum-Wert existiert, sein Auslöser ist nicht belegt.

### Idempotenz / At-least-once

Die Engine garantiert **At-least-once**: Jeder External Task wird mindestens einmal bearbeitet, nach Ablauf der Sperre kann er ein zweites Mal geholt werden. **Exactly-once** gibt es nicht. **Idempotenz** ist die Pflicht des Worker-Codes, daraus keinen doppelten Effekt werden zu lassen.

- **Synonyme:** keines. Schreibweise immer „At-least-once“ und „Exactly-once“, mit Bindestrichen.
- **Abzugrenzen von:** Retry (erneuter Versuch), Lock (Sperre).
- **Beispiel:** „`sendMail` muss so gebaut sein, dass ein zweiter Aufruf keinen doppelten Effekt hat.“

---

## 3. Fehlerzustände und Reparatur

### Incident

Ein **persistierter Fehlerzustand an einer Process Instance**, der manuelles Eingreifen erfordert. Arten: `service_task`, `timer`, `persistence`, `bpmn_error`. Maximal 100 pro Instanz.

- **Synonyme:** keines. Nicht „Fehler“ (zu unspezifisch), nicht „Exception“.
- **Abzugrenzen von:** HTTP-Fehler (Transportebene), `bpmnError` (ein Aufruf, nicht der Zustand).
- **Beispiel:** `{"kind":"service_task","node_id":"send-email","message":"SMTP nicht erreichbar","resolved_at":null}`

> **Beleglage zu `bpmn_error`:** Der Enum-Wert ist belegt (`engine-server/interfaces.md` §Incident-API), der auslösende Fall nicht. Belegt ist nur der Service-Task-Fall.

### Boundary Error Event

Ein Boundary Event am Service Task, das einen `bpmnError` **fängt** und die Ausführung auf den alternativen Pfad setzt. Der Abgleich erfolgt über den aufgelösten `errorCode`.

- **Synonyme:** keines.
- **Abzugrenzen von:** Error End Event (wirft, fängt nicht), Incident (entsteht nur, wenn nichts fängt).
- **Beispiel:** `<bpmn:errorEventDefinition errorRef="err_invalid_address" />` in einem `<bpmn:boundaryEvent attachedToRef="send-email">`

> **Catch-all-Falle:** Ohne auflösbaren `errorRef` erhält das Ereignis `error_code = None` und fängt **jeden** Code an dieser Aktivität — still, ohne Deploy-Fehler.

### Error End Event

Ein End Event, das einen BPMN-Fehler **wirft**. Belegt ist die Code-Auflösung über `errorRef`; ohne Treffer gilt `generic_error`.

- **Synonyme:** keines.
- **Abzugrenzen von:** Boundary Error Event, `bpmnError` (der Worker-Aufruf).
- **Beispiel:** `<endEvent><errorEventDefinition errorRef="err_invalid_address" /></endEvent>`

> **Offen:** Ob ein **ungefangenes** Error End Event einen Incident `bpmn_error` erzeugt, ist nicht belegt. Die Aussage steht deshalb nicht im Vertrag von `reference/bpmn-elemente.md`, sondern unter **Offen**.

### `OrphanedToken`

Ein Token, das bei einer Migration im Zielmodell keinen Knoten findet. Die Migration wird mit **422** abgelehnt und ändert nichts.

- **Synonyme:** keines.
- **Abzugrenzen von:** Incident (persistierter Zustand; `OrphanedToken` ist ein Abbruchgrund der Operation).
- **Beispiel:** „Ein Token ohne Zielknoten macht die Migration unmöglich — atomar, ohne Teiländerung.“

### Migration / Modification

**Migration** verschiebt laufende Instanzen auf eine andere Definition-Version. **Modification** ändert innerhalb einer Instanz (Variablen, Token-Position), ohne die Version zu wechseln.

- **Synonyme:** keines.
- **Abzugrenzen von:** einander; Deploy (legt eine neue Version an).
- **Beispiel:** „`/migrate` ist eine Migration, `/move-token` eine Modification.“

### Flattening / JoinBarrier

**Flattening** löst Embedded Sub-Prozesse zur Parse-Zeit in den Hauptgraph auf (ADR-003). Die **JoinBarrier** ist die Laufzeitstruktur, mit der ein Join über `expected_count` weiß, auf wie viele Tokens er wartet.

- **Synonyme:** keines. Nicht „Inlining“, nicht „Gateway“.
- **Abzugrenzen von:** Call Activity (bleibt eigener Aufruf), Gateway (Modellknoten).
- **Beispiel:** „Sub-Prozesse werden geflattet; ein OR-Join setzt `expected_count` auf die Zahl der genommenen Zweige.“

### Rhai

Die **Skript-Engine** von BPMNinja (ADR-008). Kein JavaScript, kein Groovy. Limits: 50 000 Operationen, 2 MiB Speicher, 1 000 ms Laufzeit.

- **Synonyme:** keines. Nicht „JavaScript-Engine“.
- **Abzugrenzen von:** External Task (externer Code), Listener-Skript.
- **Beispiel:** „Skripte laufen in Rhai, nicht in JavaScript.“

---

## 4. HTTP-Vertrag

### Fehlerformat

Jede Fehlerantwort hat die Form `{ "error": "Human-readable error message" }`. Kein `code`-Feld, kein verschachteltes Objekt.

- **Synonyme:** keines.
- **Abzugrenzen von:** Erfolgsantwort (frei strukturiert).
- **Beispiel:** `{"error":"InvalidDefinition: ..."}`

### SSE-Ereignis / Catch-up

`GET /api/events` sendet Server-Sent Events (`instance_changed`, `task_changed`, `definition_changed`), grobkörnig und über einen Broadcast-Kanal mit 256 Plätzen. Ein langsamer Consumer verliert Ereignisse; **Catch-up gibt es nicht**.

- **Synonyme:** keines. Nicht „WebSocket“, nicht „Replay“.
- **Abzugrenzen von:** REST-Polling, Long-Polling.
- **Beispiel:** „Es gibt keinen Catch-up. Das Protokoll ist: Ereignis empfangen → per REST nachladen.“

### Front-Matter-Felder

Die neun Pflichtfelder jeder Seite: `title`, `description`, `audience`, `prerequisites`, `content_type`, `status`, `last_reviewed`, `owner`, `related`. Reihenfolge und Wertebereiche regelt Style Guide R5.1–R5.7.

- **Synonyme:** keines.
- **Abzugrenzen von:** HTTP-Header (Transportebene).
- **Beispiel:** `content_type: how-to` / `status: draft` / `last_reviewed: 2026-10-24`

---

## 5. Konfiguration und Betrieb

### fail-closed

Ein Gate, das im Zweifel **abschließt** statt durchzulassen: ohne Key und mit Nicht-Loopback-Bind antworten alle `/api`-Routen mit **401**; ohne NATS startet die Engine nicht.

- **Synonyme:** keines. Schreibweise „fail-closed“, kleingeschrieben, mit Bindestrich.
- **Abzugrenzen von:** fail-open, Fehler (Zustand, nicht Verhalten).
- **Beispiel:** „Das Gate ist fail-closed: **401** auf allen `/api`-Routen außer `/api/health` und `/api/ready`.“

### Long-Polling / Backoff

**Long-Polling:** Der Server hält eine `fetchAndLock`-Anfrage offen, bis ein Task eintrifft oder `asyncResponseTimeout` abläuft. **Backoff:** wachsende Wartezeit zwischen Wiederholungen — die Persistenz-Retry-Queue nutzt exponentiellen Backoff bis 60 s, maximal 50 Versuche.

- **Synonyme:** keines. Schreibweise „Long-Polling“, „Backoff“ (ein Wort).
- **Abzugrenzen von:** Sperre (`lockDuration`), Retry (der Versuch selbst).
- **Beispiel:** „Long-Polling hält die Verbindung; die Sperre hält den Task.“

### `REQUIRE_NATS` / `BIND_ADDR`

`REQUIRE_NATS` (Default `true`) erzwingt NATS beim Start; `false` startet den In-Memory-Modus ohne Persistenz, `/api/ready` bleibt `503`. `BIND_ADDR` (Default `0.0.0.0`) setzt den Listen-Host; Loopback sind `127.0.0.1`, `::1` und `localhost`.

- **Synonyme:** `BIND_ADDRESS` (Alias, zulässig).
- **Abzugrenzen von:** `NATS_URL` (Adresse), `PORT` (Port).
- **Beispiel:** „`REQUIRE_NATS=false` ist ausschließlich für ephemere lokale Läufe gedacht.“

### KV-Bucket / Object Store / Stream

Die drei Rollen von NATS JetStream in BPMNinja: **KV-Bucket** für Entitäten (JSON je Key), **Object Store** für Dateien (`bpmn_xml`, `instance_files`), **Stream** für Ereignis- und Historienfolgen (`WORKFLOW_EVENTS`, `WORKFLOW_HISTORY`, `ENGINE_LOGS`).

- **Synonyme:** keines. Nicht „Datenbank“, nicht „Tabelle“.
- **Abzugrenzen von:** einander — ein Bucket ist kein Stream, ein Object Store ist kein Bucket.
- **Beispiel:** „`bpmn_xml` ist ein Object Store, `messages` ein KV-Bucket, `WORKFLOW_EVENTS` ein Stream.“

### Retention

Die Aufbewahrungsgrenze eines Streams (`max_age`, `max_bytes`, `num_replicas`). Sie wird bei **jedem** Start abgeglichen; der erste Start mit niedrigeren Limits verwirft sofort ältere Nachrichten.

- **Synonyme:** keines. Nicht „Backup“ — Retention löscht, sie sichert nicht.
- **Abzugrenzen von:** Backup, Archivierung (`history_instances`).
- **Beispiel:** „Setze die Retention-Variablen **vor** dem ersten Start, sonst schneidet der Abgleich die Historie ab.“

### Readiness / Health

`GET /api/ready` liefert **200**, wenn die Engine Zustand sicher halten kann, sonst **503**. `GET /api/health` liefert **200**, solange der Prozess lebt, und sagt nichts über Persistenz.

- **Synonyme:** keines.
- **Abzugrenzen von:** einander; ein grüner Health-Check ist kein Beweis für Persistenz.
- **Beispiel:** „`200` auf `/api/health` und `503` auf `/api/ready` heißt: Der Prozess lebt, die Persistenz nicht.“

---

## 6. Nicht verwendete Begriffe (Sperrliste)

Diese Begriffe sind in der Dokumentation **nicht** zu verwenden. Linke Spalte: verboten. Rechte Spalte: stattdessen.

| Verboten | Stattdessen |
|---|---|
| „Workflow Engine“ (als Produktname) | „BPMNinja“ oder „Engine“ |
| „Business Error“ | `bpmnError` / „fachlicher Fehler“ |
| „Exception“ (für Incident) | „Incident“ |
| „Timeout“ (für die Sperre) | `lockDuration` / „Sperre“ |
| „Queue-Name“ (für Topic) | „Topic“ |
| „Prozess-ID“ | `instance_id` oder `definition_key` (je nach Bezug) |
| „hier klicken“ | beschreibender Linktext |
| „at least once“, „exactly once“ | „At-least-once“, „Exactly-once“ |
| „einfach“, „nahtlos“, „leistungsstark“, „revolutionär“ | konkrete Aussage mit Beleg |
| „offensichtlich“, „selbstverständlich“ | Begründung oder Beleg |
| „wir“ (für das Produkt) | „die Engine“ / Passiv |
| „JavaScript-Engine“ (für Skripte) | „Rhai“ |
| „WebSocket“ (für `/api/events`) | „SSE“ / „Server-Sent Events“ |

---

## 7. Zielgruppen (geschlossene Werteliste)

`audience` enthält ausschließlich diese Werte, in dieser Schreibweise:

| Wert | Kernfrage |
|---|---|
| Einsteiger und Lernende | „Wie bekomme ich einen Prozess von XML bis zum Abschluss?“ |
| Prozessmodellierer | „Wird mein Element unterstützt, und verhält es sich wie in Camunda?“ |
| Entwickler und Integratoren | „Wie baue ich einen Worker, der Tasks korrekt abschließt und Fehler meldet?“ |
| Betriebsteams und Plattform-Administratoren | „Welche Keys, Grenzwerte und Prüfungen brauche ich für den Betrieb?“ |
| Stakeholder und Entscheider | „Was kann die Engine bewusst nicht, und was kostet der Betrieb?“ |
| Beitragende und Maintainer | „Wie ist der Code aufgebaut, und wo ändere ich was?“ |

---

## Offen

- **Sprungziele ohne Anker:** Die Übersicht nennt Abschnitte als Text. Grund: Style Guide R6.7 verbietet Anker auf Überschriften mit Umlauten, solange die Ankerauflösung nicht geprüft ist. **Offen:** Nach der Prüfung werden die Sprungziele wieder verlinkt.
- **`reference/glossar.md`:** Das Glossar liegt unter `_review/glossary.md`, nicht in der Referenzschicht. **Offen:** Entscheidung über den endgültigen Ort (siehe `reference/README.md`).
- **Bucket-Namen:** `messages` und `history_instances` folgen `persistence-nats/src/client.rs`; `persistence/gotchas.md` nennt `message_catches`, `history` und `history_completed_instances`. **Offen:** quellseitig bestätigen (technischer Review, B-02). Das Glossar führt deshalb keine eigenen Bucket-Namen.
- **`bpmn_error`:** Enum-Wert belegt, Auslöser nicht. **Offen:** mit einem Live-Lauf klären.
- **Client-Paketname:** Repository-Pfad `bpmn-ninja-external-task-client` ist belegt; ein Registry-Paket `@bpmninja/external-task-client` ist nicht belegt. **Offen:** Registry-Nachweis oder dauerhaft Repository-Build dokumentieren.
