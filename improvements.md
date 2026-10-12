# BPMNinja — Priorisierte Verbesserungen

**Datum:** 2026-07-17  
**Audit-Version:** Crates/Desktop `0.7.18` (Claude.md nannte teils noch `0.7.10`)  
**Gesundheit gesamt:** 7.2 / 10  

Quelle: Architektur- und Codebase-Audit (statische Analyse). Unabhängig von `docs/PROJECT_EVALUATION.md` (8.8/10); hier bewusst stärker auf Produktionsrisiken gewichtet.

Dieses Dokument hält die **konkreten, priorisierten Verbesserungsmöglichkeiten** fest. Es ersetzt keine Feature-Roadmap, sondern bündelt technische Schulden, Risiken und High-Leverage-Maßnahmen aus dem Audit.

---

## Executive Summary

BPMNinja ist eine gut architekturierte, disziplinierte schlanke BPMN-2.0-Engine: hexagonaler Core, Token-Ausführung, Camunda-ähnliche External Tasks, NATS-Persistenz, SSE, solides Test-/Fuzz-Setup.

**Nicht** produktionsgehärtet als Multi-Tenant- oder internet-exponierte Workflow-Plattform: REST-API ohne Auth, CORS offen, File-Uploads ohne Größenlimit, stiller In-Memory-Fallback bei NATS-Ausfall (Datenverlust nach Restart), Rhai-`max_memory` konfiguriert aber nicht angewandt.

| Sicht | Score (ca.) |
|-------|-------------|
| Library / Engine-Core | ~8.0 |
| Deploy als Process-Platform „morgen“ | ~6.0 |
| Gesamt (dieses Audit) | **7.2** |

---

## Top 5 (höchster Hebel zuerst)

1. **Offene Control-Plane schließen:** Auth-Middleware + CORS-Allowlist + Upload-Limits.
2. **Fail-closed bei Durability:** `REQUIRE_NATS` / Persistence Pflicht außerhalb Dev; Readiness muss Durability spiegeln.
3. **Rhai `max_memory` wirklich anwenden** — kleinster Code-Fix mit echtem Security-Impact.
4. **`persistence-nats` integrationstesten** — der Produktionspfad ist die dünnste kritische Schicht (~2 Tests).
5. **Retry-Queue begrenzen + Quality-Metrics reparieren** — Betriebsstabilität und vertrauenswürdige Qualitäts-Gates.

---

## Kritische Issues (Kurzreferenz)

| ID | Problem | Ort (Beispiele) | Severity |
|----|---------|-----------------|----------|
| C1 | Keine AuthN/AuthZ auf der gesamten API | `engine-server/src/server/mod.rs` | Blocker bei Exposure |
| C2 | Silent in-memory Fallback bei NATS-Ausfall | `engine-server/src/main.rs` | Datenverlust-Footgun |
| C3 | Unbegrenzter File-Upload (DoS) | `engine-server/src/server/files.rs` | High |
| C4 | CORS `Any` | `engine-server/src/server/mod.rs` | High (mit C1) |
| C5 | Rhai `max_memory` tot / nicht angewandt | `engine-core/src/scripting/runner.rs` | High (Sandbox) |
| C6 | Single-Node; kein Multi-Instance-Locking | Engine + NATS; Roadmap planned | HA-Blocker |
| C7 | `engine_logs.jsonl` in Git getrackt | Repo-Root | Medium (Hygiene/Leak) |
| C8 | Unsanitized Filename in `Content-Disposition` | `files.rs` | Medium |
| C9 | Mutation-Badge 0.0% / kaputte Metrics | `docs/quality-badges.json` | Medium (Signal) |

**Status 2026-10-12:** C1 (optionaler API-Key), C2, C3, C4, C5, C7, C8 und C9 sind behoben. Offen ist nur **C6** (Multi-Node, siehe S1 / Phase 4b F4).

### Details

**C1 — Keine Auth:** Wer `:8081` erreicht, kann deployen, starten/stoppen/migraten, Tasks complete, Files uploaden, Tokens bewegen, Definitionen löschen, History/Logs/Metrics lesen. OIDC steht in der README als „Planned“.

**C2 — Silent In-Memory:** Bei `NatsPersistence::connect`-Fehler → `WorkflowEngine::new()` ohne Persistenz. `/api/health` bleibt OK; nach Restart ist alles weg. Docker-Compose ohne Fail-Fast.

**C3 — Upload DoS:** Multipart wird per `field.bytes()` vollständig gepuffert; JSON-Body-Limit 5 MB gilt nicht für Multipart.

**C5 — Rhai Memory:** `ScriptConfig.max_memory` / `RHAI_MAX_MEMORY_BYTES` existieren und sind dokumentiert; `build_engine()` setzt Ops/Timeout/Collection-Limits, ruft aber kein Memory-Cap auf.

---

## Quick Wins

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| Q1 | Rhai `max_memory` in `build_engine()` verdrahten + Unit-Test | High | Low | Low | Docs versprechen 2 MiB Cap; aktuell nicht angewandt (`scripting/runner.rs`). |
| Q2 | Fail-fast-Flag (`REQUIRE_NATS` / `REQUIRE_PERSISTENCE`), in Docker-Docs default-on | High | Low | Low | Verhindert stillen Datenverlust-Pfad. |
| Q3 | Hartes Größenlimit für Multipart-Uploads + Ablehnung | High | Low | Low | Stoppt trivialen DoS über `files.rs`. |
| Q4 | CORS per Env-Allowlist (kein `Any` im Prod-Profil) | High | Low | Low | Browser-Missbrauch zusammen mit fehlender Auth. |
| Q5 | `engine_logs.jsonl` aus Git entfernen, `.gitignore`, Rotation dokumentieren | Medium | Low | Low | Runtime-Log (~1.4 MB) getrackt; Leak-/Noise-Risiko. |
| Q6 | Claude.md / README-Version auf `0.7.18` syncen (oder generieren) | Medium | Low | None | Docs-Drift → Vertrauensverlust. |
| Q7 | Mutation-Badge / quality-metrics-Pipeline reparieren | Medium | Medium | Low | Öffentliches Signal 0.0% ist falsch. |

---

## Medium-term Improvements

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| M1 | API-Auth (v1: API-Key oder JWT; danach OIDC) | High | Medium | Medium | Ohne Auth kein ehrlicher „Production API“-Anspruch. |
| M2 | NATS-Integrationstests (testcontainers): Restore, Retry, Partial Failure | High | Medium | Low | `persistence-nats` hat nur ~2 Tests. |
| M3 | Retry-Queue bounden + Dead-Letter + Alert-Metric bei Job-Verwurf | High | Medium | Medium | `mpsc::unbounded_channel` in `retry_queue.rs` → OOM bei NATS-Ausfall + Last. |
| M4 | Topic-Index für Service Tasks + Timer-Prioritätsstruktur | Medium | Medium | Medium | Full-Scan von DashMaps skaliert nicht auf Zehntausende Tasks/Timer. |
| M5 | `unit_tests.rs` (~5076 Zeilen) nach Domänen splitten | Medium | Medium | Low | Wartbarkeit, Merge-Konflikte, Navigation. |
| M6 | `cargo deny` / audit in CI; Line-Coverage publizieren | Medium | Low–Med | Low | Supply-Chain + transparente Qualitätsmetriken. |
| M7 | `Content-Disposition`-Filenames sanitizen | Medium | Low | Low | Header-Injection-Risiko in `files.rs`. |
| M8 | Health vs Readiness: Readiness failt, wenn required Persistence fehlt | High | Low | Low | K8s-tauglich; silent in-memory bleibt sonst „healthy“. |

---

## Strategic / Larger Refactors

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| S1 | Multi-Node-Cluster (Leader für Timer + distributed Task-Lock/Fencing) | High | High | High | Zwei Server-Instanzen gegen dasselbe NATS = Split-Brain. |
| S2 | `WorkflowPersistence` splitten (Core / History / Files / Monitoring) | Medium | High | Medium | 35 async Methods auf einem Trait — ISP-Druck, teure Backends. |
| S3 | Stärkeres Konsistenzmodell (Batch/Journal oder Event Sourcing) | High | High | High | Multi-KV-Writes sind nicht atomar; Crash → Orphans. |
| S4 | FEEL/Expression-Pfad oder explizite Camunda-Kompatibilitätsmatrix | Medium | High | Medium | `condition.rs` = simple Comparisons; komplexe Logik über Rhai. |
| S5 | Volles OIDC + RBAC (Deploy vs Operate vs Work) | High | High | Medium | Multi-Tenant / Enterprise. |

---

## Empfohlener Aktionsplan (phasiert)

### Phase 0 — Sofort (1–3 Tage) — ✅ erledigt (2026-07)

1. ~~**Q1** Rhai Memory-Limit verdrahten + Tests~~ ✅  
2. ~~**Q3** Upload-Größenlimit~~ ✅  
3. ~~**Q2 / M8** Fail-fast Persistence + Readiness-Semantik~~ ✅  
4. ~~**Q5** Getrackte Logs entfernen / gitignore~~ ✅  
5. ~~**Q6** Versions-Docs syncen~~ ✅  

### Phase 1 — Production Minimum Bar (1–2 Wochen) — ✅ erledigt

1. ~~**M1** API-Authentifizierung (API-Key v1: `BPMNINJA_API_KEY`, Bearer oder `X-API-Key`)~~ ✅ (OIDC → S5)  
2. ~~**Q4** CORS per Env absichern (`CORS_ORIGINS`-Allowlist, nie stilles `Any`)~~ ✅  
3. ~~**M3** Bounded Retry + Metrics~~ ✅  
4. ~~**M2** NATS-Integrationstests (Restore/Retry)~~ ✅  
5. ~~**Q7** Mutation-Metrics-Pipeline reparieren~~ ✅ (Parser fix; Badge aktualisiert sich beim nächsten Mutation-CI-Lauf)

### Phase 2 — Scale & Maintainability (2–6 Wochen)

1. ~~**M4** Indexierung Timer/Topics (`TopicIndex`, `TimerDueIndex`)~~ ✅  
2. ~~**M5** Mega-Tests splitten (`engine-core/src/engine/tests/`, 19 Dateien)~~ ✅  
3. ~~**M6** Supply-Chain-CI (`cargo deny check` in `ci.yml`)~~ ✅  
4. Optional: OpenTelemetry-Traces  
5. `agent-orchestrator`: ausbauen oder archivieren  

### Phase 3 — Platform (Quartal+)

1. **S1** Multi-Node Design & Implementation  
2. **S2 / S3** Persistence-Modell, falls Multi-Node es erzwingt  
3. **S5** RBAC  
4. Batch-Operationen (bereits auf Roadmap)  

### Phase 4 — Performance & Cloud-native (Vorschlag, 2026-10-12)

> Status: **Vorschlag**, noch nicht beauftragt. Jede Teilphase braucht vor der Umsetzung einen eigenen Plan + `GO`.
> Grundsatz: Der In-Memory-Kern ist bereits schnell — die Latenz entsteht fast vollständig durch **I/O-Roundtrips pro Schritt**. Deshalb zuerst messen (4a/P0), dann Roundtrips eliminieren, dann verteilen.

#### 4a — Engine-Performance (Hot Path)

Befunde aus dem Code (Stand v0.7.24):

| # | Maßnahme | Impact | Effort | Risiko | Befund / Warum |
|---|----------|--------|--------|--------|----------------|
| P0 (✅ teilweise) | Messbasis: Criterion-Benches erweitern (Gateways, Scripts, Timer, mit NATS) + HTTP-Lasttest (z. B. `oha`/k6) + Flamegraph; Regressions-Gate in CI — **erledigt:** Benches (`engine_bench.rs`, `persistence-nats/benches/nats_bench.rs`); **offen:** HTTP-Lasttest, Flamegraph, CI-Gate | High | Low | None | `engine_bench.rs` misst nur lineare Prozesse ohne Persistenz. Ohne Baseline sind „schneller“-Ziele nicht überprüfbar. |
| P1 ✅ | NATS-Bucket-Handles beim Connect cachen | High | Low | Low | `trait_impl.rs` rief vor **jeder** Operation `js.get_key_value(..)` / `get_object_store(..)` auf (24 Stellen + `list_kv_entries`) — ein zusätzlicher JetStream-API-Roundtrip pro Schreib-/Lesezugriff. |
| P2 | History pro Batch statt pro Schritt schreiben (gesammelt am Ende von `run_instance_batch` bzw. asynchron über bounded Channel) | High | Medium | Medium | `run_instance_batch` klont pro Schritt alle Variablen (`DiffSnapshot::from_instance`) und wartet pro Schritt auf `append_history_entry` (JetStream-Publish inkl. Inline-Retry). |
| P3 ✅ | `persist_instance`: Lock vor dem Netzwerk-Await freigeben (Snapshot + gestreifte `PersistLocks`); Größenprüfung ohne Puffer. Doppelte Serialisierung bleibt bis P8 (braucht Trait-Änderung) | High | Low | Low | Serialisiert zweimal (Größenprüfung + Save) und hält den Instanz-Write-Lock während `save_instance` inkl. Retry-Backoff. |
| P4 ✅ | Long-Polling auf `tokio::sync::Notify` umstellen (umgesetzt: ein globaler Notify; pro Topic = Folgeschritt bei vielen Workern) | High | Low | Low | `fetchAndLock` pollt in einer 500-ms-Sleep-Schleife (`server/tasks.rs`) → bis zu 500 ms Latenz pro Service-Task-Übergabe. |
| P5 ✅ | Timer-Scheduler ereignisgesteuert: bis zum nächsten Fälligkeitszeitpunkt aus `TimerDueIndex` schlafen, bei neuem Timer per `Notify` wecken | Medium | Low | Low | Fester 1-s-Takt (`TIMER_INTERVAL_MS`) → bis zu 1 s Verspätung, CPU-Wakeups im Leerlauf. |
| P6 | Rhai: eine `Engine` wiederverwenden (Feature `sync` ist aktiv) + AST pro (Definition, Knoten) beim Deploy vorkompilieren | Medium | Medium | Low | `execute_script_safe` baut pro Aufruf eine neue `rhai::Engine` und kompiliert das Script jedes Mal neu. |
| P7 | Bedingungen beim Deploy vorparsen | Low–Med | Low | Low | `condition.rs` parst den Ausdruck bei jeder Auswertung. |
| P8 | Kompaktes Persistenzformat (z. B. `postcard`/`rmp-serde`) hinter Feature-Flag; `audit_log` aus dem Instanz-Dokument lösen; Delta- statt Voll-Dokument-Writes | Medium | High | Medium | Jeder Save schreibt die komplette `ProcessInstance` als JSON (bis 900 KiB Limit). |
| P9 | Globaler Allocator `mimalloc` im Server; `lto = "fat"` + PGO für Release evaluieren | Low–Med | Low | Low | Allokationslastiger Hot Path (Clones, JSON). |
| P10 | Feingranulare SSE-Events (Instanz-/Task-ID + Delta) statt „alles neu laden“ | Medium | Medium | Low | Grobe Events lösen bei jedem Client einen vollen REST-Refetch aus. |

**Messergebnis 4a-Quick-Wins (2026-10-12, Criterion, lokal Windows + lokaler NATS, Median vorher → nachher):**

| Bench | Vorher | Nachher | Änderung |
|---|---|---|---|
| NATS `save_instance` | 496 µs | 203 µs | −60 % (P1) |
| NATS `save_token` | 467 µs | 192 µs | −59 % (P1) |
| Engine mit NATS: start → 5 Script-Tasks → end | 7.78 ms | 4.95 ms | −36 % (P1 + P3) |
| NATS `append_history_entry` | 16.6 µs | 15.7 µs | keine signifikante Änderung (Publish wartet nicht auf Ack) |
| In-Memory-Benches (linear, Gateways, Scripts, XOR) | — | — | −2 % bis −19 %; der In-Memory-Pfad wurde kaum verändert, daher größtenteils Laufzeit-Varianz |

P4/P5 verbessern **Latenz**, nicht Durchsatz: Service-Task-Übergabe vorher bis 500 ms, jetzt sofort (Notify); Timer vorher bis 1 s verspätet, jetzt zum Fälligkeitszeitpunkt (durch Tests belegt, nicht durch Benches).

Nebenbefund: `append_history_entry` wartet nicht auf den JetStream-Ack — History-Einträge können bei NATS-Problemen unbemerkt verloren gehen. Gehört zu P2/F3.

#### 4b — Fundament für Verteilung (Voraussetzung für 4c–4e)

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| F1 | `EventBus`-Port in `engine-core/src/port/` (Domain-Events publizieren, Commands konsumieren) — hexagonal wie `WorkflowPersistence` | High | Medium | Low | Heute ist NATS nur Speicher; es gibt keine Abstraktion für Messaging. Alle Broker-Integrationen hängen daran. |
| F2 | CloudEvents 1.0 als Event-Format (JSON + Binary Mode) | Medium | Low | Low | Interop mit Knative Eventing, Dapr, Kafka-Ökosystem. |
| F3 | Transactional Outbox: Zustandsänderung + Event atomar | High | Medium | Medium | Ohne Outbox gehen Events bei Crash zwischen Save und Publish verloren (vgl. S3). |
| F4 | Instanz-Ownership per Lease (NATS-KV-CAS mit Revision/TTL) oder Partitionierung nach Instanz-ID | High | High | High | = S1 (Multi-Node). Ohne Ownership kein horizontales Skalieren, kein Serverless. |
| F5 | Lazy Hydration: Instanzen bei Bedarf laden statt „Restore all“ beim Start | High | Medium | Medium | `StartupCoordinator` lädt alles in RAM → lange Kaltstarts, Speicher wächst mit Instanzzahl. |
| F6 | OpenTelemetry-Tracing mit W3C-`traceparent`-Propagation über REST, NATS, Kafka | Medium | Medium | Low | Steht bereits in Phase 2; Pflicht für Mesh-/Microservice-Betrieb. |

#### 4c — Native Messaging: NATS, Redpanda, Kafka

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| N1 | External Tasks nativ über NATS JetStream: Work-Queue-Stream pro Topic (`bpmn.tasks.{topic}`), Pull-Consumer = Worker. Mapping: `AckWait` = Lock-Dauer, `ack` = complete, `nak`/`term` = failure, `in_progress` = extendLock | High | Medium | Medium | Push-nativ, ohne HTTP-Polling; Lastverteilung und Redelivery übernimmt JetStream. REST bleibt parallel bestehen. |
| N2 | Message-Korrelation über Subjects (`bpmn.messages.{name}`) + Request/Reply-API als NATS-Micro-Service | Medium | Medium | Low | Latenzarmer Service-zu-Service-Zugang ohne HTTP. |
| N3 | Neues Crate `eventbus-kafka`: Kafka-API-Adapter, deckt Kafka **und** Redpanda ab (Redpanda ist Kafka-API-kompatibel). Topics: `bpmninja.events`, `bpmninja.tasks.{topic}`, `bpmninja.commands`; Partitionierung nach Instanz-ID für Reihenfolge | High | High | Medium | Bibliothekswahl entscheiden: `rdkafka` (ausgereift, C-Abhängigkeit librdkafka — Windows/`deny.toml` prüfen) vs. `rskafka` (pure Rust, weniger Features). |
| N4 | Deklarative Bindings: Message-Start-/Catch-Events direkt aus Kafka-/NATS-Topics (BPMN-Extension-Attribut) | Medium | Medium | Low | Prozesse ohne Glue-Code an Event-Streams hängen. |
| N5 | NATS-Transport im TypeScript-External-Task-Client | Medium | Medium | Low | Worker profitieren direkt von N1. |
| N6 | Transport-Benchmarks (REST vs. NATS vs. Kafka) im P0-Harness | Medium | Low | None | Belegt die Geschwindigkeitsaussagen. |

#### 4d — Microservices & Dapr

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| D1 | `persistence-dapr`: `WorkflowPersistence` über die Dapr State API (gRPC zum Sidecar) | High | High | Medium | Öffnet ~30 State Stores (Redis, Postgres, Cosmos DB …). Achtung: Prefix-Scans/Queries nicht in jedem Store → Index-Keys oder Dapr Query API nötig. |
| D2 | Dapr Pub/Sub als `EventBus`-Adapter (F1) | High | Low | Low | Kafka/NATS/Redpanda/Service Bus per Component-YAML statt Code. |
| D3 | Service-Task-Connector „Dapr Service Invocation“ (`appId`/`method` als Extension-Attribut) inkl. Dapr-Resiliency-Policies | High | Medium | Low | Direkter Aufruf von Microservices als Alternative zu Fetch-and-Lock. |
| D4 | Dapr Bindings als Trigger für Start-Events (Cron, Kafka, HTTP …) | Medium | Medium | Low | Einheitliche Trigger-Schicht. |
| D5 | Spike: Prozessinstanz als **Dapr Actor** (turn-based Concurrency, Placement über Nodes, Reminders für Timer) | High | High | High | Könnte F4/S1 (Ownership, Multi-Node) und P5 (Timer) gemeinsam lösen. Erst als Spike mit Benchmark bewerten. |
| D6 | Rollen statt Zerschlagung: ein Binary, per `ROLE=api,executor,timer,history` deploybar (Modular Monolith); History/Query als CQRS-Read-Model | Medium | Medium | Medium | Echte Zerlegung in viele Services kostet Netzwerk-Hops und widerspricht dem Performance-Ziel; Rollen erlauben trotzdem getrenntes Skalieren. |
| D7 | gRPC-API (tonic) parallel zu REST | Medium | Medium | Low | Effizienter für Service-zu-Service und Mesh-L7-Routing. |
| D8 | Referenz-Setup: Dapr-Components, docker-compose und Kubernetes-Manifeste, Beispiel-Worker | Medium | Low | None | Developer Experience. |

#### 4e — Serverless & Service Mesh

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| C1 | Kaltstart optimieren: Startzeit messen, musl-/distroless-Image, Lazy Hydration (F5) | High | Medium | Low | Voraussetzung für Scale-to-Zero. |
| C2 | KEDA-Scaler auf JetStream-Consumer-Lag bzw. Kafka-Lag für Executor-Pods (Scale-to-Zero) | High | Low | Low | Ereignisgetriebenes Skalieren; setzt F4 + N1/N3 voraus. |
| C3 | Knative-Serving-Deployment für die API-Rolle | Medium | Low | Medium | Setzt zustandslose API-Rolle (D6) voraus. |
| C4 | Dauerhafte Timer außerhalb des Prozesses (dedizierte Timer-Rolle, Dapr Reminders oder NATS-Scheduling evaluieren) | High | Medium | Medium | Bei Scale-to-Zero darf kein Timer an einem laufenden Pod hängen. |
| C5 | FaaS-Connectoren: Service Tasks rufen AWS Lambda / Cloud Run / Knative Functions / OpenFaaS per HTTP + CloudEvents auf | Medium | Medium | Low | Serverless-Worker ohne eigenen Poll-Loop. |
| C6 | Mesh-Readiness: Helm-Chart, Graceful Drain für Sidecars, mTLS über Istio/Linkerd (optional SPIFFE), OTel (F6), HPA über Prometheus-Metriken | Medium | Medium | Low | Health/Ready existieren bereits; es fehlen Deployment-Artefakte. |
| C7 | Research: Scripts/Connectoren als WASM-Komponenten (wasmtime); `engine-core` für Edge (Spin/wasmCloud) | Low–Med | High | High | Erst nach 4a–4c; Tokio-Abhängigkeit im Kern ist das Hauptproblem. |

#### 4f — BPMN-2.0.2-Vollabdeckung (Vorschlag, 2026-10-12)

> Ziel: alle Elemente der BPMN-2.0.2-Spezifikation (OMG) — Flow Objects, Connecting Objects, Swimlanes, Artifacts, Data — werden **geparst**, und alle Elemente mit Ausführungssemantik werden **ausgeführt**. Rein visuelle Elemente (Lanes, Groups, Text Annotations, Associations) werden akzeptiert, beim Re-Export erhalten und im Modeler angezeigt, ändern aber den Ablauf nicht (so verlangt es auch die Spezifikation).
> Hinweis: Die oft zitierte Zahl „116 Elemente“ ließ sich nicht belastbar gegen die Spezifikation prüfen; maßgeblich ist die Element-Matrix unten, abgeleitet aus dem BPMN-2.0.2-Metamodell und dem aktuellen Parser (`bpmn-parser/src/models.rs`, `parser.rs`).

**Ist-Stand (v0.7.24):** 29 ausführbare `BpmnElement`-Varianten (Start/End inkl. Timer, Message, Error, Escalation, Compensation, Terminate; User/Service/Script/Send-Task, Receive/Manual/BusinessRule gemappt; XOR/AND/OR/Event-based/Complex-Gateway; Timer-/Message-Catch; Boundary Timer/Message/Error/Escalation/Compensation; Call Activity; Embedded Sub-Process (geflattet); Event-Sub-Process für Timer/Message/Error). **Fehlend:** Signal-, Conditional-, Link-, Cancel-, Multiple-/Parallel-Multiple-Events; Transaction- und Ad-hoc-Sub-Process; Multi-Instance (wird abgelehnt) und Standard-Loop; Data Inputs/Outputs, Data Associations, Properties; mehrere Pools mit Message Flows; Laufzeit-Scopes für Sub-Prozesse (Boundary-Events auf Sub-Prozessen wirken wegen Flattening nicht spezifikationsgemäß).

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| X0 | Element-Matrix + Konformitäts-Testsuite | High | Medium | None | Jede BPMN-2.0.2-Elementart mit Status (ausgeführt / akzeptiert / abgelehnt) als Tabelle in Knowledge Base + README; Parse-Tests gegen die BPMN-MIWG-Referenzmodelle (Model Interchange Working Group); fehlende Elemente müssen beim Deploy **klar abgelehnt** statt still ignoriert werden. |
| X1 | Signal-Events: Start, Intermediate Catch/Throw, Boundary (unterbrechend/nicht), End; Broadcast an alle wartenden Instanzen; `POST /api/signal` | High | Medium | Medium | Häufig genutzt; heute nicht parsebar. |
| X2 | Conditional Events: Start, Intermediate, Boundary, Event-Sub-Process; Neubewertung bei Variablenänderung | Medium | High | Medium | Braucht einen Trigger bei jeder Variablenänderung (`update_instance_variables`, Task-Abschluss). |
| X3 | Link Events (Intermediate Throw/Catch, „Sprungmarke“) | Low | Low | Low | Beim Parsen in direkte Sequence Flows auflösen. |
| X4 | Multi-Instance parallel + sequenziell (`loopCardinality`, `collection`/`elementVariable`, `completionCondition`) für Tasks, Sub-Process, Call Activity | High | High | Medium | `NextAction::MultiInstanceFork/Next` existieren bereits, werden aber nie erzeugt; heute Deploy-Fehler. |
| X5 | Standard-Loop (`standardLoopCharacteristics`: `loopCondition`, `testBefore`, `loopMaximum`) | Medium | Medium | Low | Wiederholung ohne explizite Gateway-Schleife. |
| X6 | Laufzeit-Scopes für Sub-Prozesse (Anpassung ADR-003) | High | High | High | Voraussetzung für spezifikationsgemäße Boundary-Events auf Sub-Prozessen, Transaction/Cancel und Kompensation pro Scope. Flattening bleibt als Optimierung für scope-lose Sub-Prozesse. |
| X7 | Transaction-Sub-Process + Cancel End/Boundary Event (Cancel löst Kompensation aus) | Medium | High | Medium | Setzt X6 voraus; ergänzt G1 (Saga). |
| X8 | Ad-hoc-Sub-Process (Aktivitäten in beliebiger Reihenfolge, `completionCondition`, `ordering`) | Low–Med | Medium | Medium | Setzt X6 voraus. |
| X9 | Event-Sub-Process vollständig: Error/Escalation/Compensation/Signal/Conditional-Start, unterbrechend und nicht unterbrechend | Medium | Medium | Medium | Heute nur Timer/Message/Error. |
| X10 | Multiple / Parallel-Multiple Events (mehrere Event-Definitionen an einem Event) | Low | Medium | Low | Selten, aber Teil der Spezifikation. |
| X11 | Daten: `dataObject`/`property` als Prozessvariablen mit Defaults, Data Input/Output Associations, `ioSpecification`, Camunda `inputOutput`-Mappings für Tasks und Call Activities | High | Medium | Medium | Heute nur ein globaler Variablen-Scope; Mappings sind für Camunda-Kompatibilität wichtig. |
| X12 | Kollaboration: mehrere Pools (jeder ausführbare Prozess wird deployt), Message Flows zwischen Pools → engine-interne Zustellung | Medium | Medium | Medium | Heute wird nur der erste Prozess deployt. |
| X13 | Instanziierende Receive Task, Event-based Gateway mit `instantiate` und Typ `Parallel` | Low | Low | Low | Kleine Lücken im Gateway-/Task-Mapping. |
| X14 | Business Rule Task mit DMN-Entscheidungstabellen (eigener DMN-1.x-Evaluator mit FEEL-Teilmenge) | Medium | High | Medium | Hängt mit S4 (FEEL) zusammen; heute nur als Service Task. |
| X15 | Swimlanes & Artefakte: Pools/Lanes, Groups, Text Annotations, Associations, Data Stores werden akzeptiert, beim Export erhalten, im Modeler angezeigt; optional Lane → Kandidatengruppe für User Tasks | Low–Med | Low | Low | Nicht ausführungsrelevant, darf aber nicht verloren gehen. |
| X16 | User-Task-Details: `candidateUsers`/`candidateGroups`, Fälligkeit, Priorität, `formKey`; Claim/Unclaim-API | Medium | Medium | Low | Camunda-Kompatibilität für Task-Listen. |
| X17 | Modeler (desktop-tauri): Properties-Panel für alle neuen Attribute; Playwright-Tests je Element | Medium | Medium | Low | Jede Engine-Erweiterung braucht die passende UI. |

**Empfohlene Reihenfolge:** X0 (+ V1-Modellprüfung) → X3, X13, X15 (klein) → X1 Signal → X4 Multi-Instance + X5 Loop → X11 Daten-Mappings → X9 Event-Sub-Process → X2 Conditional → X12 Kollaboration → X6 Laufzeit-Scopes (ADR) → X7 Transaction/Cancel, X8 Ad-hoc → X10 → X14 DMN, X16 → X17 laufend parallel.

**Abnahmekriterium je Element:** Parser-Test, Engine-Test (inkl. Persistenz-Roundtrip und Restart), E2E-Test über REST, Modeler-Unterstützung, Eintrag in der Element-Matrix.

#### Bewertung externer Architekturvorschläge (2026-10-12)

Quelle: eingebrachte Recherche (Rust-BPMN-Landschaft, Kafka/Redpanda-Saga-Log, WS-BPEL, AOT-Kompilierung nach C/FFI, Zeebe). Bewertet gegen die Messwerte aus 4a.

**Kernbefund aus den Messungen:** Gateway-Routing im Speicher kostet ~14 µs pro Instanz (XOR), ein NATS-Roundtrip ~200 µs, ein Script-Task ~0,6 ms. Die Engine ist durch I/O und Script-Setup begrenzt, **nicht** durch das Interpretieren des Graphen. Optimierungen am Graph-Interpreter (bis hin zu nativem Code) können deshalb nur Mikrosekunden auf einem Millisekunden-Pfad sparen.

| Vorschlag | Bewertung | Entscheidung |
|-----------|-----------|--------------|
| Kafka/Redpanda als append-only Event-/Saga-Log (Key = Instanz-ID, idempotenter Producer, `acks=all`) + materialisierte Sicht | Passt zu F1/F3/N3; Partitionierung nach Instanz-ID garantiert Reihenfolge je Instanz | **Übernehmen** als Ausprägung von N3: History/Domain-Events → Kafka/Redpanda; operativer Zustand bleibt in NATS KV (bzw. Q1) |
| Kafka als *alleiniges* System of Record mit Replay (volles Event Sourcing des Kerns) | Großer Umbau des Persistenzmodells (= S3) | **Später/optional** — nur wenn Audit-Replay gefordert ist |
| Saga-Kompensation (LIFO) | BPMNinja hat Compensation-Events mit LIFO bereits (`CompensationThrow/End`, `BoundaryCompensationEvent`) | **Ergänzen** statt neu bauen → G1 |
| Externe Read-DB (PostgreSQL / ScyllaDB) | NATS KV kann nicht abfragen; History-/Archiv-Suche filtert heute im Speicher | **Postgres-Adapter vorschlagen** (Q1); Scylla erst bei nachgewiesenem Bedarf |
| WS-BPEL-2.0-Unterstützung | Machbar über das bestehende Flattening: `sequence` → Flows, `flow` → Parallel-Gateway, `if` → XOR, `while` → XOR-Schleife, `pick` → Event-Gateway, `invoke` → Service Task, `scope`/`compensationHandler` → Compensation | **Entscheidung offen** — nur bei konkretem Bedarf (Legacy-SOAP-Landschaft) → B1 |
| AOT: BPMN/BPEL → C-Quelltext → `.so`/`.dll`, Ausführung per FFI mit Raw-Pointern und pthreads | Spart Mikrosekunden auf einem I/O-gebundenen Pfad. `unsafe`-Code im Prozess hebelt die Rhai-Sandbox und die Crash-Sicherheit aus; ein Plugin-Absturz reißt den Knoten mit. Braucht eine C-Toolchain zur Deploy-Zeit (serverless-feindlich, plattformabhängige Artefakte). Die Beispiele enthalten Speicherlecks (`mem::forget`) und Datenrennen (parallele Threads mutieren denselben Kontext) | **Abgelehnt.** Stattdessen: vorkompilierte Scripts/Conditions (P6/P7), kompilierter In-Memory-Ausführungsplan (E2), WASM-Komponenten für schnellen *sandboxed* Fremdcode (C7), In-Process-Task-Handler in sicherem Rust (E1) |
| Rayon-Pool für CPU-lastige Tasks | Nur relevant, wenn Tasks im Prozess laufen | Mit E1 umsetzen |
| Camunda-7-External-Task-Worker / Camunda-8 (Zeebe) | Eine Camunda-7-kompatible REST-Fassade lässt vorhandene Camunda-7-Worker unverändert laufen | **Übernehmen** als K1; Zeebe-Bridge (K2) nur bei Bedarf |
| Model-Checking (Ideen aus `rust_bpmn_analyzer`: Deadlocks, Livelocks, Safeness) | Unstrukturierte Inclusive-Joins können heute zur Laufzeit hängen (siehe engine-core Gotchas) | **Übernehmen** als Deploy-Validierung V1 |
| Deterministisches Replay (Idee aus `bpm-engine`) | Teil von F3/S3 | Mit F3 |

Neue Aufgaben aus der Bewertung:

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| E1 | In-Process-Task-Handler: `register_task_handler(topic, Arc<dyn TaskHandler>)` | High | Medium | Low | Service Tasks laufen direkt im Engine-Prozess ohne Fetch-and-Lock-Roundtrip — der sichere Weg zu „nativer“ Geschwindigkeit für eingebettete Nutzung. CPU-lastige Handler auf eigenem Pool (`spawn_blocking`/Rayon). |
| E2 | Kompilierter Ausführungsplan beim Deploy (Knoten/Flows als Index-Arrays statt String-HashMap-Lookups pro Schritt) | Medium | High | Medium | Erst nach P6/P2 messen, ob der Interpreter überhaupt noch relevant ist. |
| V1 | Deploy-Validierung: unerreichbare Knoten, fehlendes End-Event, unstrukturierte Inclusive-Joins, offensichtliche Deadlock-Muster | High | Medium | Low | Fehler beim Deploy statt hängender Instanzen in Produktion. |
| K1 | Camunda-7-kompatible REST-Fassade (`/engine-rest/external-task/fetchAndLock`, `complete`, `failure`, `bpmnError`, `extendLock`) | High | Medium | Low | Migrationspfad: bestehende Camunda-7-Worker funktionieren ohne Codeänderung. |
| K2 | Zeebe-/Camunda-8-Bridge | Low | High | Medium | Nur bei konkretem Bedarf. |
| G1 | Saga-Erweiterung: Kompensation für External Tasks (`compensationTopic`), Tests für Kompensation über Call-Activities und parallele Zweige | Medium | Medium | Medium | Baut auf vorhandener LIFO-Kompensation auf. |
| B1 | WS-BPEL-2.0-Frontend (Parser → `ProcessDefinition`) | offen | High | Medium | Nur bei Bedarf; Abbildung siehe Tabelle oben. |
| Q1 | `persistence-postgres` (operativer Zustand + Abfragen/Read-Model) | Medium | High | Medium | Echte Abfragen für History/Archiv statt In-Memory-Filter. |
| J1 | JSON-Definitionsformat: `ProcessDefinition` als JSON deployen/exportieren (neben BPMN-XML) | Low–Med | Low | Low | Code-first-Definitionen ohne Modeler; Idee aus `bpmn-engine`. |
| SIM1 | Prozess-Simulation (Pfadwahrscheinlichkeiten, Durchlaufzeiten) | Low | High | Low | Optional, z. B. auf Basis von `ebi_bpmn`. |

**Rust-BPMN-Werkzeuge** (geprüft am 2026-10-12 auf crates.io/GitHub: Existenz, Lizenz, Aktivität):

| Werkzeug | Befund | Einbindung in BPMNinja |
|----------|--------|------------------------|
| `rust_bpmn_analyzer` (GitHub, MIT, aktiv; Library-Crate `core`, nicht auf crates.io) | Echter Model-Checker: Safeness, Option to Complete, Proper Completion, No Dead Activities; Partial Order Reduction | **Als Abhängigkeit** (gepinnte Git-Revision, `deny.toml` `allow-git`) für V1: Prüfung beim Deploy in `spawn_blocking` mit Timeout, Modus per Env `MODEL_CHECK=off|warn|enforce` (Default `warn`) |
| `bpmncheck` (crates.io, Apache-2.0, 1 Stern) | BPMN → Petri-Netz → Model-Checking | Nicht eingebunden: überschneidet sich mit `rust_bpmn_analyzer`, zusätzlicher XML-/Petri-Stack |
| `snurr` (crates.io, MIT) | Prozess-Runner mit Closures als Task-Handlern, Thread-Parallelität | Kein Fremd-Runtime im Kern; Idee übernommen als **E1** (In-Process-Task-Handler) |
| `bpm-engine` (crates.io, MIT) | Deterministisches Replay, Crash-Sicherheit, Saga | Ideen übernommen in **F3** (Outbox/Replay) und **G1** (Saga) |
| `bpmn-engine` (crates.io 0.1.0, Apache-2.0) | Repository-Link liefert 404, kaum Nutzung | Nicht eingebunden; Idee „JSON-Definitionsformat“ als **J1**: Deploy/Export von `ProcessDefinition` als JSON neben BPMN-XML |
| `ebi_bpmn` (crates.io, MIT OR Apache-2.0, aktiv) | Parser/Writer/Executor inkl. stochastischem BPMN | Nicht im Kern; Kandidat für **SIM1** (Prozess-Simulation mit Wahrscheinlichkeiten/Durchlaufzeiten) nach Bedarf |

Offene Entscheidungen (Produktinhaber): Ziel-Größenordnung (gleichzeitig aktive Instanzen, Schritte/s), Bedarf an WS-BPEL, Request/Response- vs. choreografierte Microservices, Postgres als zusätzliches Backend.

#### Empfohlene Reihenfolge

1. **4a**: ~~P0 → P1, P3, P4, P5 (Quick Wins)~~ ✅ 2026-10-12 → P2, P6, P7 → P8–P10
2. **Aus der Bewertung** (unabhängig, hoher Nutzen): V1 (Deploy-Validierung), K1 (Camunda-7-Fassade), E1 (In-Process-Handler); B1/K2/Q1 nach Entscheidung
2b. **4f BPMN-Vollabdeckung**: X0 + V1 → X3, X13, X15 → X1 → X4 + X5 → X11 → X9 → X2 → X12 → X6 → X7, X8 → X10 → X14, X16 (X17 laufend)
3. **4b**: F1 + F2 → F6 → F3 → F4 + F5 (gemeinsam mit S1)
4. **4c**: N1 → N3 → N2, N4–N6
5. **4d**: D2 → D3 → D1 → D5-Spike → D6–D8
6. **4e**: C1 → C4 → C2 → C3, C5, C6 → C7

---

## Weitere Audit-Befunde (Kontext)

### Stärken (kurz)

- Echte hexagonale Architektur (`WorkflowPersistence`-Port)
- Klares Token-Modell (`NextAction`, closed `BpmnElement`)
- Breite BPMN-Abdeckung für schlanke Engine
- Retry-Queue, SSE, Prometheus, Fuzz, Dependabot, Knowledge Base
- External-Task-Client mit guter Testabdeckung

### Score-Dimensionen (Audit)

| Dimension | Score |
|-----------|-------|
| Architecture & Design | 8.0 |
| Code Quality & Maintainability | 7.5 |
| Performance & Scalability | 6.5 (Single-Node OK) |
| Security (exposed) | 3.5 |
| Security (local/dev tool) | 6.5 |
| Testing Strategy | 8.0 (NATS-Adapter dünn) |
| Error Handling / Observability | 8.0 |
| Dependency / Tech Stack | 7.5 |
| Documentation / DX | 7.5 |

### Performance-Hotspots

1. ~~`fetch_and_lock_service_tasks` — Full-Scan `pending_service_tasks`~~ ✅ `TopicIndex` (M4)
2. ~~`process_timers` — Full-Scan aller Timer pro Intervall (default 1s)~~ ✅ `TimerDueIndex` (M4); fester Takt bleibt → P5
3. `list_instances` / Startup-Restore — große Mengen in Memory (→ Phase 4b F5)
4. History-Queries — Filter/Sort in Memory
5. ~~Unbounded Retry-Queue~~ ✅ bounded (M3)
6. SSE Broadcast Capacity 256 — langsame Clients verlieren Events (by design)

### Was das Audit nicht vollständig bewiesen hat

- Load-Benchmarks (Benches existieren, nicht neu gelaufen)
- Vollständiges Desktop-Tauri CSP/Capability-Review
- Live Multi-Process-Race gegen NATS
- Feldgenaue OpenAPI-Vollständigkeit jeder Route
- Playwright-E2E in CI auf allen Branches

---

## Was bewusst nicht in diesem Dokument steht

- Feature-Wünsche ohne technischen Schuld-/Risikobezug (außer Roadmap-Verweise)
- Reine Lob-Abschnitte im Detail — siehe Audit-Stärken oben
- Implementierungsdetails der Lösungen (Backlog, keine Spec)

---

## Appendix — Größen-Snapshot

| Komponente | Ca. Größe |
|------------|-----------|
| engine-core | ~17.4k LOC / 56 rs files |
| engine-server | ~5.3k LOC / 33 files |
| bpmn-parser | ~2.5k LOC |
| persistence-nats | ~1.3k LOC |
| persistence-memory | ~0.4k LOC |
| agent-orchestrator | ~0.1k LOC |
| desktop-tauri (src) | ~10k TS/TSX/RS |
| external-task-client | ~2–2.6k TS |
| Größte Testdatei | `unit_tests.rs` ~5076 Zeilen |
| Größte Prod-Dateien | `trait_impl.rs` ~984, `parser.rs` ~902, `instance_ops.rs` ~848 |

---

**Nächster sinnvoller Schritt:** Phase 0–1 sind abgeschlossen, Phase 2 bis auf OpenTelemetry und `agent-orchestrator`. Als Nächstes Phase 4a (Performance-Quick-Wins, siehe unten) — sie bringt den größten Hebel bei kleinem Risiko und liefert die Messbasis für alles Weitere.
