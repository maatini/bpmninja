# Domänenrecherche BPMNinja — Forschungsbericht

**Datei:** `_research/domain-research.md`
**Autor:** Documentation Lead / Redaktionsmanagement
**Stand:** 2026-07-17 (Recherchestand)
**Status:** Abgeschlossen, mit markierten Lücken
**Zweck:** Grundlage für Informationsarchitektur, Terminologie und Redaktionsplan der BPMNinja-Dokumentation nach Diátaxis. Kein Dokument wird geschrieben oder umgebaut, bevor dieser Bericht abgenommen ist.

---

## 1. Management Summary — die fünf wichtigsten Erkenntnisse

1. **Die Domäne ist mehrdeutig, die Dokumentation muss eindeutig sein.** BPMN 2.0 ist ein OMG-Standard mit formaler Semantik (OMG BPMN 2.0.2, formal/13-12-09, Januar 2014). BPMNinja weicht an mehreren dokumentierten Stellen bewusst davon ab (Sub-Process-Flattening, Rhai statt JavaScript/Groovy, kein Multi-Instance, Task-Typ-Mapping). Diese Abweichungen sind der **wichtigste Dokumentationsgegenstand** des Projekts — sie sind gleichzeitig die häufigste Quelle falscher Nutzerannahmen. Jede Abweichung braucht einen eigenen, auffindbaren Referenz- und Erklärungsartikel.

2. **Vier Zielgruppen mit vier verschiedenen Einstiegspunkten.** Einsteiger/Lernende brauchen Tutorials (Lernen durch Tun), Entwickler/Integratoren brauchen Referenz und How-to, Betriebsteams brauchen Runbooks und Fehlerbehebung, Stakeholder brauchen Erklärung und Entscheidungsgrundlagen. Diátaxis liefert dafür die saubere Trennung in Tutorial / How-to / Reference / Explanation (Diátaxis, diataxis.fr). Eine einzige „Doku" für alle vier Gruppen ist der größte strukturelle Fehler, den wir vermeiden müssen.

3. **Der External-Task-Worker-Vertrag ist der riskanteste Integrationspunkt.** Fetch-and-Lock, Lock-Dauer, Retry-Semantik, Incident-Erzeugung und BPMN-Error-Routing sind in BPMNinja Camunda-kompatibel modelliert, aber mit eigenen Details (Millisekunden an der HTTP-Grenze, `retries: 0` erzeugt Incident, `bpmnError()` wird nicht wiederholt). Camunda dokumentiert dasselbe Muster in `camunda-external-task-client-js` (GitHub, Apache-2.0) und in der Camunda-8-REST-API (`POST /jobs/activation`). Wir müssen den Vertrag **explizit als Vertrag** dokumentieren — nicht als Sammlung von Endpunkt-Tabellen.

4. **Betriebsrisiken sind dokumentiert, aber nicht als Betriebsdokumentation.** Die Knowledge Base enthält exzellente Gotchas (fail-closed `REQUIRE_NATS`, bounded Retry-Queue mit Drops, SSE-Broadcast ohne Catch-up, NATS-JetStream-Retention, Startup-Restore-Dauer, Auth fail-closed bei Nicht-Loopback-Bind). Diese Inhalte sind heute **entwicklerzentriert** formuliert („⚠️ Never hold a lock across `.await`"). Für Betriebsteams fehlen daraus abgeleitete Runbooks, Alarmierungsempfehlungen und ein Readiness-/Liveness-Modell in Betriebssprache.

5. **Die bestehende Knowledge Base ist eine hervorragende interne Quelle, aber keine Nutzerdokumentation.** Sie ist explizit für „coding agents (AI and human)" geschrieben (`docs/knowledge-base/index.md`) und enthält harte, belegbare Fakten (Ports, Env-Variablen, Statuscodes, Bucket-Namen, Grenzwerte). Sie ist die Primärquelle für Referenz und Erklärung — muss aber in Nutzersprache und Diátaxis-Struktur überführt werden. Die Qualitätsregel der KB („Ground in code: Don't invent") übernehmen wir als Redaktionsprinzip.

---

## 2. Domänenkarte und Begriffsfeld

### 2.1 Zentrale Konzepte

| Konzept | Definition (domänenweit) | BPMNinja-spezifische Ausprägung | Belegquelle |
|---|---|---|---|
| **Prozessdefinition** | Modell eines Geschäftsprozesses, in BPMN 2.0 als XML (`<definitions>` mit `<process>`) | `ProcessDefinition`, nach Deployment immutabel (`Arc<ProcessDefinition>`); **nur ein `<process>` pro Datei** wird deployt (erstes mit `isExecutable="true"`, sonst das erste) | `README.md`; `docs/knowledge-base/modules/bpmn-parser/gotchas.md`; OMG BPMN 2.0.2 |
| **Prozessinstanz** | Eine konkrete Ausführung einer Definition | `ProcessInstance` mit `id`, `definition_key`, `business_key`, `state`, `variables`, `tokens`, `join_barriers`, `incidents` | `docs/architecture.md` §3.1 |
| **Token** | Ausführungsmarke, die genau eine aktive Position im Prozessgraph repräsentiert | `Token { id, current_node, variables, is_merged }`; **lebt an genau einem Ort** (`instance.tokens`), Pending-Tasks halten nur `token_id` | `docs/architecture.md` §4.1; `engine-core/gotchas.md` |
| **Wait State** | Zustand, in dem eine Instanz auf ein externes Ereignis wartet (User-Task, Service-Task, Timer, Message) | Vier `DashMap`-Queues: `pending_user_tasks`, `pending_service_tasks`, `pending_timers`, `pending_message_catches` | `docs/architecture.md` §2.2 |
| **Gateway** | Kontrollfluss-Verzweigung/-Zusammenführung (XOR, AND, OR, Event-Based, Complex) | XOR: erste wahre Bedingung, dann unbedingter Nicht-Default-Flow, dann Default. AND: JoinBarrier. OR: Taken-Path-Barrier mit `expected_count` = tatsächlich genommene Zweige; unstrukturierte Joins fallen auf `incoming_flow_count` zurück und **können hängen** | `README.md`; `engine-core/gotchas.md` |
| **External Task / Service Task** | Arbeit, die von einem externen Worker abgeholt und abgeschlossen wird (Fetch-and-Lock) | Topic-basiert; Topic-Auflösung: `data-topic` → `data-handler` → `camunda:topic` → Node-ID | `README.md`; `bpmn-parser/gotchas.md` |
| **Incident** | Fehlerzustand, der manuellen Eingriff erfordert | Teil der `ProcessInstance` (`ProcessInstance.incidents`), **keine eigene Persistence-Methode**; Dedup-Key `(kind, node_id, task_id)`; max. 100 pro Instanz | `engine-core/gotchas.md`; `engine-server/gotchas.md` |
| **Business Key** | Fachlicher Schlüssel zur Korrelation einer Instanz mit einem Geschäftsvorgang | Top-Level-Feld auf `StartRequest`/`StartLatestRequest`, hat Vorrang vor `business_key` in `variables` | `engine-server/gotchas.md` |
| **Message Correlation** | Zuordnung einer eingehenden Nachricht zu wartenden Message-Catch-Events | `messageName` + optional `businessKey`; `POST /api/message` | `README.md` |
| **Timer** | Zeitgesteuerte Auslösung | ISO-8601-Duration, AbsoluteDate, Cron, Repeating Interval; Monate/Jahre auf 30/365 Tage approximiert (**nicht** ISO-8601-konform für kalenderabhängige Dauern) | `README.md`; `bpmn-parser/gotchas.md` |
| **Call Activity** | Aufruf einer anderen Prozessdefinition | `calledElement`; Variablenpropagation in beide Richtungen; Tracking über `outstanding_calls`; Spawn-Fehler → `CompletedWithError` | `README.md`; `engine-core/gotchas.md` |
| **Sub-Process-Flattening** | Auflösen eingebetteter Sub-Prozesse zur Parse-Zeit | Kein Runtime-Scoping; synthetische `SubProcessEndEvent`-Knoten; Boundary-Events auf Sub-Prozessen funktionieren **nicht** wie im Standard | `bpmn-parser/gotchas.md`; ADR-003 |
| **Persistence-Port** | Abstraktion des Speichers | `WorkflowPersistence`-Trait (30+ Methoden); Adapter: NATS JetStream, In-Memory | ADR-001; `docs/architecture.md` §5.1 |
| **Readiness vs. Liveness** | Betriebszustand des Dienstes | `/api/health` = Liveness (immer 200); `/api/ready` = Durability-Readiness (503 bei fehlender/verlorener Persistenz) | `README.md`; `overview.md` |

### 2.2 Akteure

| Akteur | Rolle | Primärbedarf (Diátaxis) |
|---|---|---|
| **Prozessmodellierer / Einsteiger** | Erstellt BPMN-Diagramme, will sie laufen sehen | Tutorial, Erklärung |
| **Integrator / Worker-Entwickler** | Schreibt External-Task-Worker (TypeScript, Rust, andere) | How-to, Referenz (API-Vertrag) |
| **Plattform-Administrator** | Betrieb, Deployment, Skalierung, Backup | How-to, Runbook, Referenz (Env, Endpunkte) |
| **Betriebsteam / SRE** | Monitoring, Incident Response, Kapazität | Runbook, Fehlerbehebung, Erklärung (Retry/Drop-Semantik) |
| **Stakeholder / Entscheider** | Bewertet Eignung, Risiken, Abweichungen | Erklärung, Entscheidungsgrundlagen |
| **Beitragende Entwickler** | Ändern Engine-Code | Interne KB (bestehend), Architektur-Erklärung |

### 2.3 Datenflüsse (Kurzform)

1. **Deploy:** BPMN-XML → `POST /api/deploy` → `bpmn-parser` → `ProcessDefinition` → Persistenz (`definitions`-KV + `bpmn_xml`-KV). Bei Persistenz-Schreibfehler: Rollback der In-Memory-Definition, HTTP 500.
2. **Start:** `POST /api/start` (per `definition_key`) oder `/api/start/latest` (per `bpmn_process_id`) → Instanz + initialer Token → Execution-Loop.
3. **Ausführung:** `run_instance_batch` → `execute_step` → `NextAction` (Continue / ContinueMultiple / WaitForJoin / WaitForUser / WaitForServiceTask / WaitForTimer / WaitForCallActivity / Complete).
4. **Externe Arbeit:** Worker → `POST /api/service-task/fetchAndLock` (Long Polling) → `complete` / `failure` / `bpmnError` / `extendLock`.
5. **Ereignisse:** Engine → `tokio::sync::broadcast` (Kapazität 256) → SSE `GET /api/events` → Desktop-UI (Refetch via REST).
6. **Persistenz:** Alle zustandsbehafteten Schreibvorgänge → Inline-Retry (2×, ~50 ms) → bounded Background-Queue (Default 10 000, exponentielles Backoff, max. 50 Versuche) → bei Voll: **Drop + Metrik** `bpmn_persistence_retry_dropped_total`.
7. **Abschluss:** Instanz → `Completed` → Archivierung in `history_completed_instances`, Löschung aus aktivem `instances`-Bucket.

### 2.4 Lebenszyklen

- **Instanz:** `Running` → (`Suspended { previous_state }` ⇄ `Running`) → `Completed` | `CompletedWithError`. Suspend blockiert Timer und Task-Abschluss, **nicht** Variablen-Updates und Token-Move.
- **Token:** lokal im Execution-Loop → zentraler Store bei Wait State → zurück in den Loop bei Resume → `Merged` bei Join → Ende bei End-Event.
- **Service-Task:** pending → locked (mit `workerId`, `lockExpiration`, `retries`) → completed | failed (retries dekrementiert) → Incident (`retries <= 0`) → retry | resolve.
- **Definition:** deployt (immutabel) → referenziert von Instanzen → löschbar (`?cascade=true` löscht Instanzen mit) → Versionierung über `bpmn_process_id` + `definition_key`.

### 2.5 Typische Architekturentscheidungen (aus den ADRs)

| ADR | Entscheidung | Dokumentationsrelevanz |
|---|---|---|
| ADR-001 | Hexagonale Architektur (Ports & Adapters) | Erklärung „Warum ist die Engine I/O-frei?" |
| ADR-002 | Token-basierte Ausführung | Kern-Erklärung; Voraussetzung für Gateway-Verständnis |
| ADR-003 | Sub-Process-Flattening | **Abweichung** — muss prominent dokumentiert werden |
| ADR-004 | Lock-free Concurrency via DashMap | Erklärung für Beitragende; Betriebsrelevanz (Skalierung) |
| ADR-005 | Camunda-kompatibles Service-Task-API | Integrationsvertrag |
| ADR-006 | SSE-Push statt Polling | UI-Verhalten, „verpasste Events" erklären |
| ADR-007 | Zweistufiger Persistenz-Retry | Betriebsrisiko, Metriken |
| ADR-008 | Rhai statt JavaScript/Groovy | **Abweichung** — Migration von Camunda |
| ADR-009 | Fail-closed Durability (`REQUIRE_NATS`) | Betrieb, Deployment, Readiness |
| ADR-010 | Bounded Retry-Queue | Betrieb, Monitoring, Datenverlustrisiko |

### 2.6 Begriffe, die regelmäßig verwechselt werden

| Verwechslung | Klärung | Beleg |
|---|---|---|
| **Prozessdefinition vs. Prozessinstanz** | Definition = Modell (immutabel), Instanz = Lauf (mutabel) | `docs/architecture.md` |
| **`definition_key` vs. `bpmn_process_id`** | `definition_key` = UUID einer deployten Version; `bpmn_process_id` = fachliche Prozess-ID über Versionen hinweg (`/api/start/latest`) | `README.md` REST-Tabelle |
| **User Task vs. Service Task** | User Task = menschliche Bearbeitung (`/api/complete/:id`); Service Task = externe Worker (Fetch-and-Lock) | `README.md` |
| **Service Task vs. External Task** | In BPMN 2.0 ist „Service Task" ein Notationselement; „External Task" ist das Ausführungsmuster (Camunda). BPMNinja nutzt Service Tasks **als** External Tasks | OMG BPMN 2.0.2; Camunda-Client-README |
| **Failure vs. BPMN Error** | Failure = technischer Fehler (Retry, Incident); BPMN Error = fachliche Ausnahme (sofortiges Boundary-Routing, **kein** Retry) | `external-task-client/gotchas.md` |
| **Incident vs. Failure** | Failure ist ein Ereignis; Incident ist der persistierte, sichtbare Fehlerzustand der Instanz | `engine-core/gotchas.md` |
| **Suspend vs. Terminate** | Suspend pausiert (fortsetzbar); Terminate-End-Event bricht alle Tokens ab (Endzustand) | `README.md` |
| **Liveness vs. Readiness** | `/api/health` (Prozess lebt) vs. `/api/ready` (Persistenz verfügbar) | `overview.md` |
| **In-Memory-Persistenz vs. „kein Speicher"** | In-Memory ist ein vollwertiger Adapter, aber flüchtig; erreichbar nur mit `REQUIRE_NATS=false` | ADR-009 |
| **Message Catch vs. Message Start** | Catch wartet in laufender Instanz; Start erzeugt neue Instanz | `README.md` |
| **Timer-Duration vs. Cron** | Duration = relativ; Cron = kalenderbasiert; Monate/Jahre approximiert | `bpmn-parser/gotchas.md` |
| **Flattening vs. Call Activity** | Flattening = gleicher Scope, kein Runtime-Nesting; Call Activity = echte Kind-Instanz | `bpmn-parser/gotchas.md` |
| **SSE-Event vs. Zustand** | SSE ist ein Signal („etwas hat sich geändert"), kein Datenkanal; Client muss per REST nachladen | `engine-server/gotchas.md` |
| **`lockDuration` (ms) vs. Sekunden** | HTTP-Grenze ist Millisekunden; Engine rechnet intern in Sekunden um | `engine-server/gotchas.md`; `external-task-client/gotchas.md` |
| **History-KV vs. `WORKFLOW_EVENTS`-Stream** | History-Einträge liegen als KV-Keys vor; der Stream `WORKFLOW_EVENTS` ist ein separater Event-Strom (Widerspruch in den Quellen, siehe §3.3) | `persistence/gotchas.md` vs. `README.md`/`overview.md` |

---

## 3. Standards und Best Practices mit Quellen

### 3.1 Dokumentations-Frameworks

**Diátaxis** (diataxis.fr, Zugriff 2026-07-17)
- Selbstbeschreibung: „A systematic approach to technical documentation authoring … prescribes approaches to content, architecture and form that emerge from a systematic approach to understanding the needs of documentation users."
- Vier Nutzerbedürfnisse, vier Dokumentationsformen: **Tutorials**, **How-to Guides**, **Technical Reference**, **Explanation** — „It places them in a systematic relationship, and proposes that documentation should itself be organised around the structures of those needs."
- Diátaxis „solves problems related to documentation content (what to write), style (how to write it) and architecture (how to organise it)" und „doesn't impose implementation constraints".
- Praxisbelege auf der Startseite: Vonage („high-quality set of internal documentation that our users love"), Gatsby („the four quadrants helped us prioritize the user's goal for each type of documentation"), Cloudflare („Diátaxis became our north star for information architecture").
- **Übertragung auf BPMNinja:** Die bestehende KB ist überwiegend *Reference* (Interfaces, Gotchas) und *Explanation* (ADRs, Architektur). Es fehlen Tutorials und How-to Guides fast vollständig. Der README ist eine Mischform (Quick Start + Referenz + Feature-Liste) und muss in Diátaxis-Quadranten zerlegt werden.

**Diátaxis-Kompass** (diataxis.fr/compass/, Zugriff 2026-07-17)
- Reduziert die Einordnung auf zwei Fragen: *action or cognition?* und *acquisition or application?*
- Entscheidungstabelle (wörtlich): „informs action + acquisition of skill → a tutorial"; „informs action + application of skill → a how-to guide"; „informs cognition + application of skill → reference"; „informs cognition + acquisition of skill → explanation".
- „The compass can be applied equally to user situations that need documentation, or to documentation itself that perhaps needs to be moved or improved."
- **Übertragung:** Der Kompass ist unser Ablagewerkzeug für jeden neuen Artikel und für die Sanierung des README.

**Google Developer Documentation Style Guide** (developers.google.com/style, Zugriff 2026-07-17)
- Referenzhierarchie (wörtlich): „Project-specific style … This style guide … Third-party references" — für Rechtschreibung Merriam-Webster, für nichttechnischen Stil Chicago Manual of Style (17. Aufl.), für technischen Stil der Microsoft Writing Style Guide.
- Grundhaltung: „This guide contains guidelines, not rules. Depart from it when doing so improves your content." — „When you depart from this guide, be consistent throughout your document."
- **Übertragung:** Wir definieren einen projektspezifischen Stil (Terminologie-Glossar, siehe §6) und nutzen Google als Fallback für Formulierungsfragen.

**Microsoft Writing Style Guide** (learn.microsoft.com/en-us/style-guide/welcome/, Zugriff 2026-07-17)
- Leitprinzip „Make every word matter"; Tonalität „warm and relaxed, crisp and clear, and ready to lend a hand".
- Enthält Kapitel zu bias-freier Kommunikation und globaler Kommunikation.
- **Übertragung:** Relevant für die Einsteiger- und Stakeholder-Inhalte (Ton), weniger für Referenz (Dichte).

**Write the Docs — Documentation Principles** (writethedocs.org/guide/writing/docs-principles/, Zugriff 2026-07-17)
- Allgemeine Prinzipien: **Precursory** („Begin documenting before you begin developing"), **Participatory**.
- Inhaltsprinzipien: **ARID** („Accept (some) Repetition In Documentation"), **Skimmable**, **Exemplary**, **Consistent**, **Current** — mit der klaren Aussage: „Consider incorrect documentation to be worse than missing documentation."
- Quellenprinzipien: **Nearby** (nah am Code), **Unique** („Eliminate content overlap between separate sources … to prevent any parallel maintenance (or worse — lack of maintenance) of the same information across multiple sources").
- Publikationsprinzipien: **Discoverable**, **Addressable** (Deep-Links auf Abschnittsebene), **Cumulative**, **Complete**, **Beautiful**.
- **Übertragung:** Direkt anwendbar. „Unique" begründet, warum KB und Nutzerdoku nicht denselben Inhalt parallel pflegen dürfen. „Addressable" begründet Deep-Links pro Endpunkt/Env-Variable. „Complete" begründet, warum wir die Abweichungsliste vollständig führen müssen (nicht 50 von 100 Hydranten).

**Write the Docs — Docs as Code** (writethedocs.org/guide/docs-as-code/, Zugriff 2026-07-17)
- Dokumentation mit denselben Werkzeugen wie Code: Issue Tracker, Git, Plain-Text-Markup, Code Review, automatisierte Tests.
- „You can block merging of new features if they don't include documentation, which incentivizes developers to write about features while they are fresh."
- **Übertragung:** Redaktionsworkflow als PR-Prozess; Doku-Änderungen laufen durch Review wie Code.

**Write the Docs — Accessibility** (writethedocs.org/guide/writing/accessibility/, Zugriff 2026-07-17)
- „… if the content design is simple, concise, and clear you are already doing a lot for both a11y and localization."
- Verweist auf MDN, WebAIM, IBM Equal Access Toolkit, Microsoft Style Guide Accessibility Terms.
- **Übertragung:** Einfache Sprache ist gleichzeitig Barrierefreiheits- und Lokalisierungsmaßnahme.

### 3.2 Normen für Informationsmanagement und Dokumentation

| Norm | Gegenstand | Status der Prüfung | Verwendung im Projekt |
|---|---|---|---|
| **ISO/IEC/IEEE 26511** | Requirements for managers of information for users | **Primärquelle nicht abrufbar.** Abrufversuche: `iso.org/standard/77461.html` → HTTP 403; `standards.ieee.org/ieee/26511/6842/` → liefert eine **falsche** Seite (IEEE C62.69a-2017, Überspannungsschutz); Wikipedia-Artikel `ISO/IEC_26511` und `ISO/IEC_IEEE_26511` → HTTP 404; Websuche lieferte nur irrelevante Treffer (DeepL, IEEE-Startseite). **Nicht inhaltlich zitiert.** | Redaktionsmanagement, Rollen, Freigabeprozess — als Orientierungsrahmen benannt, nicht als Zitat |
| **ISO/IEC/IEEE 26514** | Design and development of information for users | **Primärquelle nicht abrufbar.** `standards.ieee.org/ieee/26514/6843/` → liefert falsche Seite (IEEE 1704-2021, Zählerkommunikation); Wikipedia-Artikel `ISO/IEC_26514` → HTTP 404. **Nicht inhaltlich zitiert.** | Struktur- und Entwicklungsprozess der Doku — als Orientierung |
| **ISO/IEC/IEEE 26515** | Developing information for users in an agile environment | **Nicht geprüft** (kein erfolgreicher Abruf; Wikipedia-Artikel `ISO/IEC_26515` → HTTP 404). | Passt zum iterativen Redaktionsmodell; **offene Lücke** |
| **ISO/IEC 40500:2012 / WCAG 2.0** | Web Content Accessibility Guidelines als ISO-Standard | Belegt über Wikipedia „Web Content Accessibility Guidelines" (Zugriff 2026-07-17): „WCAG 2.0 was published in December 2008 and became an ISO standard, ISO/IEC 40500:2012 in October 2012"; Level A/AA wurden „in early 2014 … incorporated as references in clause 9.2 ('Web content requirements') of the European standard EN 301 549". | Barrierefreiheit der Doku-Website und der Desktop-UI-Doku |
| **WCAG 2.2** | Aktuelle W3C-Empfehlung | Direkter Abruf von `w3.org/TR/WCAG22/` **fehlgeschlagen (HTTP 403)**; belegt über Wikipedia-Sekundärquelle: „WCAG 2.2 became a W3C Recommendation on 5 October 2023", „Nine new criteria make their definitive debut in this new version". | Zielniveau AA für Doku-Website; **Primärzitat nachzuholen** |

### 3.3 Domänenspezifische Standards

**OMG BPMN 2.0.2** (omg.org/spec/BPMN/2.0.2/, Zugriff 2026-07-17)
- Formale Version: **2.0.2, Januar 2014**, Dokument `formal/13-12-09`; Status „formal".
- Selbstbeschreibung: „Business Process Model and Notation has become the de-facto standard for business processes diagrams. It is intended to be used directly by the stakeholders who design, manage and realize business processes, but at the same time be precise enough to allow BPMN diagrams to be translated into software process components."
- Normative maschinenlesbare Dokumente: `BPMN20.xsd`, `Semantic.xsd`, `BPMNDI.xsd`, `DC.xsd`, `DI.xsd` (alle `dtc/10-05-04`), CMOF-Modelle (`BPMN20.cmof`, `BPMNDI.cmof`, `DC.cmof`, `DI.cmof`), `Infrastructure.cmof` (`dtc/10-05-15`).
- Informative Dokumente: „BPMN 2.0 examples non-normative document" (`dtc/10-06-02`), Beispiel-Archive (`dtc/10-06-03`).
- **Übertragung:** Jede BPMNinja-Abweichung muss gegen die normative Semantik benannt werden (nicht gegen „BPMN allgemein"). Die XSDs sind die Referenz für Parser-Verhalten. Die Trennung normativ/informativ übernehmen wir als Vorbild für „unser Vertrag" vs. „Hintergrund".

**Camunda External Task Pattern** (github.com/camunda/camunda-external-task-client-js, Zugriff 2026-07-17; docs.camunda.io Job Workers, Zugriff 2026-07-17; docs.camunda.io Activate Jobs, Zugriff 2026-07-17)
- Client-README: „External Tasks are service tasks whose execution differs particularly from the execution of other service tasks … units of work are polled from the engine before being completed." Lizenz Apache-2.0, ESM, Node ≥ 18.
- Client-Features (README): Fetch and Lock (Polling), Complete, Handle Failure (mit `errorMessage`, `errorDetails`, `retries`, `retryTimeout`), Handle BPMN Error, Extend Lock, Unlock, Lock, Prozess-/Lokale Variablen, `BasicAuthInterceptor`, `KeycloakAuthInterceptor`, `logger`.
- Camunda-8-Konzept „Job Workers": Job-Typen sind case-sensitive; „Timeout: The time a job is assigned to the worker. If a job is not completed within this time, it can be reassigned by Zeebe to another worker."; Long Polling via `request-timeout`; „If remaining retries is greater than zero, the job is retried and reassigned. If remaining retries is zero or negative, an incident is raised and the job is not retried until the incident is resolved."; Retry-Backoff möglich; „Zeebe is an 'at least once' system with respect to job delivery and that worker code must be idempotent."
- Camunda-8-REST `POST /jobs/activation`: Permission `PROCESS_DEFINITION`/`UPDATE_PROCESS_INSTANCE`; HTTP 503 bei Backpressure mit `RESOURCE_EXHAUSTED` im Titel, „Clients are recommended to eventually retry those requests after a backoff period."
- **Übertragung:** BPMNinja spiegelt dieses Muster (ADR-005). Die Doku muss die **Gemeinsamkeiten** (Vertrag) und die **Unterschiede** (Endpunktpfade `/api/service-task/*`, Millisekunden, `retries: 0` → Incident, globaler Retry-Backoff pro Client) klar trennen. Die Idempotenz-Anforderung aus Camunda gilt auch für BPMNinja-Worker und muss in den Integrator-How-tos stehen.

**NATS JetStream** (docs.nats.io/concepts/jetstream, Zugriff 2026-07-17; projektspezifisch `persistence/gotchas.md`)
- „Core NATS delivers messages only to subscribers connected at the moment of publication - at most once, never replayed. JetStream adds a persistence layer on top, giving you at-least-once delivery - messages survive restarts and can be replayed."
- Drei Bausteine: **Stream** (serverseitiger Speicher, an Subjects gebunden), **Consumer** (serverseitiger, zustandsbehafteter Cursor), **Client** (acknowledgt; „if a message isn't acknowledged in time, the server redelivers it, which is what gives you at-least-once delivery").
- Streams sind konfigurierbar für „storage (memory or disk), retention (whether limits or consumer acks remove messages), replication".
- Projektspezifisch: JetStream muss mit `--js` aktiviert sein; Streams `WORKFLOW_EVENTS` (7 Tage, 1 GiB, 1 Replica) und `WORKFLOW_HISTORY` (30 Tage, 1 GiB, 1 Replica), konfigurierbar über `BPMNINJA_EVENT_RETENTION_DAYS`, `BPMNINJA_HISTORY_RETENTION_DAYS`, `BPMNINJA_STREAM_MAX_BYTES`, `BPMNINJA_STREAM_REPLICAS`.
- **Widerspruch, ausdrücklich benannt:** Die KB bezeichnet History als „KV bucket (not a stream, despite the `WORKFLOW_EVENTS` stream name)" (`persistence/gotchas.md`), während `README.md` und `overview.md` von „event streaming for history" sprechen. Für die Nutzerdoku gilt bis zur Klärung: History-Einträge liegen als KV-Keys vor; der Stream `WORKFLOW_EVENTS` ist ein separater Event-Strom. Diese Unterscheidung muss vor dem Schreiben eines Persistenz-Kapitels mit dem Engineering geklärt werden (siehe §8).

### 3.4 Sicherheitsanforderungen (projektspezifisch belegt)

| Anforderung | Beleg | Doku-Konsequenz |
|---|---|---|
| API-Auth fail-closed bei Nicht-Loopback-Bind | `engine-server/gotchas.md`: ohne Key und mit `BIND_ADDR` ≠ Loopback → 401 auf allen `/api`-Routen außer `/api/health`, `/api/ready` | Sicherheits-How-to „Deployment absichern" mit explizitem Dev-Opt-out `BPMNINJA_ALLOW_UNAUTHENTICATED=true` |
| Rollentrennung Admin/Worker | Admin-Key darf alles; Worker-Key nur `fetchAndLock`, `complete`, `failure`, `extendLock`, `bpmnError`; alles andere 403 (inkl. `retry`/`resolve`, `/metrics`, Incident-Routen) | Referenz der Rollenmatrix; Warnung in Integrator-How-to |
| CORS-Allowlist statt `Any` | `CORS_ORIGINS` Default-Liste (`http://localhost:1420`, `http://127.0.0.1:1420`, `tauri://localhost`, `https://tauri.localhost`); `*` nur explizit | Deployment-Referenz |
| Upload-/Body-Limits | `MAX_XML_BYTES` 10 MiB (413), `MAX_UPLOAD_BYTES` 5 MiB (413) | Fehlerbehebungs-Tabelle |
| Rhai-Sandbox | `RHAI_MAX_OPERATIONS` 50 000, `RHAI_MAX_MEMORY_BYTES` 2 MiB, `RHAI_TIMEOUT_MS` 1000 | Erklärung „Scripting-Grenzen" |
| Keine Secrets in Logs | Auth-Fehlversuche werden mit Methode + Pfad geloggt, „keine Secrets"; Zähler `bpmn_auth_failures_total{reason}` | Betriebs-How-to Logging |
| NATS-Zugangsdaten | docker-compose nutzt User/Pass `bpmninja`/`bpmninja`, „changeable"; NATS-Ports nur an Loopback gebunden | Sicherheitswarnung im Deployment-How-to |
| Log-Datei wächst unbegrenzt | `engine_logs.jsonl` „is NOT truncated — it grows indefinitely (mitigated by `LOG_FILE=off` or external rotation)" | Betriebs-How-to Log-Rotation |

### 3.5 Barrierefreiheit

- **Doku-Website:** Ziel WCAG 2.2 Level AA (belegt über Wikipedia; Primärzitat offen). Konkret: semantische Überschriftenhierarchie, Alternativtexte für alle Mermaid-Diagramme (die KB nutzt Mermaid intensiv — jedes Diagramm braucht eine Textbeschreibung), ausreichende Kontraste, Tastaturnavigation.
- **Desktop-UI-Doku:** Die KB dokumentiert bereits Radix-UI-Anforderungen (`DialogDescription` für Screenreader, teils `sr-only`) — `desktop-tauri/gotchas.md`. Diese Anforderung gehört in die Beitragenden-Doku, nicht nur in die Gotchas.
- **Sprache:** Einfache Sprache für Einsteiger (Microsoft-Prinzip „
