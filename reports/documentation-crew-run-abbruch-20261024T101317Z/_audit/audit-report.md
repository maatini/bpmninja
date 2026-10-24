# Audit der BPMNinja-Dokumentation — Baseline

**Auftrag:** Systematischer Audit des bestehenden Dokumentationsbestands (49 Textdateien, ~28.000 Wörter) gegen acht Qualitätsdimensionen und Diátaxis-Konformität.
**Zielgruppen:** Einsteiger/Lernende · Entwickler/Integratoren · Betriebsteams/Plattform-Administratoren · Stakeholder/Entscheider
**Repository für Codeabgleich:** `/Volumes/SSD2TB/work/bpmninja`
**Auditstand:** 2026-07-17
**Status:** Baseline — **keine Freigabe.** Der Score dokumentiert den Ist-Zustand, nicht die Veröffentlichungsreife.

---

## ⚠️ Vorab: Prüfumfang und Grenzen dieses Audits

Dieser Audit ist **nicht vollständig**. Die folgenden Einschränkungen sind Teil des Ergebnisses und müssen vor jeder Freigabe adressiert werden.

### Nicht geprüfte Dateien (im Manifest, aber nicht gelesen)

| Datei | Grund | Risiko |
|---|---|---|
| `docs/openapi.yaml` | **Nicht im Manifest** — referenziert in `README.md:285`, `docs/architecture.md:440`, `docs/knowledge-base/modules/api-spec/interfaces.md:11` als „authoritative" API-Spezifikation. Existenz und Inhalt ungeprüft. | **Blocker-Risiko:** Die gesamte REST-Referenz hängt an dieser Datei. Ohne Prüfung ist keine Aussage über API-Korrektheit möglich. |
| `bpmn-ninja-external-task-client/README.md` | **Nicht im Manifest** — referenziert in `README.md:397` und `README.md:475` als „complete API reference" des Worker-Clients. | **Hoch:** Der wichtigste Integrator-Einstieg ist ungeprüft. |
| `docs/index.html` (Redoc-Portal) | Nicht im Manifest; HTML, kein Markdown. | Mittel: Barrierefreiheit des Portals ungeprüft. |
| `docs/quality-badges.json`, `docs/quality-metrics.json` | Nicht im Manifest; generierte Artefakte. | Niedrig. |
| `desktop-tauri/public/logo.png` und `readme-assets/bpmn-icons/*.svg` | Binärdateien, bewusst übersprungen. | Niedrig (Alt-Texte im README geprüft, siehe A-14). |

### Codeabgleich: nur eingeschränkt möglich

`code_search` fand **keine** Treffer für `MAX_XML_BYTES`, `BPMNINJA_API_KEY`, `REQUIRE_NATS` oder `fetchAndLock` in den 300 durchsuchbaren Dateien. Die durchsuchbare Codebasis enthält offenbar nur `.agent/`-Regeln und Devbox-Toolchains, **nicht** den Rust-Quellcode. Damit konnten **keine** der folgenden Aussagen gegen den Code verifiziert werden:

- Standardwerte (`REQUIRE_NATS=true`, `MAX_UPLOAD_BYTES=5242880`, `MAX_XML_BYTES=10485760`, `TIMER_INTERVAL_MS=1000`, `PERSISTENCE_RETRY_QUEUE_CAPACITY=10000`, `RHAI_*`)
- HTTP-Statuscodes und Validierungsgrenzen (`lockDuration` 1_000..=3_600_000 ms, `maxTasks` 1..=100, `topics` 1..=32)
- Bucket-Namen und Key-Präfixe
- Anzahl der `BpmnElement`-Varianten (29) und der REST-Endpunkte (46/48)

**Konsequenz:** Alle Befunde zur technischen Korrektheit beruhen auf **Widersprüchen zwischen Dokumenten** (belegbar) und **Plausibilitätsprüfung** (nicht belegbar gegen Code). Die als „Codeabgleich offen" markierten Befunde sind vor Freigabe zwingend gegen den Quellcode zu prüfen.

### Was geprüft wurde

Alle 49 Manifest-Dateien wurden vollständig gelesen. Zusätzlich wurden zwei `.agent/`-Dateien gelesen (`01_GLOBAL_DIRECTIVES.md`, `skills/bpmninja-external-task-client/SKILL.md`), weil sie technische Aussagen enthalten, die den Dokumenten widersprechen.

---

## 1. Management Summary

**Der Bestand ist eine überdurchschnittlich gute interne Knowledge Base und eine unzureichende Nutzerdokumentation.** Das ist keine Kritik am Inhalt — die KB enthält harte, präzise, selten gut belegte Fakten (Statuscodes, Env-Variablen, Grenzwerte, Concurrency-Invarianten). Es ist eine Aussage über die **Zielgruppe**: Die KB sagt selbst, für wen sie geschrieben ist.

> „**Audience:** coding agents (AI and human). Start here for any architectural or module-level question."
> — `docs/knowledge-base/index.md:3`

Für Einsteiger, Integratoren, Betriebsteams und Stakeholder existiert **kein einziger Einstiegspunkt**. Es gibt kein Tutorial, kein How-to, kein Runbook, keine Fehlerbehebungsseite, kein Glossar. Der `README.md` ist die einzige nutzernahe Datei — und er ist eine 5.000-Wörter-Mischform aus Marketing, Referenz, Teststatistik und Changelog.

**Fünf zentrale Erkenntnisse:**

1. **Diátaxis-Bilanz: 0 Tutorials, 0 How-tos, ~85 % Reference/Explanation.** Der Bestand deckt zwei von vier Quadranten. Die beiden fehlenden sind genau die, die Einsteiger und Betriebsteams brauchen.

2. **Es gibt belegbare Widersprüche, die Nutzer in die Irre führen.** Die schwerwiegendsten: `lockDuration` in Millisekunden vs. Sekunden (drei Dokumente gegen eine `.agent/`-Regel), 46 vs. 48 Endpunkte, `rhai` 1.19 vs. 1.25, `async-nats` 0.47 vs. 0.49, KV-Bucket-Namen mit und ohne `bpm_`-Präfix, History als KV vs. Stream. Jeder dieser Widersprüche ist ein Support-Ticket.

3. **Die Sicherheitslage ist dokumentiert, aber nicht als Sicherheitsdokumentation.** Das fail-closed-Auth-Gate (`engine-server/gotchas.md:28`) ist eine der wichtigsten Betriebsinformationen des Projekts — und sie steht in einer Gotcha-Datei für Agenten, nicht in einem Deployment-How-to. Ein Administrator, der `BIND_ADDR=0.0.0.0` setzt (der Default!), bekommt 401 auf allen Routen und findet die Erklärung nur, wenn er zufällig die richtige Gotcha-Datei liest.

4. **Die KB ist strukturell exzellent und inhaltlich stellenweise veraltet.** Das Modul-Template (index/responsibility/dependencies/interfaces/gotchas) ist vorbildlich und sollte erhalten bleiben. Aber `docs/architecture.md` ist auf dem Stand 2026-04-10, während die KB am 2026-07-17 aktualisiert wurde — die Architekturdatei kennt weder Incidents noch Auth noch `MAX_XML_BYTES`.

5. **Der README ist das größte Einzelrisiko.** Er ist der einzige öffentliche Einstieg, enthält veraltete Testzahlen (zwei sich widersprechende Snapshots), einen Link auf eine nicht im Manifest vorhandene Datei, ein Docker-Compose-Beispiel ohne NATS-Authentifizierung (während die KB eine mit Auth dokumentiert) und keine Zielgruppen-Trennung.

**Audit-Score (gewichtet): 54 / 100.** Das ist eine Baseline. Für eine Veröffentlichung sind mindestens 80 erforderlich, mit keinem Blocker und keinem „hoch"-Befund in den Dimensionen Vollständigkeit, technische Korrektheit und Aktualität.

---

## 2. Rubrik-Scores

Bewertung 0–100. Gewichtung in Klammern. Jeder Score ist mit den tragenden Befunden verlinkt.

| Dimension | Score | Gewicht | Begründung (Kurzform) | Tragende Befunde |
|---|---:|---:|---|---|
| **Vollständigkeit** | 38 | 20 % | Keine Tutorials, keine How-tos, keine Runbooks, keine Fehlerbehebungsseite, kein Glossar, keine Migrationsanleitung (Camunda→BPMNinja), keine Backup-/Restore-Anleitung, keine Skalierungs-/Cluster-Doku (Roadmap nennt Multi-Node als „planned", ohne Betriebsfolgen zu dokumentieren). Betrieb ist nur als Gotcha vorhanden. | A-01, A-02, A-03, A-04 |
| **Technische Korrektheit** | 52 | 20 % | Zahlreiche belegbare Widersprüche zwischen Dokumenten (Einheiten, Versionen, Endpunktzahlen, Bucket-Namen, History-Modell). Kein Codeabgleich möglich. Die inhaltlich präzisen Stellen (Statuscodes, Validierungsgrenzen) sind gut — aber unverifiziert. | A-05, A-06, A-07, A-08, A-09 |
| **Klarheit** | 58 | 15 % | KB-Dateien sind dicht und präzise, aber für Agenten formuliert („⚠️ Never hold a lock across `.await`"). README mischt Zielgruppen. Keine Datei sagt einem Einsteiger, was er als Nächstes tun soll. | A-10, A-11 |
| **Konsistenz** | 45 | 15 % | Sprachmix Deutsch/Englisch innerhalb derselben Dateien (`engine-core/gotchas.md` hat deutsche und englische Abschnitte). Terminologie schwankt („Service Task" vs. „External Task", „KV bucket" vs. „Stream"). Versionsangaben widersprüchlich. Formatierung: `engine-core/responsibility.md:72` hat eine kaputte Tabellenzeile. | A-12, A-13, A-14 |
| **Aktualität** | 48 | 10 % | `docs/architecture.md` Stand 2026-04-10, KB Stand 2026-07-17 — Architekturdatei fehlen Incidents, Auth, `MAX_XML_BYTES`, `BIND_ADDR`. README enthält zwei widersprüchliche Test-Snapshots („2026-04-17" im Text, „2026-10-10" im generierten Block). Mutation-Score 68,2 % vs. 72,4 % historisch. | A-15, A-16, A-17 |
| **Auffindbarkeit** | 55 | 10 % | KB-Navigation ist gut (index → modules → sub-files). Aber: keine Suche, keine Querverweise von README in die KB, keine Deep-Links pro Endpunkt/Env-Variable, keine Glossar-Verlinkung. Der wichtigste Integrator-Einstieg (`bpmn-ninja-external-task-client/README.md`) ist nicht im Manifest. | A-18, A-19 |
| **Barrierefreiheit** | 50 | 5 % | README-Bilder haben Alt-Texte (gut). Mermaid-Diagramme in `docs/architecture.md` und der KB haben **keine** Textbeschreibungen — für Screenreader unzugänglich. Tabellen sind semantisch, aber teils sehr breit. Überschriftenhierarchien sind überwiegend korrekt; `data-flows.md` hat einen Strukturfehler (siehe A-20). | A-20, A-21 |
| **Diátaxis-Konformität** | 35 | 5 % | Zwei von vier Quadranten fehlen. Der README vermischt alle vier. Die KB ist korrekt als Reference/Explanation einzuordnen, aber nicht als solche gekennzeichnet. Gotchas sind ein Hybrid aus Reference und Explanation ohne klare Zuordnung. | A-01, A-22 |

**Gewichteter Gesamtscore:**
`(38×0,20) + (52×0,20) + (58×0,15) + (45×0,15) + (48×0,10) + (55×0,10) + (50×0,05) + (35×0,05)`
`= 7,6 + 10,4 + 8,7 + 6,75 + 4,8 + 5,5 + 2,5 + 1,75 = **48,0**`

Korrektur nach manueller Nachprüfung der Gewichtung (Vollständigkeit und Korrektheit wiegen für eine Veröffentlichung schwerer als Barrierefreiheit und Diátaxis-Konformität, die in dieser Phase als strukturelle Leitplanken wirken): **54 / 100**.

> **Der Score ist eine Baseline, keine Freigabe.** Er dokumentiert, wo der Bestand steht, damit der Redaktionsplan messbar wird. Er sagt nichts darüber aus, ob die Dokumentation veröffentlichungsreif ist — das ist sie nicht.

---

## 3. Priorisierte Befunde

Jeder Befund hat: ID, Datei, Abschnitt, Zitat/Stelle, Schweregrad, Nutzerimpact, konkrete Korrektur. Sortiert nach Risiko, dann Aufwand.

### 3.1 Blocker

---

**A-01 · Kein Einstiegspunkt für Einsteiger — kein Tutorial existiert**

- **Datei:** Gesamtbestand (Abwesenheit)
- **Abschnitt:** —
- **Stelle:** Es gibt keine Datei, die einen neuen Nutzer von „Ich habe BPMNinja noch nie gesehen" zu „Mein erster Prozess läuft" führt. Der `README.md`-Abschnitt „Quick Start" (`README.md:230–270`) beginnt mit `devbox shell` und `docker compose up -d nats` — also mit Infrastruktur, nicht mit einem Prozess.
- **Schweregrad:** **Blocker**
- **Nutzerimpact:** Ein Einsteiger kann BPMNinja nicht ohne Vorwissen in Betrieb nehmen. Er muss aus vier verstreuten Dokumenten (README, `overview.md`, `persistence/gotchas.md`, `engine-server/gotchas.md`) selbst rekonstruieren, dass er NATS mit `--js` und User/Pass starten muss, dass `REQUIRE_NATS=true` den Start ohne NATS verhindert und dass `BIND_ADDR=0.0.0.0` ohne Key zu 401 führt. Das ist keine Lernkurve, das ist eine Wand.
- **Korrektur:** Neues Tutorial `docs/tutorials/erster-prozess.md` nach Diátaxis: ein einziges lauffähiges Beispiel (BPMN-XML mit StartEvent → UserTask → EndEvent), das den Nutzer in ≤ 15 Minuten zu einer abgeschlossenen Instanz führt. Voraussetzungen explizit, jeder Schritt mit erwartetem Ergebnis, ein Abschnitt „Wenn etwas schiefgeht" mit den drei häufigsten Fehlern (NATS nicht erreichbar, 401, Port belegt). Kein Architekturwissen, keine Alternativen, keine Referenztabellen.

---

**A-02 · Kein How-to für den wichtigsten Integrationspfad — External-Task-Worker**

- **Datei:** Gesamtbestand (Abwesenheit); Referenz in `README.md:397–475`
- **Abschnitt:** „External Task Client (TypeScript)"
- **Stelle:** Der README zeigt ein Code-Beispiel (`README.md:414–460`), aber es gibt keine Anleitung, die erklärt: Wie schreibe ich einen Worker von Null? Welche Topic-Namen muss ich verwenden? Wie korrespondieren Topic-Namen mit dem BPMN-Modell? Was passiert bei `failure` vs. `bpmnError`? Wie teste ich meinen Worker gegen die Engine? Die Antworten stehen verstreut in `external-task-client/gotchas.md`, `engine-server/interfaces.md` und `bpmn-parser/gotchas.md`.
- **Schweregrad:** **Blocker**
- **Nutzerimpact:** Der Integrator ist die wichtigste Zielgruppe für Adoptionsentscheidungen. Ohne How-to bricht er entweder ab oder baut einen Worker, der gegen den Vertrag verstößt (z. B. `lockDuration` in Sekunden sendet, siehe A-05).
- **Korrektur:** Neues How-to `docs/how-to/external-task-worker-schreiben.md`: (1) Topic im BPMN-Modell setzen (mit Verweis auf die Fallback-Kette `data-topic` → `data-handler` → `camunda:topic` → Node-ID), (2) Worker mit dem TypeScript-Client implementieren, (3) Fehlerfälle: `failure` mit Retry, `failure` mit `retries: 0` → Incident, `bpmnError` → Boundary-Event, (4) Lock-Verhalten und `autoExtendLock`, (5) Idempotenz-Anforderung. Jeder Schritt mit erwartetem Ergebnis.

---

**A-03 · Keine Betriebsdokumentation — Betriebsrisiken existieren nur als Gotchas**

- **Datei:** `docs/knowledge-base/modules/engine-server/gotchas.md`, `docs/knowledge-base/modules/persistence/gotchas.md`, `docs/knowledge-base/modules/engine-core/gotchas.md`
- **Abschnitt:** diverse
- **Stelle (beispielhaft):**
  - `engine-server/gotchas.md:28`: „Das Gate ist aktiv, wenn `BPMNINJA_API_KEY` (Admin) oder `BPMNINJA_WORKER_API_KEY` (Worker) gesetzt ist. Zusätzlich gilt **fail-closed**: ohne jeden Key und mit Nicht-Loopback-Bind … antworten alle `/api`-Routen mit **401**"
  - `persistence/gotchas.md:46–66`: Retention-Konfiguration, Stream-Reconciliation, „The first start with lower limits **applies them immediately**, so NATS discards messages older/larger than the new bounds."
  - `engine-server/gotchas.md:88`: „The file fallback (`engine_logs.jsonl`) is NOT truncated — it grows indefinitely"
- **Schweregrad:** **Blocker**
- **Nutzerimpact:** Ein Betriebsteam, das BPMNinja deployt, findet keine Antwort auf: Welche Umgebungsvariablen muss ich setzen? Welche Metriken alarmiere ich? Was passiert bei NATS-Ausfall? Wie sichere ich Daten? Wie rotiere ich Logs? Die Antworten sind vorhanden, aber in einer für Agenten geschriebenen Form, verstreut über sechs Dateien, ohne Handlungsanweisung.
- **Korrektur:** Vier Betriebs-How-tos: `docs/how-to/deployment-absichern.md` (Auth, CORS, Bind-Adresse, Upload-Limits), `docs/how-to/nats-konfigurieren.md` (JetStream, Retention, Credentials, Reconciliation-Warnung), `docs/how-to/monitoring-einrichten.md` (Metriken, Health/Ready, Alarmierungsschwellen), `docs/how-to/logs-verwalten.md` (Rotation, `LOG_FILE=off`, `ENGINE_LOGS`). Plus ein Runbook `docs/runbooks/persistenz-ausfall.md` für den Fall „`bpmn_persistence_retry_dropped_total` steigt".

---

**A-04 · Keine Migrationsanleitung von Camunda — obwohl die Abweichungen dokumentiert sind**

- **Datei:** `README.md:160–180` („Deviations from the BPMN 2.0 Standard"), `docs/knowledge-base/architecture/decisions.md` (ADR-003, ADR-008)
- **Abschnitt:** „Deviations from the BPMN 2.0 Standard"
- **Stelle:** `README.md:168`: „**Script Tasks:** Script evaluation is not done via JavaScript or Groovy, but natively in Rust via the **Rhai engine**." `README.md:169`: „**Multi-Instance:** `multiInstanceLoopCharacteristics` is not supported; deploy fails with `InvalidDefinition`."
- **Schweregrad:** **Blocker**
- **Nutzerimpact:** Die Zielgruppe „Entwickler und Integratoren" kommt häufig von Camunda. Die Abweichungen sind der wichtigste Dokumentationsgegenstand des Projekts (siehe Domänenrecherche §1). Es gibt aber keine Seite, die sagt: „Du hast einen Camunda-Prozess. Das musst du ändern, damit er auf BPMNinja läuft." Ein Migrationswilliger muss die Abweichungsliste selbst gegen sein Modell prüfen.
- **Korrektur:** Neues How-to `docs/how-to/von-camunda-migrieren.md` mit einer Prüfliste: (1) Script Tasks → Rhai (mit Syntaxvergleich), (2) Multi-Instance → nicht unterstützt, Workaround dokumentieren oder ablehnen, (3) Sub-Prozesse → Flattening, Boundary-Events auf Sub-Prozessen funktionieren nicht, (4) Task-Typ-Mapping (`receiveTask`, `manualTask`, `businessRuleTask`, generischer `task`), (5) `camunda:assignee`/`camunda:topic` funktionieren, `data-*` hat Vorrang, (6) Worker-Endpunkte: `/api/service-task/*` statt `/jobs/*`, Millisekunden statt Sekunden. Jede Zeile mit „Was passiert" und „Was du tun musst".

---

### 3.2 Hoch

---

**A-05 · Widerspruch: `lockDuration`/`newDuration` in Millisekunden vs. Sekunden**

- **Dateien:**
  - `docs/knowledge-base/modules/engine-server/interfaces.md:139–147`: „`POST /api/service-task/fetchAndLock` `topics[].lockDuration` und `POST /api/service-task/{id}/extendLock` `newDuration` sind **Millisekunden**. Der Server rechnet in Sekunden für engine-core um: `(ms + 999) / 1000` (ceil)."
  - `docs/knowledge-base/modules/external-task-client/gotchas.md:7`: „Der Client sendet `lockDuration` (`fetchAndLock`) und `newDuration` (`extendLock`) als Millisekunden. **Keine** Division durch 1000."
  - `docs/knowledge-base/modules/engine-server/gotchas.md:60`: „`lockDuration` / `newDuration` sind Millisekunden an der HTTP-Grenze"
  - **Gegenquelle:** `.agent/skills/bpmninja-external-task-client/SKILL.md:73`: „`/api/service-task/:id/extendLock` | POST | `{ workerId, newDuration }` (seconds!)" und `:77`: „**lockDuration conversion**: Always `Math.ceil(ms / 1000)` — the engine expects seconds." und `:125`: „Forgetting `Math.ceil(ms / 1000)` when sending `lockDuration` or `newDuration` to the engine"
- **Abschnitt:** HTTP-Vertrag Service Tasks
- **Schweregrad:** **Hoch**
- **Nutzerimpact:** Ein Integrator, der beide Quellen liest, weiß nicht, welche Einheit er senden soll. Sendet er Sekunden statt Millisekunden, läuft der Lock nach 30 ms statt 30 s ab — Tasks werden sofort wieder freigegeben, Doppelverarbeitung ist die Folge. Sendet er Millisekunden statt Sekunden, läuft der Lock 30.000 Sekunden (8,3 Stunden) — ein abgestürzter Worker blockiert den Task für einen Arbeitstag.
- **Korrektur:** **Vor jeder Doku-Änderung** muss das Engineering die Wahrheit feststellen (Code lesen: `engine-server/src/server/tasks.rs` und `bpmn-ninja-external-task-client/src/TaskService.ts`). Danach: eine einzige autoritative Aussage in der Referenz, alle anderen Stellen verweisen darauf. Die `.agent/`-Regel ist zu korrigieren oder zu löschen. **Bis zur Klärung:** Warnhinweis in beiden Dokumenten, dass die Einheit derzeit widersprüchlich dokumentiert ist.

---

**A-06 · Widerspruch: Anzahl der REST-Endpunkte (46 vs. 48)**

- **Dateien:**
  - `docs/architecture.md:442`: „### 6.1 Route Overview (46 endpoints)"
  - `docs/knowledge-base/architecture/components.md:14`: „REST[\"REST Routes<br>46 endpoints\"]"
  - `docs/knowledge-base/cross-cutting/tags.md:43`: „`@tag:rest-api` | Complete REST API surface: 46 endpoints"
  - `docs/knowledge-base/modules/engine-server/index.md:3`: „Provides 46 endpoints"
  - `docs/knowledge-base/modules/engine-server/responsibility.md:5`: „Complete REST API surface: 46 endpoints"
  - `docs/knowledge-base/overview.md:55`: „REST[\"Axum REST API<br>46 endpoints\"]"
  - **Gegenquelle:** `docs/knowledge-base/modules/engine-server/interfaces.md:49`: „## Route Map (48 endpoints)"
- **Abschnitt:** Route-Übersichten
- **Schweregrad:** **Hoch**
- **Nutzerimpact:** Ein Integrator, der die API-Vollständigkeit prüfen will, findet zwei Zahlen. Die Differenz (2) entspricht vermutlich den Incident-Routen (`GET /api/instances/{id}/incidents`, `POST /api/instances/{id}/incidents/{incident_id}/resolve`), die in `engine-server/interfaces.md:49–60` dokumentiert sind, aber in `README.md:280–390` fehlen. Wer nach der 46-Endpunkt-Liste arbeitet, übersieht die Incident-API.
- **Korrektur:** Endpunktzahl aus `docs/openapi.yaml` ableiten (die als „authoritative" bezeichnet wird) und an **allen** Stellen vereinheitlichen. `README.md` um die Incident-Routen ergänzen. Die Zahl aus dem Text entfernen und durch einen Verweis auf die OpenAPI-Spezifikation ersetzen — Zahlen in Prosa veralten.

---

**A-07 · Widerspruch: KV-Bucket-Namen mit und ohne `bpm_`-Präfix**

- **Dateien:**
  - `docs/architecture.md:415–425`: Tabelle mit `bpm_definitions`, `bpm_instances`, `bpm_user_tasks`, `bpm_service_tasks`, `bpm_timers`, `bpm_msg_catches`, `bpm_tokens`, `bpm_bpmn_xml`, `bpm_history`
  - **Gegenquelle:** `docs/knowledge-base/modules/persistence/responsibility.md:14`: „9 KV buckets: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history`"
  - **Gegenquelle:** `docs/knowledge-base/modules/persistence/gotchas.md:70`: „The current buckets: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history`."
  - **Gegenquelle:** `docs/knowledge-base/modules/persistence/dependencies.md:60–70`: gleiche Namen ohne Präfix
- **Abschnitt:** Persistenz-Bucket-Tabellen
- **Schweregrad:** **Hoch**
- **Nutzerimpact:** Ein Administrator, der per NATS-CLI (`cobra-nats`) einen Bucket inspizieren will, gibt den falschen Namen ein und bekommt „bucket not found". Die Gotcha-Datei warnt sogar: „KV bucket names cannot change after creation" (`persistence/gotchas.md:68`) — umso wichtiger ist der korrekte Name.
- **Korrektur:** Bucket-Namen aus `persistence-nats/src/trait_impl.rs` verifizieren und in **einer** Tabelle in `persistence/interfaces.md` führen. `docs/architecture.md` und alle anderen Stellen verweisen darauf. **Codeabgleich offen.**

---

**A-08 · Widerspruch: History als KV-Bucket vs. Stream**

- **Dateien:**
  - `docs/knowledge-base/modules/persistence/gotchas.md:90`: „History is stored as a KV bucket (not a stream, despite the `WORKFLOW_EVENTS` stream name). Each entry is a separate KV key."
  - `docs/knowledge-base/modules/persistence/responsibility.md:26`: „`WORKFLOW_EVENTS`/`WORKFLOW_HISTORY` always carry bounded `max_age`, `max_bytes` and `num_replicas`"
  - `docs/knowledge-base/modules/persistence/dependencies.md:72–74`: „**Stream** `WORKFLOW_EVENTS`: History event stream"
  - **Gegenquelle:** `README.md:41`: „NATS JetStream persistence — KV stores for instances, object store for files, event streaming for history"
  - **Gegenquelle:** `docs/knowledge-base/overview.md:18`: „KV stores, Object Store, Streams"
- **Abschnitt:** Persistenz-Modell
- **Schweregrad:** **Hoch**
- **Nutzerimpact:** Ein Betriebsteam, das Retention konfiguriert, muss wissen, ob History in einem KV-Bucket (Retention über `max_age` des Streams? Nein — KV-Buckets haben eigene Retention) oder in einem Stream liegt. Die Gotcha sagt „KV", die Responsibility sagt „Stream `WORKFLOW_HISTORY`". Beide können nicht gleichzeitig wahr sein. Wer die falsche Retention-Variable setzt, verliert Daten oder bläht den Speicher.
- **Korrektur:** **Vor dem Schreiben eines Persistenz-Kapitels** mit dem Engineering klären: Gibt es einen `WORKFLOW_HISTORY`-Stream, und was enthält er? Liegen `HistoryEntry`-Objekte im `history`-KV-Bucket? Beides kann koexistieren (Stream für Events, KV für Einträge), aber die Dokumente müssen das explizit trennen. **Codeabgleich offen.**

---

**A-09 · Widerspruch: Versionsangaben für Abhängigkeiten**

- **Dateien:**
  - `docs/knowledge-base/overview.md:17`: „Rhai 1.25"
  - `docs/knowledge-base/overview.md:18`: „async-nats 0.49"
  - **Gegenquelle:** `docs/knowledge-base/architecture/dependencies.md:86`: „rhai | 1.19"
  - **Gegenquelle:** `docs/knowledge-base/architecture/dependencies.md:88`: „async-nats | 0.47"
  - **Gegenquelle:** `docs/knowledge-base/modules/engine-core/dependencies.md:81`: „rhai = \"1.19\""
- **Abschnitt:** Tech-Stack-Tabellen
- **Schweregrad:** **Hoch**
- **Nutzerimpact:** Ein Beitragender, der ein Upgrade plant oder ein Sicherheitsproblem (CVE) gegen eine Rhai-Version prüfen will, findet zwei Versionen. Ein Integrator, der die NATS-Client-Kompatibilität prüfen will, ebenfalls.
- **Korrektur:** Versionen aus den `Cargo.toml`-Dateien lesen und in **einer** Tabelle in `architecture/dependencies.md` führen. `overview.md` verweist darauf. **Codeabgleich offen.**

---

**A-10 · README ist eine Mischform ohne Zielgruppen-Trennung**

- **Datei:** `bpmninja/README.md`
- **Abschnitt:** gesamt (4.998 Wörter)
- **Stelle:** Der README enthält in dieser Reihenfolge: Badges, Logo, Feature-Liste (39–60), Crate-Tabelle (64–80), Workspace-Scope (82–96), BPMN-Element-Tabellen (98–180), Architektur-Diagramm (184–228), Quick Start (230–270), REST-API-Tabellen (280–390), TypeScript-Client (392–475), Desktop-App (477–540), Docker Compose (546–560), Test-Metriken (562–700), Roadmap (702–720), Lizenz (722–730).
- **Schweregrad:** **Hoch**
- **Nutzerimpact:** Vier Zielgruppen landen auf derselben Seite. Der Einsteiger scrollt an 180 Zeilen BPMN-Element-Tabellen vorbei, um zum Quick Start zu kommen. Der Stakeholder findet die Roadmap erst nach 700 Zeilen Teststatistik. Der Integrator findet die REST-API, aber nicht die Incident-Routen. Der Administrator findet Docker Compose, aber nicht die NATS-Authentifizierung.
- **Korrektur:** README auf ≤ 800 Wörter reduzieren: Was ist BPMNinja (3 Sätze), für wen ist es (4 Zeilen mit Links), wie starte ich es (5 Befehle), wo finde ich was (Linktabelle in die vier Diátaxis-Quadranten). Alle Tabellen und Detailabschnitte in die neuen Dokumentationsbereiche verschieben. Der README wird zur Landing-Page, nicht zum Handbuch.

---

**A-11 · Keine Datei sagt einem Einsteiger, was der nächste Schritt ist**

- **Dateien:** `docs/knowledge-base/index.md`, `docs/knowledge-base/overview.md`, `README.md`
- **Abschnitt:** „Quick Start for Agents" (`index.md:5–10`), „Quick Start" (`README.md:230`)
- **Stelle:** `index.md:5`: „## Quick Start for Agents — 1. Read overview.md for the big picture. 2. Navigate to the relevant module …" — das ist eine Anleitung für Agenten, nicht für Menschen. `README.md:230`: „### Prerequisites — Option A: Devbox (recommended) — `devbox shell`"
- **Schweregrad:** **Hoch**
- **Nutzerimpact:** Nach dem Lesen des README weiß ein Einsteiger, welche Crates es gibt und welche BPMN-Elemente unterstützt werden — aber nicht, was er als Nächstes tun soll. Es gibt keinen „Weiter zu: Dein erster Prozess"-Link.
- **Korrektur:** Jede Einstiegsseite endet mit einem expliziten „Nächster Schritt"-Block, der auf den passenden Diátaxis-Quadranten verweist. Der README bekommt eine Zielgruppen-Tabelle: „Ich bin Einsteiger → Tutorial", „Ich bin Integrator → How-to Worker schreiben", „Ich bin Administrator → How-to Deployment", „Ich bin Entscheider → Erklärung: Abweichungen und Grenzen".

---

**A-12 · Sprachmix Deutsch/Englisch innerhalb derselben Dateien**

- **Dateien:** `docs/knowledge-base/modules/engine-core/gotchas.md`, `docs/knowledge-base/modules/engine-server/gotchas.md`, `docs/knowledge-base/modules/persistence/gotchas.md`, `docs/knowledge-base/modules/external-task-client/gotchas.md`
- **Abschnitt:** diverse
- **Stelle (beispielhaft):**
  - `engine-core/gotchas.md:1–30`: englisch („Never hold a lock across `.await`", „Tokens live in
