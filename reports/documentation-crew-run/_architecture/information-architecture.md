# Informationsarchitektur „BPMNinja" — IA-Entwurf, Dateiplan, Migrationsmatrix und Landingpage

**Ausgabewurzel:** `/Volumes/SSD2TB/work/bpmninja/reports/documentation-crew-run`
**Autor:** Documentation Lead / Redaktionsmanagement
**Stand:** 2026-10-24
**Grundlage:** `_research/domain-research.md`, `_audit/audit-report.md` sowie Primärlesung von `bpmninja/README.md`, `docs/architecture.md`, `docs/knowledge-base/index.md`, `overview.md`, `maintenance.md`, `architecture/{decisions,data-flows,dependencies,components}.md`, `cross-cutting/{shared-patterns,tags}.md` und der Moduldateien `engine-core/{interfaces,gotchas,responsibility,dependencies,index}.md`, `engine-server/{interfaces,gotchas,responsibility,dependencies,index}.md`, `bpmn-parser/{interfaces,gotchas,responsibility,dependencies}.md`, `persistence/{interfaces,gotchas,responsibility,dependencies}.md`, `external-task-client/{interfaces,gotchas,responsibility,dependencies}.md`, `desktop-tauri/{interfaces,gotchas,responsibility,dependencies}.md`, `api-spec/{interfaces,responsibility}.md`, `fuzz/{index,responsibility}.md`, `agent-orchestrator/index.md`.

---

## 0. Redaktionsentscheidungen, die dieser IA zugrunde liegen

Jede Strukturentscheidung nennt die Nutzeraufgabe, die sie löst. Diese sieben Entscheidungen sind bindend für alle nachfolgenden Dokumente.

| # | Entscheidung | Nutzeraufgabe, die sie löst | Beleg / Begründung |
|---|---|---|---|
| E1 | **Dokumentationssprache ist Deutsch.** Code, API-Pfade, Feldnamen, Env-Variablen und Fehlercodes bleiben unübersetzt. | Einsteiger und Betriebsteams lesen in ihrer Arbeitssprache; Terminologie driftet nicht mehr zwischen „Instanz" und „instance". | Auftrag nennt „Dokumentationssprache: Deutsch". Audit AUD-011 belegt Sprachmix innerhalb derselben Dateien (`engine-core/gotchas.md`, `engine-server/gotchas.md`, `external-task-client/gotchas.md`, `desktop-tauri/gotchas.md`). |
| E2 | **Diátaxis wird strikt getrennt: `tutorials/`, `how-to/`, `reference/`, `explanation/`.** Betriebs-Runbooks liegen als How-to unter `how-to/betrieb/`. | Ein Leser, der lernen will, wird nicht mit Referenztabellen konfrontiert; ein Betriebsteam, das einen Ausfall behebt, muss kein Tutorial lesen. | Diátaxis Map warnt vor dem „Blur" zwischen Tutorial und How-to (Research §3.1). Audit §2 „Diátaxis-Konformität 60/100": `README.md` mischt Tutorial, Reference, How-to und Roadmap; `gotchas.md` mischt Reference, How-to und Explanation. |
| E3 | **Die Knowledge Base bleibt als Referenz- und Explanation-Schicht erhalten, wird aber nicht mehr als Einstieg beworben.** `docs/knowledge-base/index.md` wird zur Beitragenden-Referenz umetikettiert. | Beitragende und Maintainer behalten ihre tiefe Moduldokumentation; Einsteiger werden nicht mehr als Erstes dorthin geschickt. | `docs/knowledge-base/index.md`: „Audience: coding agents (AI and human)." Audit AUD-003. |
| E4 | **`docs/openapi.yaml` ist die einzige Quelle der Wahrheit für die REST-API.** Jede Endpunktzahl in Fließtext wird durch einen Verweis ersetzt. | Ein Integrator kann die API-Vollständigkeit prüfen, ohne sechs widersprüchliche Zahlen zu vergleichen. | Audit AUD-006 (46 vs. 48 Endpunkte an sieben Stellen), AUD-018, AUD-019. `api-spec/interfaces.md`: „`docs/openapi.yaml` … (authoritative)". |
| E5 | **Jede Seite trägt Front-Matter nach Standard F1 (Abschnitt 5).** `status` und `last_reviewed` sind Pflicht. | Ein Leser erkennt vor dem Lesen, ob eine Seite geprüft, veraltet oder Entwurf ist; der Redaktionsprozess kann Freigaben belegen. | Audit §2 „Aktualität 55/100": mehrere Stände parallel, keine Seite nennt einen „gültig ab"-Stand. |
| E6 | **Abweichungen von BPMN 2.0 und Camunda bekommen genau eine Referenzseite** (`reference/kompatibilitaet.md`), auf die README, Parser-Interfaces und Parser-Gotchas verlinken. | Wer von Camunda kommt, findet alle Abweichungen an einer Stelle statt in README + ADRs + Parser-Gotchas. | Audit AUD-002, AUD-009, AUD-010. Research §1 Erkenntnis 2. |
| E7 | **Lock-Dauer-Semantik wird an genau einer Stelle kanonisch definiert** (`reference/service-task-api.md`), alle anderen Seiten verlinken. | Ein Integrator sendet nie versehentlich Sekunden statt Millisekunden. | Audit AUD-001 (Blocker): `engine-server/interfaces.md` sagt ms, `.agent/skills/bpmninja-external-task-client/SKILL.md` sagt „seconds!". `engine-server/interfaces.md` §„HTTP lock duration" ist die belegte Wahrheit: „sind **Millisekunden** … `(ms + 999) / 1000` (ceil)". |

---

## 1. Nutzergruppen und ihre Aufgaben

Sechs Rollen, jede mit Einstieg, typischer Frage, Erfolgskriterium und bevorzugtem Content-Typ. Die Rollen sind aus dem Auftrag (vier Zielgruppen) und aus Research §2.2 (Akteure) abgeleitet; „Prozessmodellierer" und „Beitragende/Maintainer" sind als eigenständige Rollen ergänzt, weil sie im Audit als faktisch bediente, aber nicht adressierte Gruppen belegt sind.

### 1.1 Einsteiger und Lernende

- **Einstieg:** `README.md` → „Ich lerne BPMNinja" → `tutorials/erster-prozess.md`
- **Typische Frage:** „Wie bekomme ich einen Prozess von XML bis zum sichtbaren Abschluss laufen?"
- **Erfolgskriterium:** Eine Prozessinstanz ist in ≤ 15 Minuten von `Deploy` bis `Completed` gelaufen, ohne dass der Leser die Knowledge Base öffnen musste.
- **Bevorzugter Content-Typ:** Tutorial (eine Lektion, garantiertes Ergebnis), danach Explanation zum Verstehen.
- **Belegte Lücke:** Audit AUD-003. README-Quickstart endet bei `cargo run -p engine-server`; kein Beispiel-XML, kein `curl`-Aufruf, kein sichtbarer Erfolg.

### 1.2 Fortgeschrittene Nutzer / Prozessmodellierer

- **Einstieg:** `README.md` → „Ich modelliere Prozesse" → `how-to/prozess-deployen.md` und `reference/bpmn-elemente.md`
- **Typische Frage:** „Warum deployt mein Diagramm nicht — und was passiert mit `manualTask`, `businessRuleTask`, `multiInstanceLoopCharacteristics`?"
- **Erfolgskriterium:** Der Modellierer kennt vor dem Deploy die Element-Abbildung und die Ablehnungsfälle und kann einen Deploy-Fehler einer konkreten Regel zuordnen.
- **Bevorzugter Content-Typ:** How-to (Ziel erreichen) + Reference (Elementverhalten).
- **Belegte Lücke:** Audit AUD-002 (README widerspricht Parser-Gotchas bei `receiveTask`, `manualTask`, `businessRuleTask`, generischem `task`, leerem `scriptTask`), AUD-010 (Multi-Instance in `engine-core/interfaces.md` als unterstützt gelistet, in `bpmn-parser/gotchas.md` als Parse-Fehler).

### 1.3 Entwickler und Integratoren

- **Einstieg:** `README.md` → „Ich baue einen Worker" → `how-to/external-task-worker.md`
- **Typische Frage:** „Wie schreibe ich einen Worker, der einen Task lockt, abschließt, bei Fehlschlag einen Incident erzeugt und bei fachlichem Fehler in ein Boundary Error Event läuft?"
- **Erfolgskriterium:** Ein Worker verarbeitet einen Task end-to-end; ein `failure` mit `retries: 0` erzeugt einen sichtbaren Incident; ein `bpmnError` läuft in den Boundary-Pfad; `lockDuration` wird in Millisekunden gesendet.
- **Bevorzugter Content-Typ:** How-to + Reference (API-Verträge).
- **Belegte Lücke:** Audit AUD-004 (README verweist auf ein Paket-README außerhalb des Manifests), AUD-001 (Lock-Einheit widersprüchlich).

### 1.4 Betriebsteams und Plattform-Administratoren

- **Einstieg:** `README.md` → „Ich betreibe BPMNinja" → `how-to/betrieb/produktion-checkliste.md`
- **Typische Frage:** „Welche Keys muss ich setzen, was passiert bei NATS-Ausfall, welche Metriken alarmiere ich, und wie verhindere ich Datenverlust bei der Stream-Retention-Migration?"
- **Erfolgskriterium:** Jede Betriebsaufgabe (Auth, Readiness, NATS-Ausfall, Incident-Triage, Log-Rotation, Retention-Migration) hat eine Seite mit Voraussetzungen, Schritten, erwartetem Ergebnis und Fehlerfall.
- **Bevorzugter Content-Typ:** How-to (Runbook) + Reference (Env-Variablen, Metriken).
- **Belegte Lücke:** Audit AUD-005. Fail-closed-Auth, `REQUIRE_NATS`, Retry-Queue-Drops, SSE-Verluste, unbegrenzt wachsende Log-Datei und Retention-Migration sind über sechs Dateien verstreut (`engine-server/gotchas.md`, `engine-server/interfaces.md`, `persistence/gotchas.md`, `desktop-tauri/gotchas.md`, `data-flows.md` §7, `shared-patterns.md`).

### 1.5 Stakeholder und Entscheider

- **Einstieg:** `README.md` → „Ich bewerte BPMNinja" → `explanation/architektur-ueberblick.md` und `explanation/grenzen-und-trade-offs.md`
- **Typische Frage:** „Was kann die Engine, was kann sie bewusst nicht, welche Betriebsrisiken sind eingebaut, und was bedeutet ‚Camunda-kompatibel' konkret?"
- **Erfolgskriterium:** Ein Entscheider kann nach zwei Seiten eine Eignungsentscheidung begründen und kennt die drei größten Betriebsrisiken (fail-closed Auth, begrenzte Retry-Queue mit Drop, Stream-Retention-Migration).
- **Bevorzugter Content-Typ:** Explanation (diskursiv, Trade-offs, keine Schrittfolgen).
- **Belegte Lücke:** Audit §1 Befund 5. Keine Eignungs- oder Risikoseite vorhanden.

### 1.6 Beitragende und Maintainer

- **Einstieg:** `docs/knowledge-base/index.md` (bleibt als Beitragenden-Einstieg) → `how-to/beitragen/dokumentation-pflegen.md`
- **Typische Frage:** „Wo ändere ich was, wenn ich eine Route, ein BPMN-Element oder eine Abhängigkeit ändere — und welche Datei ist dann die Wahrheit?"
- **Erfolgskriterium:** Ein Beitragender findet über die Änderungsmatrix in `how-to/beitragen/dokumentation-pflegen.md` alle betroffenen Dateien, ohne den Bestand zu durchsuchen.
- **Bevorzugter Content-Typ:** Reference (interne Invarianten) + How-to (Beitragsprozess).
- **Belegte Stärke, die erhalten bleibt:** `maintenance.md` enthält bereits eine „When to Update"-Liste und eine Dateistruktur-Konvention; diese wird in die Änderungsmatrix überführt, nicht ersetzt.

---

## 2. Diátaxis-Zuordnung

### 2.1 Grundregel

Jede Seite hat genau einen `content_type`. Mischformen sind unzulässig; wo der Bestand sie enthält, wird getrennt. Die Zuordnung folgt dem Nutzerbedürfnis, nicht dem Produktfeature: Ein und dasselbe Thema (z. B. External Tasks) erscheint in bis zu drei Quadranten mit unterschiedlicher Aufgabe.

| Quadrant | Ordner | Nutzerbedürfnis | Pflichtmerkmale | Verboten |
|---|---|---|---|---|
| **Tutorial** | `tutorials/` | „Bring mir das bei." | Garantiertes Ergebnis, nummerierte Schritte, erwartete Ausgabe nach jedem Schritt, keine Alternativen, keine Referenztabellen | „Wenn Sie stattdessen …", Optionsmatrizen, Fehlerkataloge |
| **How-to** | `how-to/` | „Wie erreiche ich X?" | Ziel im Titel, Voraussetzungen, Schrittfolge, erwartetes Ergebnis, Fehlerfall | Vollständige API-Listen, Grundsatzdiskussionen, Lernprogression |
| **Reference** | `reference/` | „Was ist X?" | Tabellen, Verträge, Statuscodes, Defaults, Grenzwerte, keine Anleitungsprosa | „In diesem Tutorial …", Empfehlungen, Tutorialschritte |
| **Explanation** | `explanation/` | „Warum ist das so?" | Diskurs, Trade-offs, Alternativen, Begründungen, Verweise auf ADRs | Schrittfolgen, Befehle zum Kopieren, vollständige Tabellen |

### 2.2 Konkrete Zuordnung des Bestands und der Neuanlagen

| Inhalt | Ziel-Quadrant | Begründung (Nutzeraufgabe) |
|---|---|---|
| Erster Prozess deployen, starten, abschließen | `tutorials/` | Lernen durch Tun; ein Ergebnis, keine Optionen. |
| Worker schreiben und gegen die Engine testen | `how-to/` | Zielorientiert; der Leser hat bereits ein Modell. |
| Prozess deployen und Versionen verwalten | `how-to/` | Zielorientiert; setzt Modell voraus. |
| Produktion in Betrieb nehmen (Auth, Readiness, Retention) | `how-to/betrieb/` | Runbook; der Leser ist im Betrieb, nicht im Lernen. |
| NATS-Ausfall behandeln | `how-to/betrieb/` | Runbook mit Eskalationspfad. |
| Incident triagieren und auflösen | `how-to/betrieb/` | Runbook; Incident-Modell ist Referenz, Triage ist How-to. |
| BPMN-Element-Abbildung (XML → `BpmnElement`) | `reference/` | Nachschlagen; `bpmn-parser/interfaces.md` ist die belegte Quelle. |
| Kompatibilität und Abweichungen (BPMN 2.0 / Camunda 7) | `reference/` | Nachschlagen; Entscheidung E6. |
| Service-Task-API inkl. Lock-Dauer in ms | `reference/` | Vertrag; Entscheidung E7. |
| REST-API-Übersicht | `reference/` | Verweist auf `docs/openapi.yaml` (E4). |
| Konfiguration und Env-Variablen | `reference/` | Defaults und Grenzwerte; heute in `shared-patterns.md` und `engine-server/dependencies.md` verstreut. |
| Fehlercodes und HTTP-Status | `reference/` | `engine-server/interfaces.md` §„AppError → HTTP Status Mapping" ist vollständig und bleibt Quelle. |
| Glossar und „Nicht verwechseln" | `reference/` | Nachschlagen; Research §2.6 liefert 19 belegte Verwechslungspaare. |
| Token-basierte Ausführung | `explanation/` | Verstehen; ADR-002 ist die Begründung. |
| Architekturüberblick für Entscheider | `explanation/` | Bewerten; ADR-001, ADR-004, ADR-006. |
| Grenzen und Trade-offs | `explanation/` | Bewerten; ADR-003, ADR-008, ADR-009, ADR-010. |
| Modul-Referenz (Responsibility/Dependencies/Interfaces/Gotchas) | `reference/` (Knowledge Base) | Beitragende; bleibt bestehen, wird umetikettiert. |
| ADRs, Datenflüsse, Shared Patterns | `explanation/` (Knowledge Base) | Begründungen; bleiben bestehen. |

### 2.3 Mischformen im Bestand und ihre Trennung

| Mischform (belegt) | Was gemischt ist | Trennung |
|---|---|---|
| `bpmninja/README.md` | Tutorial (Quick Start), Reference (Element-Tabellen, REST-API, Env-Variablen), How-to (Docker Compose, Desktop-Installation), Explanation (Deviations), Roadmap | README wird zur Landingpage (Abschnitt 7). Quick Start → `tutorials/erster-prozess.md`. Element-Tabellen → `reference/bpmn-elemente.md`. REST-API → `reference/rest-api.md`. Deviations → `reference/kompatibilitaet.md`. Docker/Desktop → `how-to/`. Roadmap entfällt aus der Doku (Google Style Guide: nichts „ankündigen"). |
| `engine-server/gotchas.md` | Reference (Fakten, Statuscodes), How-to („Adding a new route", Schritt 1–5), Explanation (Begründungen) | Fakten → `reference/rest-api.md` und `reference/konfiguration.md`. „Adding a new route" → `how-to/beitragen/route-hinzufuegen.md`. Begründungen → `explanation/architektur-ueberblick.md`. |
| `bpmn-parser/gotchas.md` | Reference (Mapping-Tabelle), Explanation (Flattening-Begründung), How-to („Adding a new BpmnElement variant") | Mapping → `reference/bpmn-elemente.md`. Flattening → `explanation/architektur-ueberblick.md`. Variant-Anleitung → `how-to/beitragen/bpmn-element-hinzufuegen.md`. |
| `persistence/gotchas.md` | Reference (Bucket-Namen, Retention-Defaults), How-to („Adding a new entity type"), Explanation (Retry-Begründung) | Buckets/Retention → `reference/konfiguration.md`. Entity-Anleitung → `how-to/beitragen/persistenz-entity-hinzufuegen.md`. Retry → `explanation/grenzen-und-trade-offs.md`. |
| `engine-core/gotchas.md` | Reference (Invarianten), How-to (Codebeispiele „so nicht"), Explanation (Incident-Lebenszyklus) | Invarianten bleiben in der Knowledge Base als Beitragenden-Referenz; Incident-Lebenszyklus → `explanation/architektur-ueberblick.md`; Codebeispiele bleiben als Beitragenden-How-to in der Knowledge Base. |
| `docs/architecture.md` | Explanation (Architektur), Reference (Trait-Liste, Enum, KV-Buckets, Code-Statistik) | Architektur → `explanation/architektur-ueberblick.md`. Trait/Enum → Verweis auf `engine-core/interfaces.md` (Audit AUD-020, AUD-021). KV-Buckets → `reference/konfiguration.md` (Audit AUD-022). Code-Statistik → entfällt bzw. verweist auf den generierten `QUALITY_METRICS`-Block (Audit AUD-008). |

---

## 3. Dateiplan

Alle Pfade relativ zu `docs/`. Priorität: **P0** = Veröffentlichungsblocker, **P1** = zur ersten Freigabe erforderlich, **P2** = Folgeausbaustufe. Jede Zeile nennt Zielgruppe, Zweck, Abhängigkeiten und die Nutzeraufgabe.

### 3.1 `tutorials/`

| Pfad | Titel | Zweck | Zielgruppe | Abhängigkeiten | Prio |
|---|---|---|---|---|---|
| `tutorials/erster-prozess.md` | Ersten Prozess ausführen | Ein minimales BPMN-XML (Start → UserTask → End) deployen, starten, User Task abschließen und `Completed` beobachten — mit `curl` und erwarteter Ausgabe nach jedem Schritt. | Einsteiger und Lernende | `reference/rest-api.md` (Endpunkt-Verträge), `reference/bpmn-elemente.md` (welche Elemente erlaubt sind) | P0 |
| `tutorials/erster-worker.md` | Ersten External-Task-Worker bauen | Service Task im Modell, Topic vergeben, TypeScript-Worker starten, Task abschließen, Ergebnis im Instanzzustand sehen. | Einsteiger mit Entwicklerhintergrund | `tutorials/erster-prozess.md`, `reference/service-task-api.md` | P1 |

**Abnahmekriterium `erster-prozess.md`:** Ein Leser ohne BPMN-Vorkenntnis erreicht in ≤ 15 Minuten eine abgeschlossene Instanz; jede `curl`-Antwort ist als erwartete Ausgabe abgedruckt; kein Schritt verweist auf eine ungeprüfte externe Datei.

### 3.2 `how-to/`

| Pfad | Titel | Zweck | Zielgruppe | Abhängigkeiten | Prio |
|---|---|---|---|---|---|
| `how-to/external-task-worker.md` | External-Task-Worker schreiben und testen | Vollständiger Worker mit `fetchAndLock`, `complete`, `failure` (`retries: 0` → Incident), `bpmnError` (→ Boundary Error Event), `extendLock` in **Millisekunden**, Idempotenz-Hinweis. | Entwickler und Integratoren | `reference/service-task-api.md` (E7), `reference/fehlercodes.md` | P0 |
| `how-to/prozess-deployen.md` | Prozess deployen und Versionen verwalten | Deploy, Versionsinkrement, `start/latest`, Löschen mit/ohne `?cascade=true`, `DefinitionHasInstances` (409), XML-Abruf als `text/plain`. | Fortgeschrittene Nutzer / Modellierer | `reference/bpmn-elemente.md`, `reference/rest-api.md` | P0 |
| `how-to/betrieb/produktion-checkliste.md` | Produktion in Betrieb nehmen | Auth-Keys setzen, `BIND_ADDR` bewusst wählen, `REQUIRE_NATS=true`, CORS-Allowlist, Body-Limits, Retention-Variablen **vor** dem ersten Upgrade-Start setzen, Log-Rotation. | Betriebsteams / Plattform-Administratoren | `reference/konfiguration.md`, `how-to/betrieb/nats-ausfall.md` | P0 |
| `how-to/betrieb/nats-ausfall.md` | NATS-Ausfall behandeln | Erkennen (`/api/ready` 503, `storage_status.state: "failed"`), Retry-Queue-Drops (`bpmn_persistence_retry_dropped_total`) bewerten, Eskalation, Wiederanlauf und Startup-Restore prüfen. | Betriebsteams | `reference/metriken.md`, `explanation/grenzen-und-trade-offs.md` | P0 |
| `how-to/betrieb/incident-triage.md` | Incidents triagieren und auflösen | Incident-Arten (`service_task`, `timer`, `persistence`, `bpmn_error`) unterscheiden, `retry` vs. `resolve` wählen, Admin-Rolle beachten (Worker-Key → 403), Dedup über `(kind, node_id, task_id)` verstehen. | Betriebsteams | `reference/fehlercodes.md`, `reference/service-task-api.md` | P1 |
| `how-to/betrieb/log-und-metriken.md` | Logs und Metriken betreiben | Log-Buffer (5 000) vs. `ENGINE_LOGS` (50 000) vs. Datei-Fallback (unbegrenzt), `LOG_FILE=off`, Prometheus-Scrape, `/metrics` nur bei gesetztem Handle. | Betriebsteams | `reference/metriken.md`, `reference/konfiguration.md` | P1 |
| `how-to/beitragen/dokumentation-pflegen.md` | Dokumentation pflegen | Änderungsmatrix: welche Änderung welche Datei berührt; Review- und Freigabeprozess; Front-Matter-Pflichtfelder. | Beitragende / Maintainer | `_architecture/content-plan.md` | P1 |
| `how-to/beitragen/route-hinzufuegen.md` | Neue REST-Route hinzufügen | Handler, Registrierung, `AppError`-Mapping, E2E-Test, `docs/openapi.yaml` aktualisieren. | Beitragende | `reference/rest-api.md` | P2 |
| `how-to/beitragen/bpmn-element-hinzufuegen.md` | Neues BPMN-Element hinzufügen | Variante in `BpmnElement`, Parser-Mapping, Handler, Test. | Beitragende | `reference/bpmn-elemente.md` | P2 |
| `how-to/beitragen/persistenz-entity-hinzufuegen.md` | Neue persistente Entity hinzufügen | Trait-Methode, NATS-Bucket, In-Memory-Adapter, `PersistJob`, Restore, `RestoreStats`. | Beitragende | `reference/konfiguration.md` | P2 |

**Abnahmekriterium `external-task-worker.md`:** Worker verarbeitet einen Task; `failure` mit `retries: 0` erzeugt einen Incident; `bpmnError` läuft in ein Boundary Error Event; `lockDuration: 30000` wird als 30 s wirksam (nicht 30 ms).

### 3.3 `reference/`

| Pfad | Titel | Zweck | Zielgruppe | Abhängigkeiten | Prio |
|---|---|---|---|---|---|
| `reference/index.md` | Referenz | **Referenzlandingpage** mit Tabellen der Inhalte: Elemente, Kompatibilität, Service-Task-API, REST-API, Konfiguration, Fehlercodes, Glossar. | Alle | alle Detailseiten | P0 |
| `reference/bpmn-elemente.md` | BPMN-Elemente und Abbildung | XML-Tag → `BpmnElement`-Variante, Ablehnungsfälle (`InvalidDefinition`), Topic-/Assignee-Fallbacks, Timer-Formate, ein `<process>` pro Datei. Quelle: `bpmn-parser/interfaces.md` + `bpmn-parser/gotchas.md`. | Modellierer, Integratoren | `reference/kompatibilitaet.md` | P0 |
| `reference/kompatibilitaet.md` | Kompatibilität und Abweichungen | Tabelle „Element \| BPMN 2.0 \| Camunda 7 \| BPMNinja \| Begründung \| Migrationshinweis" für Sub-Process-Flattening, Rhai statt JS/Groovy, kein Multi-Instance, kein DMN, Task-Typ-Mapping, Inclusive-Join-Fallback, ein `<process>` pro Datei. | Modellierer, Entscheider, Integratoren | OMG BPMN 2.0.2, Camunda 7 External Tasks (Research §3.2) | P0 |
| `reference/service-task-api.md` | Service-Task-API | `fetchAndLock`-Validierung (1..=32 Topics, `lockDuration` 1 000..=3 600 000 **ms**, `maxTasks` 1..=100), `complete`/`failure`/`bpmnError`/`extendLock`/`retry`/`resolve`, Lock-Ablauf und Doppelbearbeitung, Idempotenz. **Kanonische Stelle für E7.** | Integratoren, Betriebsteams | `docs/openapi.yaml` | P0 |
| `reference/rest-api.md` | REST-API-Übersicht | Routengruppen mit Verweis auf `docs/openapi.yaml` als Quelle der Wahrheit; Auth-Rolle je Route; Fehlerformat `{ "error": … }`. Keine eigene Endpunktzahl. | Integratoren | `docs/openapi.yaml` (E4) | P0 |
| `reference/konfiguration.md` | Konfiguration und Umgebungsvariablen | Alle Env-Variablen mit Default und Wirkung: `NATS_URL`, `REQUIRE_NATS`, `PORT`, `BIND_ADDR`, `BPMNINJA_API_KEY`, `BPMNINJA_WORKER_API_KEY`, `BPMNINJA_ALLOW_UNAUTHENTICATED`, `CORS_ORIGINS`, `MAX_XML_BYTES`, `MAX_UPLOAD_BYTES`, `TIMER_INTERVAL_MS`, `RUST_LOG`, `LOG_FORMAT`, `LOG_FILE`, `RHAI_*`, `PERSISTENCE_RETRY_QUEUE_CAPACITY`, `BPMNINJA_EVENT_RETENTION_DAYS`, `BPMNINJA_HISTORY_RETENTION_DAYS`, `BPMNINJA_STREAM_MAX_BYTES`, `BPMNINJA_STREAM_REPLICAS`. | Betriebsteams, Beitragende | `shared-patterns.md`, `persistence/gotchas.md`, `engine-server/interfaces.md` | P0 |
| `reference/fehlercodes.md` | Fehlercodes und HTTP-Status | `EngineError` → HTTP-Status, `AppError`-Fälle (400/401/403/404/409/413/415/422/500), Incident-Arten, `OrphanedToken` (422). | Integratoren, Betriebsteams | `engine-server/interfaces.md` | P1 |
| `reference/metriken.md` | Metriken und Alarme | `bpmn_auth_failures_total{reason}`, `bpmn_persistence_retry_dropped_total`, `bpmn_persistence_retry_exhausted_total`, `bpmn_incidents_total{kind}`, `/api/health` vs. `/api/ready`. | Betriebsteams | `engine-server/responsibility.md`, `engine-core/interfaces.md` | P1 |
| `reference/glossar.md` | Glossar und „Nicht verwechseln" | 19 belegte Verwechslungspaare aus Research §2.6 (Definition ↔ Instance, Service Task ↔ External Task, BPMN Error ↔ technischer Fehler, Migration ↔ Modification, Liveness ↔ Readiness, Suspend ↔ Terminate, ms ↔ s, `bpmn_process_id` ↔ `bpmn_id` u. a.). | Alle | Research §2.6 | P1 |

**Abnahmekriterium `kompatibilitaet.md`:** Jede Abweichung ist gegen OMG BPMN 2.0.2 und Camunda 7 belegt; jede Zeile nennt einen Migrationshinweis; keine Aussage stammt aus dem widersprüchlichen README-Bullet „Task type mapping" (Audit AUD-002).

### 3.4 `explanation/`

| Pfad | Titel | Zweck | Zielgruppe | Abhängigkeiten | Prio |
|---|---|---|---|---|---|
| `explanation/architektur-ueberblick.md` | Architektur verstehen | Token-basierte Ausführung (ADR-002), Ports & Adapters (ADR-001), Lock-freie Wait-State-Queues (ADR-004), SSE-Push (ADR-006), Sub-Process-Flattening (ADR-003), Startup-Restore. Mit Textalternativen zu allen Diagrammen. | Einsteiger (nach dem Tutorial), Entscheider, Beitragende | `architecture/decisions.md`, `architecture/data-flows.md` | P0 |
| `explanation/grenzen-und-trade-offs.md` | Grenzen und Trade-offs | Was BPMNinja bewusst nicht kann (kein DMN, kein Multi-Instance, kein Signal/Cancel/Link-Event, keine Transaktions-/Ad-Hoc-Sub-Prozesse), warum (ADR-003, ADR-008), und welche Betriebsrisiken daraus folgen (fail-closed Auth ADR-009, begrenzte Retry-Queue ADR-010, SSE-Verluste ADR-006, Retention-Migration). | Entscheider, Betriebsteams | `reference/kompatibilitaet.md`, `persistence/gotchas.md` | P0 |

**Abnahmekriterium `grenzen-und-trade-offs.md`:** Jede Grenze nennt die Nutzerkonsequenz und die Alternative (z. B. „Call Activity statt Embedded Sub-Process, wenn Runtime-Scoping gebraucht wird" — belegt in `bpmn-parser/gotchas.md`).

### 3.5 `docs/knowledge-base/` (bleibt, wird umetikettiert)

| Pfad | Aktion | Zielgruppe | Prio |
|---|---|---|---|
| `knowledge-base/index.md` | Titel und Audience ändern: „Knowledge Base für Beitragende und Maintainer"; Verweis auf `README.md` als Nutzer-Einstieg. | Beitragende | P0 |
| `knowledge-base/**` (alle Modul- und Architekturdateien) | Inhaltlich übernehmen; nur die in der Migrationsmatrix (Abschnitt 6) genannten Korrekturen anwenden. | Beitragende | P1 |

---

## 4. Navigationskonzept

### 4.1 Ebenen

1. **Landingpage** (`README.md`): Zielgruppen-Einstiege, „Was möchten Sie tun?", Statushinweis, Link auf Referenzlandingpage und Knowledge Base.
2. **Quadranten-Landingpages**: `reference/index.md` (Pflicht, P0). `tutorials/`, `how-to/` und `explanation/` erhalten je eine `index.md` mit Tabellen der Inhalte (P1).
3. **Detailseiten**: je eine Aufgabe oder ein Vertrag.
4. **Querverweise**: jeder How-to verlinkt auf die zugehörige Referenz; jede Referenzseite verlinkt auf die erklärende Seite; jede Explanation verlinkt auf die betroffene Referenz.

### 4.2 „Was möchten Sie tun?"-Einstiege (auf der Landingpage)

| Frage des Lesers | Ziel |
|---|---|
| Ich lerne BPMNinja zum ersten Mal. | `tutorials/erster-prozess.md` |
| Ich will einen Prozess deployen. | `how-to/prozess-deployen.md` |
| Ich will einen Worker bauen. | `how-to/external-task-worker.md` |
| Ich will BPMNinja produktiv betreiben. | `how-to/betrieb/produktion-checkliste.md` |
| Ich muss einen Ausfall beheben. | `how-to/betrieb/nats-ausfall.md` |
| Ich muss einen Incident lösen. | `how-to/betrieb/incident-triage.md` |
|
