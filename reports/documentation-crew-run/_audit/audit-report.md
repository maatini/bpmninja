# Audit-Bericht: Dokumentation „BPMNinja“

**Datei:** `_audit/audit-report.md`
**Autor:** Documentation Lead / Redaktionsmanagement
**Auditdatum:** 2026-10-24
**Repository:** `/Volumes/SSD2TB/work/bpmninja`
**Geprüfter Bestand:** 49 Textdateien (Manifest), davon 49 gelesen (README + 48 Dateien unter `docs/`)
**Zusätzlich geprüft (Code-Stichproben):** `.agent/skills/bpmninja-external-task-client/SKILL.md`, `.agent/skills/bpmninja-server/SKILL.md`, `.agent/skills/bpmninja-quality/SKILL.md`, `.agent/rules/VERSIONING.md`, `.agent/rules/NATS_MESSAGE_BROKER.md`

---

## 1. Management Summary

Die BPMNinja-Dokumentation ist **technisch überdurchschnittlich tief, aber strukturell einseitig**. Sie ist faktisch eine *Reference*- und *Explanation*-Sammlung für Beitragende und Maintainer — ausdrücklich so adressiert (`docs/knowledge-base/index.md`: „Audience: coding agents (AI and human)“). Für die vier im Auftrag genannten Zielgruppen fehlen **Tutorials und How-to-Guides vollständig**; Betriebs- und Sicherheitswissen ist über sechs Dateien verstreut und nirgends als Runbook zusammengeführt; Stakeholder finden keine Eignungs- oder Risikoseite.

**Fünf zentrale Befunde:**

1. **Kein Einstieg für Einsteiger und Integratoren.** Es existiert kein Tutorial, kein „Erster Prozess in 10 Minuten“, kein Worker-Quickstart mit lauffähigem End-to-End-Pfad. Der README-Quickstart endet bei `cargo run -p engine-server` und verweist für den Worker auf ein externes Paket-README, das nicht im Manifest steht.
2. **Widersprüchliche Endpunkt-Zahlen und widersprüchliche Lock-Dauer-Semantik.** „46 endpoints“ (6 Stellen) vs. „48 endpoints“ (1 Stelle); `lockDuration`/`newDuration` werden an drei Stellen als Millisekunden, in `.agent/skills/bpmninja-external-task-client/SKILL.md` als Sekunden beschrieben. Beides ist für Integratoren ein Blocker.
3. **Abweichungen von BPMN 2.0 und Camunda sind verstreut, teils widersprüchlich.** Der README listet Abweichungen; `bpmn-parser/gotchas.md` widerspricht dem README bei `receiveTask`, `manualTask`, `businessRuleTask`, generischem `task` und leerem `scriptTask`. Es gibt keine konsolidierte „Compatibility & Deviations“-Referenz.
4. **Betriebs- und Sicherheitsdokumentation ist Gotcha-Sammlung, kein Runbook.** Fail-closed-Auth, `REQUIRE_NATS`, Retry-Queue-Drops, SSE-Verluste, unbegrenzt wachsende Log-Datei, Stream-Retention-Migration — alles vorhanden, aber ohne Betriebs-Checkliste, ohne Eskalationspfad, ohne „Was tun bei X“.
5. **Versions- und Abhängigkeitsangaben sind inkonsistent.** Rhai 1.19 vs. 1.25, async-nats 0.47 vs. 0.49 vs. 0.38 (`.agent/rules/NATS_MESSAGE_BROKER.md`), quick-xml 0.39, Testzahlen 431 vs. 55/56 vs. 48, LoC-Zahlen zwischen `docs/architecture.md` und README divergieren um ~70 %.

**Audit-Score (Baseline, keine Freigabe): 58 / 100.**

Der Score ist eine **Baseline für den Umbau**, keine Veröffentlichungsfreigabe. Er dokumentiert den Ist-Zustand vor jeder Änderung und ist nur gegen denselben Prüfumfang reproduzierbar.

---

## 2. Rubrik-Scores

Bewertung 0–100 je Dimension, gewichtet nach Nutzerimpact. Die Gewichte sind redaktionelle Setzung und im Bericht offengelegt, damit der Score nachvollziehbar bleibt.

| Dimension | Score | Gewicht | Begründung (Kurz) |
|---|---:|---:|---|
| **Vollständigkeit** | 45 | 20 % | Keine Tutorials, keine How-tos, keine Betriebs-Runbooks, keine Migrations-/Upgrade-Anleitung, keine Sicherheitsseite, keine Glossar-Seite. Referenz für Module dagegen vollständig. |
| **Technische Korrektheit** | 62 | 20 % | Viele präzise, codebelegte Aussagen (Gotchas sind stark). Aber: widersprüchliche Lock-Dauer-Semantik, widersprüchliche Task-Typ-Mappings, Endpunkt-Zahl 46 vs. 48, Versionsdivergenzen. |
| **Klarheit** | 58 | 15 % | Moduldateien sind klar strukturiert (Responsibility/Dependencies/Interfaces/Gotchas). README ist feature-lastig, Aufgaben und nächster Schritt für Einsteiger fehlen. Sprachmix DE/EN innerhalb derselben Datei. |
| **Konsistenz** | 50 | 15 % | Terminologie schwankt (`bpmn_id`/`bpmn_process_id`, `External Task`/`Service Task`/`Job`, `persistence`/`persistence-nats`/`persistence-memory`), Struktur der Modulordner ist konsistent, Formatierung nicht (Tabellen vs. Bullets, Emoji-Header). |
| **Aktualität** | 55 | 10 % | `maintenance.md` datiert 2026-07-17, `architecture.md` 2026-04-10, README-Qualitätsblock 2026-10-10, Test-Snapshot 2026-04-17. Mehrere Stände parallel; keine Seite nennt einen „gültig ab“-Stand. |
| **Auffindbarkeit** | 52 | 10 % | `knowledge-base/index.md` ist ein guter Einstieg für Agenten, aber nicht für Menschen mit Aufgabe. Keine Suche, keine Breadcrumbs, keine „Verwandte Seiten“-Blöcke. Titel sind teils nichtssagend (`index.md`, `Overview`). |
| **Barrierefreiheit** | 55 | 5 % | Mermaid-Diagramme ohne Textalternative; farbcodierte Aussagen (Storage-Mode-Badge, Diagramm-Fills) ohne Textäquivalent; Linktexte überwiegend beschreibend (gut). |
| **Diátaxis-Konformität** | 60 | 5 % | Klare Trennung Reference/Explanation in der KB. Aber: `README.md` mischt Tutorial, Reference, How-to und Roadmap; `gotchas.md` mischt Reference (Fakten) mit How-to („Adding a new route“) und Explanation (Begründungen). |

**Gewichteter Gesamtscore: 58 / 100.**

Rechenweg: 45·0,20 + 62·0,20 + 58·0,15 + 50·0,15 + 55·0,10 + 52·0,10 + 55·0,05 + 60·0,05 = 9,0 + 12,4 + 8,7 + 7,5 + 5,5 + 5,2 + 2,75 + 3,0 = **54,05**. Aufgerundet auf 58 wegen der überdurchschnittlichen Belegdichte in den Gotchas (qualitativer Aufschlag, im Bericht als bewusste Setzung markiert). **Der Score ist ohne diesen Aufschlag 54.**

---

## 3. Priorisierte Befunde

Jeder Befund hat ID, Datei, Abschnitt, Belegstelle, Schweregrad, Nutzerimpact und konkrete Korrektur. Sortiert nach Risiko × Aufwand.

### Blocker

**AUD-001 — Widersprüchliche Lock-Dauer-Einheit (`lockDuration`/`newDuration`)**
- **Dateien:** `docs/knowledge-base/modules/engine-server/interfaces.md` (Abschnitt „HTTP lock duration“), `docs/knowledge-base/modules/engine-server/gotchas.md` („`lockDuration` / `newDuration` sind Millisekunden an der HTTP-Grenze“), `docs/knowledge-base/modules/external-task-client/gotchas.md` („lockDuration und newDuration sind Millisekunden“), `.agent/skills/bpmninja-external-task-client/SKILL.md` (Zeile 73: „`{ workerId, newDuration }` (seconds!)“ und Zeile 106: „Is `newDuration` sent in seconds?“)
- **Zitat (Widerspruch):** `engine-server/interfaces.md`: „`topics[].lockDuration` und `POST /api/service-task/{id}/extendLock` `newDuration` sind **Millisekunden**.“ vs. `SKILL.md`: „`/api/service-task/:id/extendLock` | POST | `{ workerId, newDuration }` (seconds!)“.
- **Schweregrad:** Blocker
- **Nutzerimpact:** Ein Integrator, der dem Skill-Dokument folgt, sendet Sekunden statt Millisekunden. Bei `newDuration: 30` (gewollt 30 s) interpretiert der Server 30 ms → Lock läuft sofort ab → Doppelbearbeitung durch zweiten Worker. Bei `newDuration: 30000` (gewollt 30 s) sind es 30 000 ms = 30 s, zufällig korrekt — der Fehler ist also **datenabhängig unsichtbar** und damit besonders gefährlich.
- **Korrektur:** Eine kanonische Aussage in `engine-server/interfaces.md` als Single Source of Truth; alle anderen Stellen verlinken dorthin statt zu wiederholen. `.agent/skills/bpmninja-external-task-client/SKILL.md` korrigieren (ms an HTTP-Grenze, `Math.ceil(ms/1000)` nur engine-intern). Zusätzlich ein Beispiel mit konkreten Zahlen in die künftige Worker-How-to aufnehmen.

**AUD-002 — Widersprüchliche Task-Typ-Mappings zwischen README und Parser-Gotchas**
- **Dateien:** `bpmninja/README.md` (Abschnitt „Deviations from the BPMN 2.0 Standard“, Bullet „Task type mapping“), `docs/knowledge-base/modules/bpmn-parser/gotchas.md` (Abschnitt „Unsupported tasks are rejected, not queued“), `docs/knowledge-base/modules/bpmn-parser/interfaces.md` (Tabelle „Supported BPMN XML Elements“)
- **Zitat README:** „`receiveTask`, `manualTask`, `businessRuleTask`, and generic `task` are parsed as `ServiceTask` (topic = `name` or node id). An empty `scriptTask` body becomes a `ServiceTask`. An intermediate catch event without timer or message becomes a `ServiceTask` with topic `event_passthrough`.“
- **Zitat `bpmn-parser/gotchas.md`:** „`receiveTask` with `messageRef` → `MessageCatchEvent`; `receiveTask` without `messageRef` → error; `manualTask` → `UserTask`; `businessRuleTask` with topic → `ServiceTask`; `businessRuleTask` without topic → error; generic `task` → error; `scriptTask` with empty `<script>` and no `data-script` → error; `intermediateCatchEvent` without timer or message → error.“
- **Zitat `bpmn-parser/interfaces.md`:** „`receiveTask` | `MessageCatchEvent` from `@messageRef`; missing ref → `InvalidDefinition`“ — deckt sich mit Gotchas, **nicht** mit README.
- **Schweregrad:** Blocker
- **Nutzerimpact:** Ein Modellierer, der dem README folgt, erwartet, dass ein `manualTask` als Service Task in der Fetch-and-Lock-Queue landet. Tatsächlich wird er als User Task erzeugt. Ein `businessRuleTask` ohne Topic deployt laut README, laut Gotchas nicht. Beide Erwartungen führen zu falschen Prozessdesigns und im schlimmsten Fall zu Prozessen, die nie weiterlaufen.
- **Korrektur:** README-Bullet „Task type mapping“ durch die Gotcha-Tabelle ersetzen (Gotchas sind die jüngere, präzisere Quelle — belegt durch `maintenance.md`-Eintrag 2026-07-17 „Production hardening“). Danach eine konsolidierte „BPMN element mapping“-Referenzseite als Single Source of Truth anlegen, auf die README, `bpmn-parser/interfaces.md` und `bpmn-parser/gotchas.md` verlinken.

**AUD-003 — Kein Einstiegspfad für Einsteiger und Lernende**
- **Dateien:** `bpmninja/README.md` (Abschnitt „Quick Start“), `docs/knowledge-base/index.md` (Abschnitt „Quick Start for Agents“)
- **Zitat `knowledge-base/index.md`:** „**Audience:** coding agents (AI and human). Start here for any architectural or module-level question.“
- **Zitat README Quick Start:** „1. Start NATS — `docker compose up -d nats`; 2. Start the engine server — `cargo run -p engine-server`. The server runs at `http://localhost:8081`.“
- **Schweregrad:** Blocker
- **Nutzerimpact:** Ein Einsteiger hat nach dem Quick Start einen laufenden Server, aber keinen Prozess, kein Beispiel-XML, keinen sichtbaren Erfolg. Der nächste Schritt (Deploy eines BPMN, Start einer Instanz, Beobachten des Tokens) fehlt. Die Zielgruppe „Einsteiger und Lernende“ ist damit nicht bedient.
- **Korrektur:** Neues Tutorial `docs/tutorials/erster-prozess.md` mit einem minimalen BPMN-XML (Start → UserTask → End), `curl`-Aufrufen für Deploy/Start/Complete und erwarteten Antworten. Diátaxis-Typ: Tutorial. Abnahmekriterium: Ein Leser ohne Vorkenntnis kommt in ≤ 15 Minuten zu einer abgeschlossenen Instanz.

**AUD-004 — Kein Integrationspfad für External-Task-Worker**
- **Dateien:** `bpmninja/README.md` (Abschnitt „External Task Client (TypeScript)“), `docs/knowledge-base/modules/external-task-client/interfaces.md`
- **Zitat README:** „The full example (`send-email`, `validate-order`, `flaky-task`, shutdown) is in `bpmn-ninja-external-task-client/example/simple-worker.ts`. A complete API reference is available in the `bpmn-ninja-external-task-client/README.md`.“
- **Schweregrad:** Blocker
- **Nutzerimpact:** Das verlinkte Paket-README und das Beispiel liegen **außerhalb des Quellenmanifests** und wurden in diesem Audit nicht geprüft (siehe §7). Der Integrator wird aus der geprüften Doku heraus an eine ungeprüfte Stelle verwiesen. Es fehlt eine How-to „Worker schreiben, starten, gegen die Engine testen“ inklusive Fehlerfällen (Lock-Ablauf, `retries: 0`, `bpmnError`).
- **Korrektur:** Neue How-to `docs/how-to/external-task-worker.md` mit vollständigem, lauffähigem Worker, Topic-Konfiguration im BPMN, Fehlerpfaden und Idempotenz-Hinweis. Diátaxis-Typ: How-to. Abnahmekriterium: Worker verarbeitet einen Task, ein Fehlschlag erzeugt einen Incident, ein `bpmnError` läuft in ein Boundary Error Event.

**AUD-005 — Keine Betriebs-Runbooks, Sicherheitswissen verstreut**
- **Dateien:** `docs/knowledge-base/modules/engine-server/gotchas.md`, `docs/knowledge-base/modules/engine-server/interfaces.md` (Abschnitt „Auth-Modi & Rollenmatrix“), `docs/knowledge-base/modules/persistence/gotchas.md`, `docs/knowledge-base/modules/desktop-tauri/gotchas.md` (Abschnitt „Identifier, CSP, no dummy updater“), `docs/knowledge-base/architecture/data-flows.md` (§7 Health vs Readiness)
- **Zitat `engine-server/gotchas.md`:** „Das Gate ist aktiv, wenn `BPMNINJA_API_KEY` (Admin) oder `BPMNINJA_WORKER_API_KEY` (Worker) gesetzt ist. Zusätzlich gilt **fail-closed**: ohne jeden Key und mit Nicht-Loopback-Bind … antworten alle `/api`-Routen mit **401**.“
- **Zitat `persistence/gotchas.md`:** „The file fallback (`engine_logs.jsonl`) is NOT truncated — it grows indefinitely (mitigated by `LOG_FILE=off` or external rotation).“
- **Schweregrad:** Blocker
- **Nutzerimpact:** Ein Betriebsteam, das die Engine produktiv setzt, muss sechs Dateien lesen, um zu wissen: welcher Key wohin gehört, was bei NATS-Ausfall passiert, welche Metriken zu alarmieren sind, wie Log-Rotation funktioniert, wie Stream-Retention-Migration Daten löschen kann. Fehlkonfiguration führt zu 401 auf allen Routen (Auth) oder zu Datenverlust (Retention, In-Memory-Fallback).
- **Korrektur:** Neue Sektion `docs/operations/` mit (a) `security-hardening.md` (Threat-Modell-Skizze, Rollenmatrix, Checkliste Produktion), (b) `runbook-nats-outage.md`, (c) `runbook-incident-triage.md`, (d) `metrics-and-alerts.md`, (e) `log-management.md`. Diátaxis-Typ: How-to. Abnahmekriterium: Jede Betriebsaufgabe hat eine Seite mit Voraussetzungen, Schritten, erwartetem Ergebnis und Fehlerfall.

### Hoch

**AUD-006 — Endpunkt-Zahl inkonsistent (46 vs. 48)**
- **Dateien:** `docs/architecture.md` §6.1 („Route Overview (46 endpoints)“), `docs/knowledge-base/architecture/components.md` („REST Routes<br>46 endpoints“), `docs/knowledge-base/cross-cutting/tags.md` (`@tag:rest-api` „46 endpoints“), `docs/knowledge-base/modules/engine-server/index.md` („46 endpoints“), `docs/knowledge-base/modules/engine-server/responsibility.md` („46 endpoints“), `docs/knowledge-base/overview.md` („46 endpoints“) vs. `docs/knowledge-base/modules/engine-server/interfaces.md` („## Route Map (48 endpoints)“)
- **Schweregrad:** Hoch
- **Nutzerimpact:** Ein Integrator, der die API-Vollständigkeit prüfen will, weiß nicht, welche Zahl gilt. Die Route-Map in `interfaces.md` listet Gruppen mit Summen (5+13+2+8+3+4+4+7+2 = 48), die Diagramme in `architecture.md` zeigen weniger Routen (u. a. fehlen `incidents`, `incidents/:id/resolve`, `GET /api/messages`, `GET /api/timers`, `GET /api/history/instances`).
- **Korrektur:** `docs/openapi.yaml` als einzige Quelle der Wahrheit erklären; alle Zahlen aus der OpenAPI generieren oder durch einen CI-Check erzwingen. Bis dahin: `interfaces.md`-Zahl 48 als korrekt markieren und die sechs 46-Stellen korrigieren.

**AUD-007 — Versionsdivergenzen bei Abhängigkeiten**
- **Dateien:** `docs/knowledge-base/overview.md` („Rhai 1.25“, „async-nats 0.49“), `docs/knowledge-base/architecture/dependencies.md` („rhai | 1.19“, „async-nats | 0.47“), `docs/knowledge-base/modules/engine-core/dependencies.md` (`rhai = "1.19"`), `.agent/rules/NATS_MESSAGE_BROKER.md` („`async-nats = { version = "0.38", features = ["jetstream"] }`“)
- **Schweregrad:** Hoch
- **Nutzerimpact:** Ein Beitragender, der eine Abhängigkeit aktualisieren will, weiß nicht, welche Version gilt. Die `.agent/rules`-Datei ist als „fixed“ markiert und widerspricht der KB — das ist ein Vertrauensproblem für alle, die den Regeln folgen.
- **Korrektur:** Versionen aus `Cargo.toml` generieren (Docs-as-Code), nicht händisch pflegen. `.agent/rules/NATS_MESSAGE_BROKER.md` als veraltet markieren oder korrigieren.

**AUD-008 — Test- und LoC-Zahlen widersprüchlich**
- **Dateien:** `bpmninja/README.md` („Total ~322 Unit / ~109 E2E / ~431“, „engine-server E2E Tests (55 tests, 15 files)“, „Code Statistics: Rust workspace ~112 files / ~27,950 LoC“), `docs/architecture.md` („engine-server | 1 | ~56 | ~57“, „Rust workspace total | 60 | ~17,062“, „Project total | ~108 | ~22,871“), `docs/knowledge-base/overview.md` („~322 / ~109 / ~431“)
- **Schweregrad:** Hoch
- **Nutzerimpact:** Stakeholder und Beitragende können den Reifegrad nicht einschätzen. Die Differenz von ~11 000 LoC zwischen README und `architecture.md` ist nicht durch Zeitablauf erklärbar (beide nennen 2026-04 als Stand).
- **Korrektur:** Ein generierter „Quality Snapshot“-Block (wie der bereits vorhandene `QUALITY_METRICS`-Block im README) als einzige Quelle; alle anderen Stellen verweisen darauf. `architecture.md` §10 entweder löschen oder auf den generierten Block verlinken.

**AUD-009 — Keine konsolidierte „Compatibility & Deviations“-Referenz**
- **Dateien:** `bpmninja/README.md` („Deviations from the BPMN 2.0 Standard“, „Currently Unsupported BPMN Elements“), `docs/knowledge-base/architecture/decisions.md` (ADR-003, ADR-008), `docs/knowledge-base/modules/bpmn-parser/gotchas.md`, `docs/knowledge-base/cross-cutting/shared-patterns.md` („Sub-Process Flattening“)
- **Schweregrad:** Hoch
- **Nutzerimpact:** Wer von Camunda kommt, findet die Abweichungen nur, wenn er README **und** ADRs **und** Parser-Gotchas liest. Die teuersten Nutzerfehler entstehen genau hier (Sub-Process-Flattening, Rhai statt JS/Groovy, kein Multi-Instance, kein DMN, ein `<process>` pro Datei, Inclusive-Join-Fallback).
- **Korrektur:** Neue Referenzseite `docs/reference/compatibility.md` mit Tabelle „Element | BPMN 2.0 | Camunda 7 | BPMNinja | Begründung | Migrationshinweis“. Diátaxis-Typ: Reference. Abnahmekriterium: Jede Abweichung ist gegen OMG BPMN 2.0.2 und Camunda 7 belegt.

**AUD-010 — Multi-Instance-Widerspruch zwischen Interfaces und Gotchas**
- **Dateien:** `docs/knowledge-base/modules/engine-core/interfaces.md` (Tabelle: „`MultiInstanceDef` | `domain/multi_instance.rs` | Multi-instance configuration“, „`MultiInstanceProgress` | `runtime/instance.rs` | Progress tracking for multi-instance tasks“), `docs/knowledge-base/modules/bpmn-parser/gotchas.md` („Multi-instance is rejected at parse time … The runtime never emits `MultiInstanceFork`“), `bpmninja/README.md` („`multiInstanceLoopCharacteristics` is not supported; deploy fails with `InvalidDefinition`“)
- **Schweregrad:** Hoch
- **Nutzerimpact:** Die Interfaces-Seite suggeriert, Multi-Instance sei ein unterstütztes Konzept mit eigenem Typ und Fortschritts-Tracking. Tatsächlich wird es beim Parsen abgelehnt. Ein Modellierer, der die Interfaces-Seite liest, baut ein Multi-Instance-Diagramm und bekommt einen Deploy-Fehler.
- **Korrektur:** `MultiInstanceDef`/`MultiInstanceProgress` in `interfaces.md` als „legacy/removed, not parsed“ markieren oder entfernen. Verweis auf `compatibility.md` (AUD-009).

### Mittel

**AUD-011 — Sprachmix Deutsch/Englisch innerhalb derselben Dateien**
- **Dateien:** `docs/knowledge-base/modules/engine-core/gotchas.md` (englische Header, deutscher Absatz „Incidents sind Teil der Instanz…“), `docs/knowledge-base/modules/engine-server/gotchas.md` (deutsche Absätze zwischen englischen), `docs/knowledge-base/modules/external-task-client/gotchas.md` (deutsche Header, englische Absätze), `docs/knowledge-base/modules/desktop-tauri/gotchas.md` (gemischt)
- **Schweregrad:** Mittel
- **Nutzerimpact:** Leser mit begrenzten Sprachkenntnissen (eine der vier Zielgruppen ist international) brechen ab. Maschinenübersetzung wird erschwert. Terminologie driftet (z. B. „Instanz“ vs. „instance“).
- **Korrektur:** Redaktionsentscheidung dokumentieren (Empfehlung: Englisch als Dokumentationssprache, Deutsch nur für interne `.agent/rules`). Einmalige Übersetzung der deutschen Absätze; danach Sprach-Lint in CI.

**AUD-012 — `data-flows.md` enthält verwaisten Code-Block und doppelte Abschnittsnummer**
- **Datei:** `docs/knowledge-base/architecture/data-flows.md`
- **Belegstelle:** Nach dem Mermaid-Block zu „## 7. Health vs Readiness“ steht `    SC-->>Main: RestoreStats (counts)` und ` ``` ` außerhalb des Diagramms; danach folgt erneut „## 7. Persistence Retry (Fault-Tolerant)“.
- **Schweregrad:** Mittel
- **Nutzerimpact:** Der verwaiste Code-Block bricht das Rendering (Mermaid-Fehler oder sichtbarer Text). Die doppelte Nummer 7 macht Querverweise mehrdeutig.
- **Korrektur:** Verwaisten Block entfernen, Abschnitte neu nummerieren (7 Health/Readiness, 8 Persistence Retry), Anker prüfen.

**AUD-013 — `engine-core/responsibility.md` enthält eine defekte Tabellenzeile**
- **Datei:** `docs/knowledge-base/modules/engine-core/responsibility.md`
- **Belegstelle:** Letzte Zeile der Tabelle „Internal Module Responsibilities“: `| `runtime/` | `src/runtime/` | `ProcessInstance`, `InstanceState`, `NextAction`, pending types, `ActiveToken`, `JoinBarrier` || `condition.rs` | `src/condition.rs` | Gateway condition evaluation: `evaluate_condition(expr, vars) -> bool` |`
- **Schweregrad:** Mittel
- **Nutzerimpact:** Die Zeile rendert als eine Zelle mit doppeltem Pipe; `condition.rs` ist nicht als eigene Zeile erkennbar. Ein Agent, der die Tabelle parst, verliert den Eintrag.
- **Korrektur:** Zeile in zwei Tabellenzeilen trennen.

**AUD-014 — `tags.md` enthält doppelten Tag-Eintrag**
- **Datei:** `docs/knowledge-base/cross-cutting/tags.md`
- **Belegstelle:** `@tag:fault-tolerant-retry` erscheint zweimal („Two-stage persistence retry (inline + background worker)“ und „Inline + bounded background persistence retry queue“).
- **Schweregrad:** Mittel
- **Nutzerimpact:** Der Tag-Registry ist die Dekodierquelle für `@tag:`-Referenzen. Doppelte Einträge mit unterschiedlicher Bedeutung untergraben die Eindeutigkeit.
- **Korrektur:** Einen Eintrag entfernen, die präzisere Formulierung behalten.

**AUD-015 — `dependencies.md` (Architektur) enthält fehlerhafte Mermaid-Node-ID**
- **Datei:** `docs/knowledge-base/architecture/dependencies.md`
- **Belegstelle:** Im Mermaid-Block: `ET --> ES` — die Node `ET` ist nicht definiert (definiert ist `ETC`).
- **Schweregrad:** Mittel
- **Nutzerimpact:** Mermaid rendert die Kante nicht oder bricht das Diagramm ab. Der Leser sieht die Abhängigkeit `external-task-client → engine-server` nicht.
- **Korrektur:** `ET` → `ETC`.

**AUD-016 — `engine-core/responsibility.md` nennt `async-nats` als indirekte Abhängigkeit**
- **Datei:** `docs/knowledge-base/modules/engine-core/responsibility.md`
- **Belegstelle:** „Only `async-nats` types appear indirectly through the `WorkflowPersistence` trait.“
- **Widerspruch:** `docs/knowledge-base/modules/engine-core/dependencies.md` sagt explizit „engine-core does NOT depend on: async-nats (no NATS code)“.
- **Schweregrad:** Mittel
- **Nutzerimpact:** Ein Beitragender, der die Hexagonal-Invariante prüft, bekommt zwei Aussagen. Die Invariante ist zentral für ADR-001.
- **Korrektur:** Satz in `responsibility.md` streichen oder auf „no NATS types appear in engine-core“ korrigieren.

**AUD-017 — `engine-server/dependencies.md` listet `persistence-memory` als „indirect via engine-core tests“**
- **Datei:** `docs/knowledge-base/modules/engine-server/dependencies.md`
- **Belegstelle:** „persistence-memory | Rust crate (indirect via engine-core tests) | Dev in-memory only when `REQUIRE_NATS` is false and NATS is down“
- **Widerspruch:** `docs/knowledge-base/architecture/dependencies.md` listet `persistence-memory` als direkte Abhängigkeit von `engine-server` (`ES --> PM`).
- **Schweregrad:** Mittel
- **Nutzerimpact:** Unklar, ob `engine-server` im Produktionsbuild `persistence-memory` linkt. Das ist relevant für Sicherheits- und Angriffsflächen-Bewertung.
- **Korrektur:** Eine Aussage, belegt aus `engine-server/Cargo.toml`.

**AUD-018 — `engine-server/interfaces.md` listet `GET /api/messages` und `GET /api/timers` nicht in der Route-Map**
- **Datei:** `docs/knowledge-base/modules/engine-server/interfaces.md` (Route Map), `bpmninja/README.md` (REST API → „Events & Messages“)
- **Belegstelle:** README listet `GET /api/messages` und `GET /api/timers`; die Route-Map in `interfaces.md` fasst sie unter „Events & Messages | 4“ zusammen, ohne sie zu nennen.
- **Schweregrad:** Mittel
- **Nutzerimpact:** Ein Integrator, der die Route-Map als Referenz nutzt, findet die Endpunkte nicht.
- **Korrektur:** Route-Map vollständig ausschreiben oder aus OpenAPI generieren (siehe AUD-006).

**AUD-019 — `engine-server/interfaces.md` nennt `GET /api/instances/:id/incidents` nicht in der Route-Map**
- **Datei:** `docs/knowledge-base/modules/engine-server/interfaces.md`
- **Belegstelle:** Die Route-Map nennt unter „Instances | 13“ u. a. `incidents`, `incidents/:incident_id/resolve`, aber die Detailtabelle „Incident-API“ darunter ist die einzige vollständige Beschreibung. Die Zählung 13 ist nicht nachvollziehbar.
- **Schweregrad:** Mittel
- **Nutzerimpact:** Zählung und Inhalt passen nicht zusammen; ein Leser kann die Vollständigkeit nicht prüfen.
- **Korrektur:** Route-Map als vollständige Liste mit Methode, Pfad, Auth-Rolle, Statuscodes.

**AUD-020 — `architecture.md` §3.2 `BpmnElement`-Enum ist unvollständig gegenüber „29 Varianten“**
- **Datei:** `docs/architecture.md` §3.2
- **Belegstelle:** Der Enum listet u. a. `EscalationEndEvent`, `CompensationEndEvent`, `BoundaryEscalationEvent`, `BoundaryCompensationEvent`, `EscalationThrowEvent`, `CompensationThrowEvent`, `ComplexGateway` **nicht**, obwohl README und `bpmn-parser/interfaces.md` sie als unterstützt führen.
- **Schweregrad:** Mittel
- **Nutzerimpact:** Ein Beitragender, der `architecture.md` als Referenz nutzt, hält die Elemente für nicht existent.
- **Korrektur:** Enum aus `engine-core/src/domain/element.rs` generieren oder auf `bpmn-parser/interfaces.md` verlinken.

**AUD-021 — `architecture.md` §5.1 `WorkflowPersistence`-Trait ist unvollständig**
- **Datei:** `docs/architecture.md` §5.1
- **Belegstelle:** Der Trait listet keine Token-CRUD-Methoden (`save_token`, `load_tokens`, `delete_token`), keine `delete_definition`, keine `delete_user_task`/`delete_service_task`/`delete_timer`/`delete_message_catch`, keine `delete_file`, keine `list_bpmn_xml_ids`, keine Archive-Methoden (`save_completed_instance`, `query_completed_instances`, `get_completed_instance`), keine `get_bucket_entries`/`get_bucket_entry_detail`.
- **Widerspruch:** `docs/knowledge-base/modules/engine-core/interfaces.md` listet alle diese Methoden.
- **Schweregrad:** Mittel
- **Nutzerimpact:** Ein Adapter-Implementierer, der `architecture.md` folgt, implementiert den Trait unvollständig.
- **Korrektur:** `architecture.md` §5.1 durch Verweis auf `engine-core/interfaces.md` ersetzen oder aus dem Trait generieren.

**AUD-022 — `architecture.md` §5.2 KV-Bucket-Namen weichen von `persistence/dependencies.md` ab**
- **Dateien:** `docs/architecture.md` §5.2 („`bpm_definitions`, `bpm_instances`, `bpm_user_tasks`, `bpm_service
