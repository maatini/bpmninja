# Forschungsbericht: Domäne „BPMNinja“ — BPMN 2.0, Workflow-Engines, Token-Ausführung und External-Task-Integration

**Datei:** `_research/domain-research.md`
**Autor:** Documentation Lead / Redaktionsmanagement
**Stand:** Recherchephase vor Schreibbeginn (kein Dokument wurde geschrieben oder umgebaut)
**Repository:** `/Volumes/SSD2TB/work/bpmninja`
**Zugriffsdatum aller Web-Quellen:** 2026-10-24 (sofern nicht anders vermerkt)

---

## 1. Management Summary — die fünf wichtigsten Erkenntnisse

1. **Die Domäne ist zweigeteilt und muss dokumentarisch getrennt bleiben.** BPMNinja verbindet zwei sehr unterschiedliche Wissensgebiete: (a) *BPMN-2.0-Semantik* (Token, Gateways, Events, Scopes) und (b) *Betrieb einer verteilten Engine* (NATS JetStream, Retry-Queue, Readiness, Auth, Migration). Die vorhandene Knowledge Base ist stark auf (b) und auf Code-Gotchas fokussiert; die BPMN-Semantik für Einsteiger fehlt als eigene, lernorientierte Schicht. Diátaxis verlangt genau diese Trennung nach Nutzerbedürfnis, nicht nach Produktfeature ([Diátaxis](https://diataxis.fr/), [Diátaxis Map](https://diataxis.fr/map/)).

2. **Die größte dokumentarische Schuld liegt bei den bewussten Abweichungen vom Standard.** BPMNinja weicht an vielen Stellen von BPMN 2.0 und von Camunda ab: Sub-Process-Flattening, Rhai statt JavaScript/Groovy, kein Multi-Instance, kein DMN, `receiveTask`/`manualTask`/`businessRuleTask` werden umgemappt, nur ein `<process>` pro Datei, Inclusive-Join-Fallback. Diese Abweichungen sind im README verstreut dokumentiert, aber nicht als *eine* konsolidierte, versionierte „Compatibility & Deviations“-Referenz mit Begründung und Migrationshinweisen. Genau hier entstehen die teuersten Nutzerfehler.

3. **External-Task-Integration ist der wichtigste Integrationspfad und braucht eine eigene, Camunda-kontrastierende Dokumentationsachse.** Camunda dokumentiert das External-Task-Pattern als eigenes Konzept mit klarer Dreiteilung (Engine erzeugt Task → Worker fetch-and-lock → Worker complete/fail) und begründet es mit System-, Technologie- und Skalierungsgrenzen ([Camunda 7 External Tasks](https://docs.camunda.org/manual/7.24/user-guide/process-engine/external-tasks/)). BPMNinja übernimmt das Muster, aber mit eigenen Endpunktpfaden (`/api/service-task/*` statt `/external-task/*`), Millisekunden an der HTTP-Grenze und eigenen Retry-/Incident-Semantiken. Ohne expliziten „Was ist gleich, was ist anders“-Vergleich werden Camunda-Erfahrene falsche Annahmen treffen.

4. **Betriebsdokumentation ist sicherheits- und verfügbarkeitskritisch und derzeit nur als Gotcha-Sammlung vorhanden.** Fail-closed-Auth (`BPMNINJA_API_KEY`, `BPMNINJA_WORKER_API_KEY`, `BPMNINJA_ALLOW_UNAUTHENTICATED`), fail-closed-Durability (`REQUIRE_NATS`), begrenzte Retry-Queue mit Drop-Metrik, SSE mit Verlusten bei langsamen Consumern, Log-Buffer mit 5 000 Einträgen und unbegrenzt wachsende Log-Datei — das sind Betriebsrisiken, die eine eigene Runbook-/Operations-Sektion mit Abnahmekriterien brauchen, nicht nur `gotchas.md`-Warnungen.

5. **Die Zielgruppe ist viergeteilt und die vorhandene Dokumentation bedient faktisch nur zwei davon gut.** `docs/knowledge-base/index.md` adressiert ausdrücklich „coding agents (AI and human)“ — also Beitragende und Maintainer. Einsteiger/Lernende, Integratoren und Betriebsteams finden keinen geführten Einstieg, keine Aufgabenpfade und keine Entscheidungshilfen. Stakeholder/Entscheider finden keine belastbare „Was kann das, was kann es nicht, was kostet der Betrieb“-Seite. Die Informationsarchitektur muss entlang der vier Diátaxis-Quadranten *und* entlang der vier Rollen geschnitten werden.

---

## 2. Domänenkarte und Begriffsfeld

### 2.1 Zentrale Konzepte

| Konzept | Definition (belegt) | Relevanz für BPMNinja |
|---|---|---|
| **BPMN 2.0** | „Business Process Model and Notation“, OMG-Standard, Version 2.0.2 formal, Januar 2014; „de-facto standard for business processes diagrams“, „precise enough to allow BPMN diagrams to be translated into software process components“ ([OMG BPMN 2.0.2](https://www.omg.org/spec/BPMN/2.0.2/)) | Normative Referenz für alle Element-Aussagen; Abweichungen müssen gegen diese Quelle benannt werden |
| **Process Definition** | Das deployte, unveränderliche Prozessmodell (XML → geparste Struktur) | In BPMNinja `ProcessDefinition`, nach Deploy immutable (`Arc<ProcessDefinition>`), Quelle: `docs/knowledge-base/overview.md` |
| **Process Instance** | Eine konkrete Ausführung einer Definition mit eigenem Variablenzustand | In BPMNinja `ProcessInstance` mit `tokens`, `variables`, `state`, `incidents`; Quelle: `docs/architecture.md` §3.1 |
| **Token** | Ausführungsmarke, die genau eine Position im Graph repräsentiert; parallele Pfade = mehrere Tokens | BPMNinja-Kerndesign (ADR-002); Tokens leben zentral in `ProcessInstance.tokens`, Pending-Tasks halten nur `token_id`; Quelle: `docs/knowledge-base/architecture/decisions.md` |
| **Wait State** | Punkt, an dem die Engine die Ausführung anhält und Zustand persistiert, bis ein externes Ereignis eintritt | Flowable formuliert es generisch: Engine läuft bis zum Wait State, persistiert, wartet ([Flowable BPMN Intro](https://www.flowable.com/open-source/docs/bpmn/ch07a-BPMN-Introduction)); BPMNinja: UserTask, ServiceTask, Timer, Message, CallActivity |
| **Gateway (XOR/AND/OR/Event-based/Complex)** | Verzweigungs- und Zusammenführungsknoten mit unterschiedlicher Semantik | BPMNinja dokumentiert Join-Verhalten detailliert inkl. `JoinBarrier`, `expected_count`, `find_downstream_join`; Quelle: `docs/architecture.md` §4.3 |
| **Boundary Event** | Ereignis, das an eine Aktivität „geklebt“ ist und diese unterbrechen (interrupting) oder begleiten (non-interrupting) kann | BPMNinja: Timer/Message/Error/Escalation/Compensation; Quelle: `bpmninja/README.md` |
| **External Task / Service Task (external pattern)** | Arbeitseinheit, die von der Engine in eine Liste gestellt und von einem unabhängigen Worker per Polling abgeholt, gelockt und abgeschlossen wird | Camunda-Definition: „the process engine publishes a unit of work to a worker to fetch and complete“ ([Camunda 7 External Tasks](https://docs.camunda.org/manual/7.24/user-guide/process-engine/external-tasks/)) |
| **Topic** | Name, der die Art der Arbeit identifiziert; Worker abonnieren Topics | Camunda: Task erhält „a topic that identifies the nature of the work“ (ebd.); BPMNinja: Topic aus `data-topic` → `data-handler` → `camunda:topic` → Node-ID |
| **Fetch-and-Lock** | Worker holt Tasks und erhält eine zeitlich begrenzte Sperre, damit kein zweiter Worker denselben Task bearbeitet | Camunda: „timestamp-based lock … Only when the lock expires, another worker can fetch the task again“ (ebd.) |
| **Long Polling** | Request bleibt serverseitig offen, bis Arbeit verfügbar ist oder ein Timeout greift | Camunda 7 (ebd.) und Camunda 8 ([Job Workers](https://docs.camunda.io/docs/components/concepts/job-workers/)) beschreiben es identisch als Ressourcenoptimierung |
| **Incident** | Zustand, in dem eine Instanz an einem Punkt festsitzt und menschliches Eingreifen braucht | Camunda 8: „an incident represents a problem in process execution … requires user interaction“ ([Incidents](https://docs.camunda.io/docs/components/concepts/incidents/)); BPMNinja: `Incident` mit `kind` ∈ {service_task, timer, persistence, bpmn_error}, max. 100 pro Instanz |
| **BPMN Error vs. technischer Fehler** | BPMN Error = fachliche Ausnahme, die über Boundary Error Event gefangen wird; technischer Fehler = Retry/Incident | Camunda 7: `camunda:errorEventDefinition` mit Expression, fangbar durch Error Boundary Event (ebd.); BPMNinja: `bpmnError` wird nicht retried, `failure` mit `retries: 0` erzeugt Incident (`docs/knowledge-base/modules/external-task-client/gotchas.md`) |
| **Retry / Backoff** | Wiederholungsstrategie mit Wartezeiten | Camunda 8: „retry back off“ beim Fail ([Job Workers](https://docs.camunda.io/docs/components/concepts/job-workers/)); BPMNinja: inline 2× 50 ms + bounded Background-Queue, exponentiell bis 60 s, max. 50 Versuche |
| **Idempotenz** | Worker-Code muss mehrfach für denselben Task laufen können | Camunda 8: „Zeebe is an 'at least once' system … worker code must be idempotent“ ([Job Workers](https://docs.camunda.io/docs/components/concepts/job-workers/)) — **gilt analog für BPMNinja**, weil Lock-Ablauf zu Doppelbearbeitung führen kann |
| **Process Instance Migration** | Aktive Instanz auf eine andere Definitionsversion umziehen, mit Element-Mapping | Camunda 8: Mapping Source→Target, kein Typwechsel, Jobs werden nicht neu erzeugt ([Process instance migration](https://docs.camunda.io/docs/components/concepts/process-instance-migration/)); BPMNinja: `POST /api/instances/:id/migrate`, `OrphanedToken` bei fehlendem Mapping |
| **Process Instance Modification** | Reparatur einer Instanz ohne Definitionswechsel (Token verschieben, Aktivität abbrechen/neu starten) | Camunda 8 grenzt es explizit von Migration ab (ebd.); BPMNinja: `POST /api/instances/:id/move-token` |
| **Durability / Readiness** | Unterscheidung „Prozess lebt“ vs. „Prozess kann Zustand sicher halten“ | BPMNinja: `/api/health` (Liveness) vs. `/api/ready` (503 bei fehlender Persistenz); Quelle: `docs/knowledge-base/architecture/data-flows.md` §7 |
| **Ports & Adapters (Hexagonal)** | Domänenlogik definiert Ports, I/O-Adapter implementieren sie | BPMNinja ADR-001: `WorkflowPersistence` als Port, NATS/In-Memory als Adapter |

### 2.2 Akteure

| Akteur | Rolle | Primärbedürfnis (Diátaxis) |
|---|---|---|
| **Einsteiger / Lernende** | Will BPMN verstehen und einen ersten Prozess laufen sehen | Tutorial (lernen), Explanation (verstehen) |
| **Prozessmodellierer** | Erstellt BPMN-XML im Modeler oder in Camunda Modeler | How-to (Ziel erreichen), Reference (Elementverhalten) |
| **Integrator / Anwendungsentwickler** | Schreibt External-Task-Worker, ruft REST-API | How-to, Reference (API-Verträge) |
| **Plattform-Administrator / Betriebsteam** | Betrieb, Skalierung, Backup, Incident-Triage, Auth | How-to (Runbooks), Reference (Env-Variablen, Metriken) |
| **Stakeholder / Entscheider** | Bewertet Eignung, Risiko, Aufwand | Explanation (Architektur, Grenzen, Trade-offs) |
| **Beitragende / Maintainer** | Ändert Code, hält Doku aktuell | Reference (interne Invarianten), Contribution-How-to |
| **Externe Systeme** | Senden Messages, starten Timer-getriggerte Prozesse | Reference (Message-Correlation, Timer-Formate) |

### 2.3 Datenflüsse (aus dem Repository belegt)

Die Knowledge Base dokumentiert sieben Datenfluss-Sequenzen (`docs/knowledge-base/architecture/data-flows.md`):

1. **Deployment:** Client → `POST /api/deploy` → Parser → `deploy_definition` → `save_definition` + `save_bpmn_xml` → `DefinitionChanged`-Event.
2. **Instanz-Ausführung:** `POST /api/start` → Instanz + Token → `run_instance_batch` → pro Schritt `execute_step` → bei Wait State Token zentral speichern + Pending-Objekt persistieren.
3. **Service Task (Fetch-and-Lock):** Worker → `fetchAndLock` → Lock setzen → Worker arbeitet → `complete` / `failure` / `bpmnError`.
4. **Timer-Verarbeitung:** Background-Task alle `TIMER_INTERVAL_MS` (Default 1000 ms) → `process_timers()` → abgelaufene Timer → Instanz fortsetzen → ggf. nächsten Repeat-Termin berechnen.
5. **SSE-Push:** Engine → `tokio::sync::broadcast` (Kapazität 256) → SSE-Handler → UI → Re-Fetch via REST.
6. **Startup-Restore:** NATS-Connect → `StartupCoordinator::restore()` → Definitions (JSON-first) → Instances → User-Tasks → Service-Tasks → Timer → Message-Catches.
7. **Health vs. Readiness:** LB/K8s → `/api/health` (immer 200) vs. `/api/ready` (503 bei fehlender/fehlerhafter Persistenz).

Zusätzlich dokumentiert: **Persistence-Retry** (inline → bounded Queue → Background-Worker mit Re-Read des aktuellen Zustands).

### 2.4 Lebenszyklen

**Prozessinstanz** (aus `InstanceState` und Gotchas abgeleitet):
`Running` → (`WaitingOnUserTask` | `WaitingOnServiceTask` | `WaitingOnTimer` | `WaitingOnMessage` | `WaitingOnCallActivity` | `ParallelExecution`) → `Completed` → **archiviert** (aus aktivem Bucket entfernt, in `history_completed_instances` verschoben). Sonderpfade: `Suspended { previous_state }` (Timer feuern nicht, Tasks nicht abschließbar, Variablen und Token-Move erlaubt), `CompletedWithError` (z. B. `CALL_ACTIVITY_SPAWN_FAILED`).

**Service Task:** erzeugt → pending → `fetchAndLock` (locked, `workerId`, `lockExpiration`) → `complete` (Token weiter) | `failure` mit `retries > 0` (unlock, erneut fetchbar) | `failure` mit `retries = 0` (Incident) | `bpmnError` (Boundary-Pfad) | Lock-Ablauf (erneut fetchbar, **Doppelbearbeitung möglich**).

**Incident:** Anlage (dedupliziert über `(kind, node_id, task_id)`, `occurrences++`) → Auflösung (`resolved_at`, `resolved_by`) → bei erneutem Auftreten neuer/erneuter Eintrag. Limit 100 pro Instanz, Trim-Reihenfolge: zuerst aufgelöste, dann älteste offene.

**Definition:** deploy → Version → ggf. neue Version → Instanzen bleiben auf alter Version bis Migration → Löschen nur ohne laufende Instanzen (`DefinitionHasInstances`, 409) oder mit `?cascade=true`.

### 2.5 Typische Architekturentscheidungen in dieser Domäne

| Entscheidung | Optionen | BPMNinja-Wahl | Beleg |
|---|---|---|---|
| Ausführungsmodell | Token-basiert vs. AST-Interpreter vs. Event-Sourcing | Token-basiert, zentral gespeichert | ADR-002 |
| Persistenz-Backend | Relational vs. Event-Stream vs. KV | NATS JetStream KV + Object Store + Streams | `docs/architecture.md` §5 |
| Service-Task-Ausführung | synchron im Prozess vs. external task pattern | External Task Pattern (Camunda-kompatibel) | ADR-005 |
| Skriptsprache | JavaScript/Groovy vs. eingebettete Rust-Sprache | Rhai, sandboxed | ADR-008 |
| Sub-Prozess-Semantik | echte Scope-Verschachtelung vs. Flattening | Flattening zur Parse-Zeit | ADR-003 |
| Nebenläufigkeit | globaler Lock vs. feingranular vs. lock-free | DashMap + per-Instanz-RwLock | ADR-004 |
| UI-Aktualisierung | Polling vs. Push | SSE-Push, grobgranular | ADR-006 |
| Fehlertoleranz Persistenz | unbegrenzt vs. begrenzt | begrenzte Queue mit Drop + Metrik | ADR-010 |
| Durability-Default | silent in-memory fallback vs. fail-closed | fail-closed (`REQUIRE_NATS=true`) | ADR-009 |
| Auth-Default | offen vs. fail-closed bei Nicht-Loopback | fail-closed | `engine-server/gotchas.md` |

### 2.6 Begriffsfeld — regelmäßig verwechselte Begriffe

Diese Liste ist die Grundlage für ein Glossar und für „Nicht verwechseln“-Boxen. Jede Zeile ist eine konkrete Dokumentationsaufgabe.

| Verwechslung | Korrekte Abgrenzung | Beleg |
|---|---|---|
| **Process Definition ↔ Process Instance** | Definition = Modell (deployt, immutable); Instance = Lauf mit Variablen und Tokens | Flowable: „All the process instances for any month share the same process definition“ ([Flowable](https://www.flowable.com/open-source/docs/bpmn/ch07a-BPMN-Introduction)) |
| **Service Task ↔ External Task** | Service Task ist das BPMN-Element; External Task ist die Arbeitseinheit, die daraus im External-Task-Pattern entsteht | Camunda 7: Service Task kann intern *oder* extern ausgeführt werden (ebd.) |
| **BPMN Error ↔ technischer Fehler** | BPMN Error = fachliche Ausnahme, fangbar per Boundary Error Event, **nicht** retried; technischer Fehler = Retry/Incident | Camunda 7 (ebd.), BPMNinja `external-task-client/gotchas.md` |
| **Incident ↔ Fehler-Log** | Incident = blockierter Instanzzustand mit Auflösungspflicht; Log = Beobachtungsdaten | Camunda 8 Incidents (ebd.) |
| **Migration ↔ Modification** | Migration = Definitionswechsel mit Mapping; Modification = Reparatur innerhalb derselben Definition | Camunda 8 (ebd.) |
| **Liveness ↔ Readiness** | Liveness = Prozess antwortet; Readiness = Zustand kann sicher gehalten werden | BPMNinja `data-flows.md` §7 |
| **Suspend ↔ Terminate** | Suspend = pausierbar, Zustand bleibt; Terminate End Event = alle Tokens abbrechen, Instanz endet | BPMNinja README (TerminateEndEvent), `engine-core/gotchas.md` (Suspend) |
| **Interrupting ↔ Non-interrupting Boundary Event** | Interrupting bricht die Aktivität ab; non-interrupting startet Parallelpfad | BPMNinja README (Boundary-Tabellen) |
| **Embedded Sub-Process ↔ Call Activity** | Embedded = im selben Definitionsgraph (bei BPMNinja geflattet); Call Activity = Aufruf einer anderen Definition mit eigener Instanz | BPMNinja `bpmn-parser/gotchas.md`: „If you need runtime sub-process scoping, you're looking for Call Activities“ |
| **XOR-Join ↔ AND-Join ↔ OR-Join** | XOR führt zusammen ohne Synchronisation; AND wartet auf alle; OR wartet auf die tatsächlich genommenen Zweige | BPMNinja `engine-core/gotchas.md` (Inclusive-Join) |
| **`lockDuration` in ms ↔ s** | HTTP-Grenze = Millisekunden; engine-core intern = Sekunden (ceil) | `engine-server/gotchas.md`, `external-task-client/gotchas.md` |
| **`retries` am Task ↔ Incident-Status** | `retries <= 0` ist nur Fetch-and-Lock-Schutz; Incident-Wahrheit liegt in `ProcessInstance.incidents` | `engine-core/gotchas.md` |
| **SSE-Event ↔ Zustandsdaten** | SSE liefert nur grobgranulare Signale; Daten müssen per REST nachgeladen werden | ADR-006, `engine-server/gotchas.md` |
| **`REQUIRE_NATS=false` ↔ „läuft doch“** | In-Memory-Modus verliert bei Restart alles; `/api/ready` bleibt 503 | ADR-009, `persistence/gotchas.md` |
| **`camunda:topic` ↔ `data-topic`** | Parser liest `data-*` zuerst, dann Camunda-Attribute; Fallback Node-ID | `bpmn-parser/gotchas.md` |
| **ISO-8601-Dauer ↔ Kalenderdauer** | Monate/Jahre werden auf 30/365 Tage approximiert — **nicht** ISO-8601-konform für kalenderabhängige Dauern | `bpmn-parser/gotchas.md` |
| **`/api/definitions/:id/xml` ↔ JSON-Antwort** | Liefert `text/plain` mit rohem XML, kein `{ "xml": ... }` | `engine-server/gotchas.md` |
| **`bpmn_process_id` ↔ `bpmn_id`** | `bpmn_process_id` ist der neue Name; `bpmn_id` bleibt Alias | `engine-server/gotchas.md` |
| **`business_key` in `variables` ↔ top-level** | Top-level hat Vorrang; Engine entfernt den Schlüssel aus den Variablen | `engine-server/gotchas.md` |

---

## 3. Standards und Best Practices mit Quellen

### 3.1 Dokumentations-Frameworks

**Diátaxis** ([diataxis.fr](https://diataxis.fr/), [Map](https://diataxis.fr/map/))
Vier Nutzerbedürfnisse, vier Formen: *Tutorials* („Can you teach me to…?“, lernorientiert, eine Lektion), *How-to guides* („How do I…?“, zielorientiert, Schrittfolge), *Reference* („What is…?“, informationsorientiert, trockene Beschreibung), *Explanation* („Why…?“, verständnisorientiert, diskursiv). Diátaxis adressiert explizit Inhalt, Stil **und** Architektur. Die Map warnt vor dem „Blur“ zwischen benachbarten Formen — insbesondere dem Zusammenfallen von Tutorial und How-to. Übertragung auf BPMNinja: Die vorhandene Knowledge Base ist fast ausschließlich *Reference* (Interfaces, Dependencies, Gotchas) mit Explanation-Anteilen (ADR, Data Flows). Tutorials und How-tos fehlen vollständig.

**Google Developer Documentation Style Guide** ([Highlights](https://developers.google.com/style/highlights), zuletzt aktualisiert 2026-10-09)
Kernregeln: konversationell und freundlich, aber nicht frivol; nichts „ankündigen“ (keine „Coming soon“-Floskeln in Doku); beschreibender Linktext; barrierefrei schreiben; für ein globales Publikum schreiben; zweite Person („you“, nicht „we“); Aktiv; Bedingungen **vor** Anweisungen; Sentence Case für Titel; nummerierte Listen für Sequenzen, Bullet-Listen sonst; Code in Code-Font, UI-Elemente fett; eindeutige Datumsformate; Alt-Text für Bilder. Übertragung: Die BPMNinja-Doku ist derzeit in Teilen in deutscher Sprache (Gotchas) und in Teilen englisch (README, Architecture) — hier ist eine **Sprachentscheidung** zu treffen und zu dokumentieren. Google verlangt außerdem konsistente Datumsformate — relevant für Timer-Beispiele (`2026-04-06T14:30:00Z`).

**Microsoft Writing Style Guide** ([Welcome](https://learn.microsoft.com/en-us/style-guide/welcome/))
Leitprinzip „Make every word matter“; Voice: „warm and relaxed, crisp and clear, and ready to lend a hand“; enthält Kapitel zu bias-freier Kommunikation und globaler Kommunikation. Übertragung: Für die Zielgruppe „Betriebsteams“ ist der „crisp and clear“-Anteil wichtiger als der „warm“-Anteil; für Einsteiger umgekehrt. Das spricht für rollenspezifische Einstiegsseiten statt einer einzigen Stimme.

**IEEE/ISO-Dokumentationsnormen** — *Recherchelücke, siehe §8.* Die einschlägigen Normen (IEEE 1063 Software User Documentation, ISO/IEC/IEEE 26511 Requirements for managers of information for users, 26512 Requirements for acquirers and suppliers, 26513 Requirements for testers and reviewers, 26514 Requirements for designers and developers, 26515 Developing information for users in an agile environment) konnten in dieser Recherche **nicht als Primärquelle abgerufen werden** (ISO-Paywall/403). Sie sind als **offene Lücke markiert** und dürfen im Bericht nicht als belegt zitiert werden. Empfehlung: Vor Schreibbeginn über eine Bibliothek oder die ISO-Online-Browsing-Plattform verifizieren, insbesondere 26511 (Management-Prozess) und 26514 (Design/Entwicklung), weil sie Abnahme- und Reviewkriterien liefern, die zu unserem Freigabeprozess passen.

### 3.2 Domänenspezifische Standards

**OMG BPMN 2.0.2** ([omg.org/spec/BPMN/2.0.2](https://www.omg.org/spec/BPMN/2.0.2/))
Formale Spezifikation, Januar 2014, normative maschinenlesbare Dokumente (BPMN20.xsd, Semantic.xsd, BPMNDI.xsd). Jede Aussage der Form „BPMN verlangt X“ muss gegen diese Spezifikation belegt werden — nicht gegen Sekundärliteratur. Für BPMNinja besonders relevant: die Semantik von Gateways, Boundary Events, Sub-Processes und Multi-Instance, weil BPMNinja hier nachweislich abweicht.

**Camunda 7 — External Tasks** ([docs.camunda.org](https://docs.camunda.org/manual/7.24/user-guide/process-engine/external-tasks/))
Definiert das External-Task-Pattern normativ für die Camunda-Welt: drei Schritte, Topic, Lock, Long Polling, `camunda:type="external"` + `camunda:topic`, `camunda:errorEventDefinition` mit Expression, Unique-Worker-Request, Blocking-Queue-Kapazität. Begründet das Muster mit fünf Vorteilen: Systemgrenzen, Technologiegrenzen, spezialisierte Worker, feingranulare Skalierung, unabhängige Wartung. **Das ist die Referenz, gegen die BPMNinjas „Camunda-kompatibel“-Aussage geprüft werden muss.**

**Camunda 8 — Job Workers, Service Tasks, Incidents, Migration** ([Job Workers](https://docs.camunda.io/docs/components/concepts/job-workers/), [Service Tasks](https://docs.camunda.io/docs/components/modeler/bpmn/service-tasks/), [Incidents](https://docs.camunda.io/docs/components/concepts/incidents/), [Migration](https://docs.camunda.io/docs/components/concepts/process-instance-migration/))
Camunda 8 ersetzt „External Task“ durch „Job“/„Job Worker“, führt `taskDefinition` mit `type` und `retries` ein, dokumentiert Long Polling, Job-Queuing, Retry-Backoff, Idempotenz-Anforderung („at least once“), Timeout-Update, Job-Streaming. Incidents werden als blockierter Instanzzustand mit explizitem Resolve-Ablauf beschrieben. Migration wird scharf von Modification getrennt. **Wichtig für BPMNinja:** Die Begriffe „External Task“ (Camunda 7) und „Job“ (Camunda 8) bezeichnen dasselbe Muster in unterschiedlichen Generationen — BPMNinja folgt sprachlich Camunda 7, sollte das aber explizit sagen, sonst suchen Camunda-8-Nutzer nach „Job Worker“ und finden nichts.

**Flowable Open Source — BPMN Introduction** ([flowable.com](https://www.flowable.com/open-source/docs/bpmn/ch07a-BPMN-Introduction))
Zeigt die didaktisch saubere Reihenfolge: Was ist BPMN → Prozess definieren → Deployen → Instanz starten → Wait State erklären. Definiert Deployment als zwei Effekte (persistente Speicherung + Parsen in Objektmodell) und führt die Key/ID-Unterscheidung ein (`startProcessInstanceByKey` vs. `startProcessInstanceByID`, ID-Format `key:version`, max. 64 Zeichen). **Direkt übertragbar als Struktur für ein BPMNinja-Einsteiger-Tutorial.**

### 3.3 Sicherheitsanforderungen

Aus dem Repository belegbar (und damit dokumentationspflichtig):

- **Fail-closed Auth:** Gate aktiv, sobald `BPMNINJA_API_KEY` oder `BPMNINJA_WORKER_API_KEY` gesetzt ist; zusätzlich fail-closed ohne Key bei Nicht-Loopback-Bind (`BIND_ADDR` ≠ `127.0.0.1`/`::1`/`localhost`) → 401 auf allen `/api`-Routen außer `/api/health` und `/api/ready`. Bewusster Opt-out: `BPMNINJA_ALLOW_UNAUTHENTICATED=true`. Quelle: `docs/knowledge-base/modules/engine-server/gotchas.md`, `interfaces.md`.
- **Rollenmatrix:** Admin darf alles; Worker nur `fetchAndLock`, `complete`, `failure`, `extendLock`, `bpmnError`; `retry`/`resolve`/`/metrics`/alles andere → 403. Fehlversuche werden mit Methode + Pfad geloggt (ohne Secrets) und als `bpmn_auth_failures_total{reason}` gezählt.
- **CORS-Allowlist statt `Any`:** Default `http://localhost:1420`, `http://127.0.0.1:1420`, `tauri://localhost`, `https://tauri.localhost`; `CORS_ORIGINS=*` nur als Dev-Escape-Hatch.
- **Body-Limits:** `MAX_XML_BYTES` 10 MiB (Deploy + JSON/XML-Body), `MAX_UPLOAD_BYTES` 5 MiB (Multipart) → 413.
- **Rhai-Sandbox:** `RHAI_MAX_OPERATIONS` (50 000), `RHAI_MAX_MEMORY_BYTES` (2 MiB, abgeleitet auf String-/Array-/Map-Limits), `RHAI_TIMEOUT_MS` (1 000).
- **NATS-Auth:** docker-compose nutzt User/Pass (`bpmninja`/`bpmninja`, änderbar) und bindet NATS-Ports nur an Loopback.
- **Tauri-CSP:** `connect-src` erlaubt `http:`/`https:`/`ws:`/`wss:` (Engine-URL nutzerkonfigurierbar), bpmn-js benötigt `style-src 'unsafe-inline'`, `script-src 'unsafe-eval' 'wasm-unsafe-eval'`, `worker-src blob:`.

**Sicherheitsdokumentations-Aufgabe:** Diese Punkte sind heute über vier Dateien verstreut. Sie brauchen eine eigene Seite „Security model & hardening“ mit Threat-Modell-Skizze (wer darf was, was passiert bei Fehlkonfiguration) und einer Checkliste für Produktion.

### 3.4 Barrierefreiheit

**WCAG 2.2** — *Primärquelle in dieser Recherche nicht abrufbar* (w3.org lieferte 403 für `/TR/WCAG22/`, `/WAI/WCAG22/quickref/` und `/WAI/standards-guidelines/wcag/`). **Als Lücke markiert (§8).** Belastbar aus dem Repository: Die Desktop-UI nutzt Radix UI und benötigt `DialogDescription` für Screenreader; fehlende Beschreibungen erzeugen Radix-Warnungen (`desktop-tauri/gotchas.md`). Google verlangt Alt-Text für Bilder und hochauflösende/vektorielle Bilder ([Highlights](https://developers.google.com/style/highlights)). Für die Doku selbst gilt: Alt-Text für alle Mermaid-Ersatzbilder, beschreibende Linktexte, keine rein farbcodierten Statusaussagen (die Storage-Mode-Badge „NATS / In-Memory“ ist farbcodiert — Doku muss den Text nennen, nicht nur die Farbe).

### 3.5 Weitere anwendbare Leitlinien (aus dem Repository ableitbar)

- **OpenAPI 3.0 als Vertrag:** `docs/openapi.yaml` ist die Referenzquelle für die REST-API; jede neue Route muss dort ergänzt werden (`engine-server/gotchas.md`, Schritt 5). Doku-Regel: API-Referenz wird **aus** OpenAPI generiert oder zumindest dagegen geprüft, nie parallel gepflegt.
- **Semantic Versioning:** Camunda wendet SemVer auf seinen Java-Client an ([Java Client](https://docs.camunda.io/docs/apis-tools/java-client/getting-started/)). BPMNinja hat Versionen 0.7.24 (Rust/Desktop) und 1.0.0 (npm-Client) — die Doku muss klarstellen, welche Version welches Kompatibilitätsversprechen hat.
- **ADR-Praxis:** Die vorhandenen ADRs (001–010) sind vorbildlich struktur
