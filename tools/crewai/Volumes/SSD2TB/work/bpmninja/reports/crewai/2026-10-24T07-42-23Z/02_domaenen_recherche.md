# BPMN-2.0-Engines und angrenzende Standards — Marktstand und BPMNinja-Lücken

**Methode:** 13 Web-Primärquellen (Abruf **2026-10-24**, Camunda-Doku jeweils Stand **8.10** bzw. **7.24**) plus Codeprüfung im Repo (Workspace 0.7.24). Wo die Quellenlage nicht eindeutig ist, ist das ausdrücklich als *unsicher* markiert.

## Vergleichstabelle

| Fähigkeit | Wettbewerber | BPMNinja-Status (belegt) | Lücke/Relevanz | Quelle |
|---|---|---|---|---|
| Kern-BPMN (Tasks, XOR/AND/OR, Boundary, Timer, Message) | C7/C8/Flowable breit | 29 `BpmnElement`-Varianten, **ComplexGateway ausführbar** (`element.rs:44`, `README.md:123`) | keine Lücke, eher Vorsprung (C8: ComplexGateway laut Coverage nicht ausführbar — *Farbsemantik im Textabruf nicht prüfbar, unsicher*) | Q1, Q10 |
| **Multi-Instance** | C8: parallel/sequenziell, `inputCollection`/`outputCollection`, Boundary-Events (8.10); C7/Flowable: ja | **Deploy-Fehler** „Multi-instance is not supported" (`parser.rs:280-290`, Test `tests.rs:1229`) | **hoch** — Marktstandard, blockiert jeden Camunda-Import mit MI-Marker | Q6, Q10 |
| DMN | C8: DMN 1.3, Decision Tables, FEEL, BKM; Flowable: Decision Tables (JUEL) | kein DMN; `businessRuleTask` → ServiceTask mit Topic, sonst Parse-Fehler (`parser.rs:501-512`, `README.md:172`) | mittel–hoch; Verhalten inkonsistent dokumentiert (README vs. Parserfehler) | Q7, Q10 |
| External Tasks / Job Worker | C7: `camunda:type/topic`, fetchAndLock, **`camunda:errorEventDefinition` mit JUEL**; C8: Job-Worker `activateJobs`, Streaming, Job-Tags | fetchAndLock/complete/failure/extendLock/bpmnError (`server/mod.rs:230-250`), TS-Client, OpenAPI | Parität zu C7-API, **nicht** zu C8 (kein Streaming/Job-Tags); `errorEventDefinition` fehlt → C7-Worker-Migration bricht | Q3, Q9 |
| Incidents | C8: Job-Retries=0, Condition≠boolean, Timer-Ausdruck, DMN-Evaluierung, ungefangener BPMN-Error | Incident nur bei Service-Task-Retries=0, retry/resolve (`service_task.rs:266-464`) | mittel: Persistenz-Erschöpfung und Timer-Fehler erzeugen **keinen** Incident/Dead-Letter | Q2, Repo |
| Compensation / Escalation / Boundary | C8: Compensation (ohne Event-Subprocess-Variante); Flowable: inkl. Cancel/Transaction | Throw/End/Boundary, LIFO, `activityRef`, interrupting/non-interrupting (`parser.rs:671-727`) | keine Lücke (Transaction/Cancel bewusst außen vor, C8 ebenfalls) | Q1, Q10 |
| Signal-Events | Flowable: Signal-Start/Catch/Boundary/Throw, Broadcast (belegt); C8 im Event-Raster geführt (*Unsicherheit*) | explizit nicht unterstützt (`README.md:179`) | mittel; NATS-Pub/Sub wäre natürliche Semantik | Q10, Q1 |
| Prozessmigration | C8: Mapping inkl. **Catch-Event-Subscriptions** (Timer-Restlaufzeit), Subprozess-/Ad-hoc-Mapping, keine Typwechsel | `/api/instances/{id}/migrate` mit Node-Mapping, Pre-Validation, `OrphanedToken`; `/move-token` (`instance_ops.rs:616`, `tests/migration.rs`) | Kern paritätisch; Subscription-/Timer-Mapping fehlt (Restlaufzeit geht verloren) | Q5, Repo |
| Persistenz / HA | C8: partitioniert+repliziert; Temporal: Event-History-Replay | NATS JetStream KV/Stream/ObjectStore, fail-closed `REQUIRE_NATS`; Timer single-writer | HA/Scale-out **nicht belegt** (kein Leader-Election-Nachweis; Doppel-Timer bei 2 Nodes unverifiziert) — unsicher | Q2, Q11, Repo |
| Observability | C8: Micrometer → Prometheus **und OTLP**, Job-Tag `trace-id`; Temporal: Metrics+Tracing | `/metrics` (Prometheus), JSON-Logs, Log-Buffer, SSE; **keine OTel-Deps** (`observability.rs:4` nennt nur den eingestellten `opentelemetry-prometheus`-Weg) | mittel: kein OTLP, keine Trace-Korrelation in History/Jobs | Q4, Q3 |
| **Multi-Tenancy / AuthZ** | C8: Tenant-ID auf Definition/Instanz/Job, Tenant-Checks, `<default>`-Tenant; Temporal: Namespaces + Authorizer/RBAC | **kein Tenant-Begriff** (0 Treffer für `tenant` im Rust-Code), nur globaler API-Key | **hoch** für SaaS/Mehrteam | Q8, Q11 |
| Audit / History | C8: Operate/History; Temporal: Event-History | History mit Diffs, Snapshots, Akteuren, Archivsuche | keine Lücke im Funktionsumfang; Performance-Rand (Consumer pro Request) | Repo |
| DX / API-Verträge / Clients | C8: REST v2 (Backward-Compat), Java/Spring/Node; Orkes: JSON-DSL, 7 SDKs; **bpmn-js: nur Rendering/Modellierung, keine Engine** | eigenes REST (`/api/...`, nicht C8-kompatibel), TypeSpec→OpenAPI, TS-Client, Tauri+bpmn-js | Lücke: kein Java-SDK, kein gRPC; API nicht C8-kompatibel (bewusste Differenzierung, aber Migrationshürde) | Q9, Q12, Q13 |

## Sechs BPMNinja-spezifische Verbesserungsideen

1. **Multi-Instance minimal implementieren** (parallel + `inputCollection`, Service-/User-Task, `loopCounter`). Heute hart abgelehnt (`parser.rs:284`); C8 unterstützt MI auf allen Activities inkl. Output-Collection (Q6). Bis dahin: Fehlermeldung um Migrationshinweis ergänzen — **höchster Migrationshebel**.
2. **Tenant-Scoping + Rollen am API-Key**: `tenant_id` an Definition, Instanz, Job, Timer; Key→(Tenant, Rolle). Belege: C8-Tenant-Checks (Q8), Temporal-Namespaces (Q11). Heute 0 Treffer im Code — Voraussetzung für Mehrteam-Betrieb.
3. **Incident-Modell über Job-Retries hinaus**: Persistenz-Erschöpfung (Retry-Queue-Drop), Timer-/Condition-Auswertungsfehler und ungefangene BPMN-Errors am Prozessscope als Incident/Dead-Letter mit Resolve-Pfad. Vorbild: C8-Incident-Ursachenliste (Q2); heute deckt `service_task.rs` nur Job-Incidents ab.
4. **OTLP-Export + Trace-Kontext**: `opentelemetry-otlp` zusätzlich zum Prometheus-Handle, `trace-id`/`span-id` in History-Events und Job-Payload (C8-Job-Tags, Q3/Q4). Damit wird die vorhandene History für verteiltes Debugging nutzbar.
5. **Camunda-7-Kompatibilitätsmatrix + `camunda:errorEventDefinition`**: C7-Worker erwarten Error-Event-Definitionen mit JUEL-Ausdruck am External Task (Q9); BPMNinja kennt nur `bpmnError` zur Laufzeit. Eine explizite „unterstützt/nicht"-Matrix verhindert falsche Migrationserwartungen (inkl. der README/Parser-Diskrepanz bei `businessRuleTask`, Fund oben).
6. **Migration um Event-Subscriptions erweitern**: Mapping für Timer-/Message-Catch-Events mit Erhalt der Restlaufzeit und explizites Ablehnen von Typwechseln — C8 dokumentiert genau diese Semantik (Q5); `migrate_instance` mappt heute nur Node-IDs.

**Bewusst nicht verfolgen (kein Handlungsbedarf):** Data Objects/Stores (C8 auch nur Modellierung, Q1), Transaction/Cancel-Subprocess (C8 ebenfalls nicht, Q1), DMN-Vollausbau — hier ist die **Doku-Korrektur** (Idee 5) wichtiger als ein FEEL-Interpreter.

## Quellen (Abruf jeweils 2026-10-24)

- **Q1** https://docs.camunda.io/docs/components/modeler/bpmn/bpmn-coverage/ (Camunda 8.10)
- **Q2** https://docs.camunda.io/docs/components/concepts/incidents/ (8.10)
- **Q3** https://docs.camunda.io/docs/components/concepts/job-workers/ (8.10)
- **Q4** https://docs.camunda.io/docs/self-managed/operational-guides/monitoring/metrics/ (8.10)
- **Q5** https://docs.camunda.io/docs/components/concepts/process-instance-migration/ (8.10)
- **Q6** https://docs.camunda.io/docs/components/modeler/bpmn/multi-instance/ (8.10)
- **Q7** https://docs.camunda.io/docs/components/modeler/dmn/ (8.10)
- **Q8** https://docs.camunda.io/docs/components/concepts/multi-tenancy/ (8.10)
- **Q9** https://docs.camunda.org/manual/7.24/user-guide/process-engine/external-tasks/ (C7 7.24)
- **Q10** https://www.flowable.com/open-source/docs/bpmn/ch07b-BPMN-Constructs (Flowable OSS, undatiert)
- **Q11** https://docs.temporal.io/namespaces (undatiert)
- **Q12** https://orkes.io/content/ (undatiert)
- **Q13** https://github.com/bpmn-io/bpmn-js (develop, undatiert)

**Repo-Belege:** `bpmn-parser/src/parser.rs:280-290, 501-512, 671-727`; `bpmn-parser/src/tests.rs:1229-1252`; `engine-core/src/domain/element.rs:5-94`; `engine-core/src/engine/service_task.rs:266-464`; `engine-core/src/engine/instance_ops.rs:616`; `engine-server/src/server/mod.rs:150-254`; `engine-server/src/observability.rs:4`; `README.md:96-181`.