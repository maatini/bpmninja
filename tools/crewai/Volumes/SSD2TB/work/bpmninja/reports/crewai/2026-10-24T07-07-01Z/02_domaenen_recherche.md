# BPMN-2.0-Engines & angrenzende Standards — Marktstand vs. BPMNinja v0.7.24

**Prüfmethode und Quellenlage (wichtig):** Meine Tool-Umgebung in dieser Session umfasste ausschließlich Repository-Zugriff (`repo_read`, `repo_search`, `repo_stats`). Ein Webabruf (`web_search`/`fetch_url`) war **nicht verfügbar**. Ich habe daher **keine Quelle live abgerufen** und zitiere keine Zitate aus dem Gedächtnis. Externe URLs sind unten als **„nicht verifiziert“** markiert — Abrufdatum: **keiner**; Verifikation ist nachzuholen. Alle Aussagen zum **BPMNinja-Status** sind dagegen primär aus dem Code belegt (Datei:Zeile) und damit belastbar. Aussagen zu Wettbewerbern sind als **unsicher** gekennzeichnet, wo ich sie nicht aus dem Repo-Kontext stützen kann.

## Quellen

| # | Quelle | Status |
|---|---|---|
| Q1 | `bpmn-parser/src/parser.rs:277–290, 501–513`; `bpmn-parser/src/tests.rs:1230` | **verifiziert** (Repo, `v0.7.24`) |
| Q2 | `engine-core/src/domain/element.rs:1–95`; `engine-core/src/runtime/instance.rs:43–46, 169–172`; `engine-core/src/engine/executor/next_action.rs:327–364` | **verifiziert** |
| Q3 | `engine-server/src/server/mod.rs:150–258`; `engine-core/src/engine/service_task.rs:266–300`; `engine-core/src/runtime/pending.rs:46–51` | **verifiziert** |
| Q4 | `engine-core/src/port/persistence.rs:91–168`; `persistence-nats/src/client.rs`; `engine-core/src/engine/tests/migration.rs` | **verifiziert** |
| Q5 | `docs/knowledge-base/modules/bpmn-parser/gotchas.md:37–57`; `docs/knowledge-base/modules/external-task-client/index.md:1–8` | **verifiziert** |
| Q6 | Camunda 7 Docs (External Tasks, Incidents, Compensation), https://docs.camunda.org/manual/latest/ | **nicht abgerufen** |
| Q7 | Camunda 8 / Zeebe Docs (Multi-Tenancy, Job Workers, Migration), https://docs.camunda.io/ | **nicht abgerufen** |
| Q8 | Flowable Docs (BPMN 2.0 Support, DMN, Tenant), https://www.flowable.com/open-source/docs/ | **nicht abgerufen** |
| Q9 | Temporal Docs (Versioning, Determinism, Observability), https://docs.temporal.io/ | **nicht abgerufen** |
| Q10 | Orkes Conductor Docs, https://orkes.io/content/ | **nicht abgerufen** |
| Q11 | bpmn-io / bpmn-js, https://github.com/bpmn-io/bpmn-js | **nicht abgerufen** |
| Q12 | OMG BPMN 2.0.2 / DMN 1.5 Specs, https://www.omg.org/spec/BPMN/ , https://www.omg.org/spec/DMN/ | **nicht abgerufen** |

## Vergleichstabelle

| Fähigkeit | Wettbewerber (unsicher, sofern nicht anders vermerkt) | BPMNinja-Status (belegt) | Lücke / Relevanz | Quelle |
|---|---|---|---|---|
| External Tasks (fetch-and-lock, lock extension, complete/failure) | Camunda 7 Kern-API | **vorhanden**: `/api/service-task/fetchAndLock`, `complete`, `failure`, `extendLock`, `bpmnError`; Node-SDK mit Long-Polling + Backoff | **keine Lücke** | Q3, Q5 |
| Incidents | Camunda 7: Jobs/Incidents inkl. Retry-Intervall | **vorhanden**, aber **nur Service Tasks** (`retries <= 0` → Incident, `retry`/`resolve`) | **Teillücke**: Script-/Listener-Fehler (`EngineError::ScriptError`) werden **nicht** zum Incident | Q3 |
| Compensation | Camunda 7/8: Boundary-Compensation + Throw | **vorhanden**: `BoundaryCompensationEvent`, `CompensationThrowEvent`, `compensation_log` (LIFO) | **keine Lücke** | Q2 |
| Boundary Events (Timer/Message/Error/Escalation) | Standard | **vorhanden**, inkl. `cancel_activity` | **keine Lücke** | Q2 |
| Multi-Instance | Camunda 7/8, Flowable, Temporal (via Loops/Child-WF) | **abgelehnt**: Deploy-Fehler bei `multiInstanceLoopCharacteristics`; Runtime emittiert `MultiInstanceFork` nie | **echte Lücke**, hohe Marktrelevanz — aber bewusst statt stiller Fehlausführung | Q1, Q5 |
| DMN / Business Rule Task | Camunda 7 (Camunda DMN), Camunda 8 (DMN-Engine), Flowable | **nicht unterstützt**: `businessRuleTask` mappt nur mit `data-topic` auf Service Task | **Lücke**, mittlere–hohe Relevanz | Q1, Q5 |
| Multi-Tenancy | Camunda 8 `tenantId`, Flowable Tenant-Engine | **keine Spur**: `tenant` in `**/*.rs` → 0 Treffer | **echte Lücke** für SaaS-Betrieb | Q3 (Volltextsuche) |
| Persistenz / Durability | Zeebe: eigenes Log + Elasticsearch; Temporal: Cassandra/Postgres | NATS JetStream KV + Object Store, 35-Methoden-Port | solide; **bekannte Schwächen** bereits im Audit (Silent-Skip, Retry-HOL, Publish-ohne-Ack) | Q4 |
| Observability | Camunda 8: OTel + Operate; Temporal: Metrics + Web UI | Prometheus-Exporter, Prozess-Metriken, HTTP-Middleware, `/api/health`, `/api/ready`, SSE | **Teillücke**: kein OTel-Trace-Export, keine W3C-`traceparent`-Weitergabe an Worker | Q3 |
| Prozessmigration | Camunda 8: eingeschränkt; Camunda 7: eigene Batch-Migration | **vorhanden**: `/api/instances/{id}/migrate` mit Node-Mapping, lehnt completed/suspended ab, scheitert bei verwaistem Token | **Stärke** (Marketing-fähig) | Q4 |
| Audit / History | Camunda: Historic-Process-Instances + Operation Log | `HistoryEntry` mit `actor_type`, `actor_id`, `diff`, `context`; zusätzlich `audit_log` **auf 200 Einträge gekappt** | Teillücke: `audit_log` ist lossy, Actor-Identität nicht aus Auth-Claim gespeist | Q4 |
| API-Verträge / Clients | REST + gRPC (Zeebe), SDKs mehrsprachig | REST + OpenAPI (TypeSpec), Desktop-Tauri, Node-Client | **eine Sprache** im SDK-Ökosystem | Q5 |
| BPMN-Abdeckung gesamt | Camunda 7 ~Vollabdeckung | 29 `BpmnElement`-Varianten, Subprozess-Flattening | Choreographien, Ad-hoc, Conversation, MI, DMN fehlen | Q2 |

## Abgeleitete Verbesserungsideen (max. 6)

1. **Multi-Instance-Runtime fertigstellen oder Code-Pfad entfernen.** `NextAction::MultiInstanceFork/Next` und `MultiInstanceProgress` existieren, aber kein Handler emittiert sie (`next_action.rs:327–364`, kein Treffer in `engine/handlers/`). Entweder `isSequential=false` über `loopCardinality`/Collection implementieren oder die toten Typen löschen, damit der Deploy-Reject nicht als Inkonsistenz wirkt. *Aufwand: M–L.*

2. **Script-/Listener-Fehler in Incidents überführen.** `fail_service_task` erzeugt Incidents (`service_task.rs:266–300`), `ScriptError` (`scripting/runner.rs:150–205`) bricht dagegen nur ab. Incident + `retry`/`resolve`-Endpoint-Wiederverwendung schließt die Camunda-7-Parität. *Aufwand: M.*

3. **`tenantId` als First-Class-Feld einführen** (Definition, Instanz, KV-Key-Präfix, API-Key-Claim `tenant`). Null Treffer im Code = harte Lücke gegenüber Camunda 8/Flowable; ohne Tenant-Isolation ist SaaS-Betrieb nicht vertretbar. *Aufwand: M–L.*

4. **`audit_log`-Overflow persistieren.** `MAX_AUDIT_LOG_ENTRIES = 200` (`runtime/constants.rs:5`, `instance.rs:202–208`) verwirft Einträge im Ringpuffer; die verdrängten Zeilen sollten als `HistoryEntry` mit `actor_type/actor_id` geschrieben werden — nutzt die bereits vorhandene History-Pipeline und macht Audit revisionsfest. *Aufwand: S.*

5. **OTel-Trace-Kontext durchreichen.** Nur Prometheus-Metriken (`engine-server/Cargo.toml:25–30`); ein W3C-`traceparent` von HTTP → `PendingServiceTask` → Worker-Client (`@bpmninja/external-task-client`) macht Cross-Service-Traces möglich — differenzierend gegenüber Temporal/Orkes bei Rust-nativen Teams. *Aufwand: M.*

6. **DMN-Entscheidung explizit machen statt implizit.** Entweder eine minimale Decision-Table-Auswertung (Single-Output, First-Hit) für `businessRuleTask` ohne Topic, **oder** ein dokumentiertes „bewusst nicht verfolgt“ im Parser-Fehlertext. Heute ist es ein harter Deploy-Fehler (`parser.rs:501–513`) — als Positionierungsschwäche sichtbar, obwohl Camunda-Nutzer DMN erwarten. *Aufwand: S (Positionierung) / L (Feature).*

**Bewusst nicht verfolgt (Empfehlung):** Choreographie-/Conversation-Diagramme, Ad-hoc-Subprozesse und vollständige BPMN-2.0-Konformität sind für eine schlanke Engine nicht wirtschaftlich — hier ist eine öffentliche „Supported Subset“-Matrix (analog `bpmn-parser/interfaces.md:56–66`) die günstigere Antwort als Feature-Bau.