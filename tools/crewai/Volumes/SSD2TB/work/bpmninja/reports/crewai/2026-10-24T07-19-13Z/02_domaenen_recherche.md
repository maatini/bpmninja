# BPMN-2.0-Engines im Marktvergleich — Stand Oktober 2026

**Methode:** 14 Primär-/Sekundärquellen (Abruf 2026-10, alle am selben Tag), plus Code-/Doku-Prüfung im BPMNinja-Repo (HEAD `a8f7305`, `README.md:96-182`, `bpmn-parser/gotchas.md`, `engine-core/src/domain/element.rs`, `engine-server/src/server/mod.rs`).

## Vergleichstabelle

| Fähigkeit | Wettbewerber | BPMNinja-Status | Lücke/Relevanz | Quelle |
|---|---|---|---|---|
| Kern-BPMN (Tasks, XOR/AND/OR, Boundary, Timer, Message) | C7, C8, Flowable: breit; **C8: ComplexGateway nicht ausführbar** | 29 Elemente, **ComplexGateway first-class** | keine Lücke, eher Vorsprung | Q2, Q9 |
| Compensation / Escalation | C7/C8/Flowable: ja (C8: Compensation-Boundary/Throw/End; Event-Subprocess-Variante fehlt) | Throw/End/Boundary, LIFO, `activityRef` | keine Lücke | Q2, Q9 |
| **Multi-Instance** | C7/C8/Flowable: ja (parallel/sequential) | **Deploy-Fehler** „Multi-instance is not supported" (`gotchas.md:37-45`) | **hoch**, Marktstandard, bewusst abgelehnt | Q2, Q9 |
| **DMN** | C7 eingebaut; C8: DMN 1.3 + FEEL; Flowable: Decision Tables, JUEL statt FEEL | kein DMN; `businessRuleTask` → ServiceTask (README) **bzw. Fehler ohne Topic** (gotchas) — inkonsistent | mittel–hoch, für C7-Migrationen erwartet | Q7, Q8 |
| Signal / Conditional / Event-Subprocess / Transaction | C8: Signal+Conditional+Event-Subprocess ja, Transaction nein; Flowable: inkl. Cancel/Transaction | Signal/Cancel/Link/Ad-hoc fehlen (README:175-181); README:153 behauptet „Scope Event Listener" — **Widerspruch zum Element-Enum, unklar** | mittel | Q2, Q9 |
| External Tasks / Job Worker | C7: `camunda:type=external` + fetchAndLock; C8: Job-Worker (activateJobs), **keine** External-Task-API | fetchAndLock/complete/failure/extendLock/bpmnError + TS-Client | Parität zu C7, nicht zu C8 | Q6, Q4 |
| Incidents | C8: Incident bei `retries=0`, Resolve/Retry, Ursachenliste | Incident bei Service-Task-Retries 0, retry/resolve | Lücke: Persistenz-Erschöpfung erzeugt **keinen** Incident/Dead-Letter (F3) | Q3 |
| Prozessmigration / -modifikation | C8: Migration mit Node-Mapping + Modification; aktive Elemente migrierbar | `/api/instances/:id/migrate` (Mapping, Pre-Validation, `OrphanedToken`) + `/move-token` | im Kern paritätisch; Subprozess-/Call-Activity-Grenzen unklar | Q5 |
| Persistenz / HA | C8 partitioniert+repliziert; Temporal: Event-History-Replay, Namespaces | NATS KV/Stream/ObjectStore, fail-closed `REQUIRE_NATS`; **Single-Writer-Timer, keine Partitionierung belegt** | HA/Scale-out unklar (Doppel-Timer bei 2 Servern nicht verifiziert) | Q10, Q12 |
| Observability | C8: Micrometer → Prometheus **und OTLP**; strukturierte Fehler | Prometheus `/metrics`, JSON-Logs, Log-Buffer, SSE; **kein OpenTelemetry** (keine OTel-Deps im Workspace) | mittel | Q12 |
| Multi-Tenancy / AuthZ | C8: Tenant-IDs + Tenant-Checks; Temporal: Namespaces + Authorizer/RBAC; Flowable: tenantId | nur globaler API-Key (`BPMNINJA_API_KEY`), kein Tenant-Begriff | **hoch** für SaaS/Teams | Q13, Q11 |
| Audit / History | C8 Operate/History; Temporal Event-History | History mit Diffs, Snapshots (alle 8), Akteuren, Archiv-Suche | keine Lücke | Repo |
| DX / Clients | C8: Java/Spring/Node + REST; Orkes: 7 SDKs (JSON-DSL); bpmn-js: nur Rendering/Modellierung, keine Ausführung | OpenAPI/Redoc, TS-Client, Tauri+bpmn-js-UI | Lücke: nur REST+TS, kein Java/gRPC | Q14, Q15, Q1 |
| Lizenz | **C8 Self-Managed: Produktion kostenpflichtig** (Camunda License v1); C7 Apache-2.0 (EOL-nah, sekundär belegt) | MIT/Apache-2.0 | **Differenzierung**, keine Lücke | Q16, Q17 |

**Bewusste Nicht-Ziele (kein Handlungsbedarf):** Data Objects/Data Stores (C8 auch nur Modellierung, Q2), Transaction-Subprocess (C8 ebenfalls nicht, Q2), Loop-Marker (C8 nicht, Q2).

## Sechs Verbesserungsideen

1. **Multi-Instance minimal implementieren** (parallel, Collection-Expression) für Service-/User-Task; bis dahin Fehlermeldung um Migrationshinweis ergänzen. Marktstandard bei C7/C8/Flowable (Q2, Q9); aktuell bricht jeder Camunda-Import mit `multiInstanceLoopCharacteristics` ab (Repo-Beleg).
2. **Camunda-7-Kompatibilitätsmatrix + `camunda:errorEventDefinition`**: fetchAndLock existiert, aber Camunda-7-Worker nutzen zusätzlich Error-Event-Definitionen am External Task (Q6). Matrix „unterstützt/nicht" verhindert falsche Migrationserwartungen.
3. **Signal-Events über NATS-Subjects**: Signal-Start/Catch/Boundary/Throw ist bei C8 ausführbar (Q2), bei BPMNinja explizit nicht. Broadcast-Semantik passt architektonisch zum bestehenden NATS-Pub/Sub — geringer Aufwand, hoher Abdeckungsgewinn.
4. **DMN-Entscheidung: ablehnen oder Light-Variante** — Doku widerspricht sich (`businessRuleTask` → ServiceTask vs. Parse-Fehler). Entweder strikt ablehnen und README:172 korrigieren, oder JSON-Entscheidungstabelle mit Hit-Policy FIRST/UNIQUE in Rhai (Vorbild: Flowable/C8, Q7/Q8).
5. **Betriebsreife: Incident statt stillem Verlust + OTel** — Persistenz-Erschöpfung (F3) muss Incident + Dead-Letter erzeugen (Benchmark: C8-Incident-Modell, Q3); `/api/ready` an Restore-Ergebnis koppeln (F2); OTel-Trace-ID an History-Einträge (C8-Job-Tags `trace-id`, Q4/Q12).
6. **Tenant-Scoping + Rollen am API-Key** — `tenant_id` an Definition/Instanz, Key→Tenant/Rolle; Belege: C8-Tenant-Checks (Q13), Temporal-Namespaces (Q11). Voraussetzung für Mehrteam-/SaaS-Betrieb, heute komplett offen.

## Quellen (alle Abruf 2026-10)

- **Q1** https://github.com/bpmn-io/bpmn-js — „rendering toolkit and web modeler", keine Engine.
- **Q2** https://docs.camunda.io/docs/components/modeler/bpmn/bpmn-coverage.md — Camunda 8.10; nicht-„implemented": ComplexGateway, Loop, Transactional Subprocess.
- **Q3** https://docs.camunda.io/docs/components/concepts/incidents/ — Incident-Ursachen/Resolve (8.10).
- **Q4** https://docs.camunda.io/docs/components/concepts/job-workers.md — Job-Worker, Job-Tags inkl. `trace-id`.
- **Q5** https://docs.camunda.io/docs/components/concepts/process-instance-migration/ — Mapping-Regeln, keine Typwechsel.
- **Q6** https://docs.camunda.org/manual/7.24/user-guide/process-engine/external-tasks/ — `camunda:type/topic`, Fetch&Lock, `camunda:errorEventDefinition`.
- **Q7** https://docs.camunda.io/docs/components/modeler/dmn/ — DMN 1.3, FEEL, Decision Tables.
- **Q8** https://www.flowable.com/open-source/docs/dmn/ch06-DMN-Introduction — Decision Tables, JUEL statt FEEL.
- **Q9** https://www.flowable.com/open-source/docs/bpmn/ch07b-BPMN-Constructs — BPMN-Konstrukte, Cancel/Compensation, Multi-Instance-Erwähnung.
- **Q10** https://docs.temporal.io/workflows — Code-first, Event-History-Replay, kein BPMN.
- **Q11** https://docs.temporal.io/namespaces — Namespace-Isolation, Authorizer/RBAC.
- **Q12** https://docs.camunda.io/docs/self-managed/operational-guides/monitoring/metrics.md — Micrometer, Prometheus, OTLP.
- **Q13** https://docs.camunda.io/docs/components/concepts/multi-tenancy/ — Tenant-IDs, Tenant-Checks.
- **Q14** https://docs.camunda.io/docs/apis-tools/working-with-apis-tools.md — offizielle Clients Java/Spring/Node, REST-Backward-Compatibility.
- **Q15** https://orkes.io/content/ — JSON-DSL, 7 SDKs, durable execution, kein BPMN.
- **Q16** https://docs.camunda.io/docs/reference/licenses.md — C8 Self-Managed Produktion = kostenpflichtige Enterprise-Lizenz.
- **Q17** https://en.wikipedia.org/wiki/Camunda — C7 Apache-2.0, „maintained by forked projects" (**EOL-Datum nicht primär belegt, unsicher**).
- **Q18** https://www.omg.org/spec/BPMN/2.0.2/ — BPMN 2.0.2 formal, Januar 2014 (Referenzstandard).