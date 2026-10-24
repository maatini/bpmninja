# BPMNinja — Verbesserungsplan (2026-10-10)

Persistierte Kopie des Analyse-Plans. Quelle: Session-Plan vom 2026-10-10.

Analyse mit sechs parallelen Sub-Agenten (Engine, Parser, Server/Security, Persistence, Desktop, Quality/CI). Code-Stand: `main` = `c599b51`, Crates/Desktop **0.7.23**. Letzter Produkt-Commit: `56c2915` (Docker native Multi-Arch). Seitdem nur Quality-Metrics-Bot und Dependabot.

**Auftrag der Analyse-Session:** Analyse + priorisierter Plan. Kein Code, kein Commit.

---

## Snapshot

BPMNinja bleibt eine disziplinierte, hexagonal geschnittene BPMN-2.0-Engine mit Token-Ausführung, NATS, Axum, Tauri und Camunda-ähnlichem fetch-and-lock. Phase 0 aus `improvements.md` (Rhai-Memory, Upload-Limit, `REQUIRE_NATS` fail-closed, bounded Retry, Log-Hygiene, Content-Disposition) ist **erledigt und bleibt zu**.

Die Oktober-Liste (Auth, CORS, Camunda-Attribute, Inclusive-Join, Pagination, Hygiene) gilt weiter. Die heutige Tiefe zeigt **zusätzliche Engine-Korrektheitslücken**, die echte Diagramme totlegen oder falsch laufen lassen. Die sind der größte Hebel.

| Sicht | Juli-Audit | Heute |
|-------|------------|--------|
| Architektur / Hexagon | 8.0 | 8.0 |
| Engine-Korrektheit | ~8.0 | **~6.5** (XOR-Join, Inclusive-Join, Timer, Incidents, MI tot) |
| Deploy als LAN-Tool | ~6.5 | ~6.5 |
| Deploy internet-exponiert | ~3.5 | **3.0** (CORS Any + Bind 0.0.0.0 = Drive-by) |
| Tests (Menge) | 8.0 | 8.0 |
| Tests (kritische Pfade) | — | **6.0** (Join/Timer/Incident/Restore dünn) |
| Docs/DX | 7.5 | 7.0 (OpenAPI 0.7.19, Agent-Spec 23 vs 29) |
| **Gesamt als schlanke Engine** | **7.2** | **6.8** |

Mutation 68.2 % (381/177), Fuzz 9/9, CI fmt+clippy+workspace-tests+Playwright (gemockt). Keine offenen Produkt-Issues; offen nur Dependabot #341 (croner) und #349 (typescript-eslint).

---

## Frozen — nicht anfassen

- Hexagonale Architektur, Token-SSOT, Subprocess-Flattening, Rhai, Camunda fetch-and-lock
- Phase 0 (`REQUIRE_NATS`, bounded Retry, Rhai-Caps, `MAX_UPLOAD_BYTES`)
- Rust-Pin 1.99.0 / Resolver 3 (`8f66c88`)
- Docker native Multi-Arch (`56c2915`)
- OIDC / Multi-Node / FEEL / DMN / Batch-Ops = Roadmap, kein Quick-Win

---

## Empfohlene Reihenfolge

Vier Wellen. Jede Welle ist ein eigener Auftrag. Nicht alles parallel.

### Welle 1 — Engine-Korrektheit (höchster Hebel)

Echte BPMN-Diagramme totlocken oder falsch ausführen. Kleine Diffs, hohe Wirkung.

| ID | Problem | Ort | Effort |
|----|---------|-----|--------|
| E1 | Inclusive-Join wartet AND-style auf `incoming_flow_count`; partieller Split totlockt | `gateway.rs:78-83`, `parallel.rs` | M |
| E2 | XOR überspringt unconditional Flows → Standard-XOR-**Merge** wirft `NoMatchingCondition` | `gateway.rs:46-64` | S |
| E3 | Recurring-Timer: `remaining_repetitions` immer `None` → `R3/...` läuft endlos; nach Fire kein `persist_timer` | `timer_processor.rs:235-256`, Handler setzen `None` | S |
| E4 | Zwei Event-Based-Timer gleicher Tick: `Timer disappeared` bricht **ganze** `process_timers`-Runde | `timer_processor.rs:27-34` | S |
| E5 | Incidents (`retries <= 0`) werden von `fetch_and_lock` wieder ausgegeben → Fail-Loop | `service_task.rs:54-86` | S |
| E6 | Unbehandelter `bpmnError` löscht Task, Token bleibt in `WaitingOnServiceTask` | `service_task.rs:661-677` | S |
| E7 | Boundary-Timer/-Messages an User/Service/Call nur DashMap, **nicht** persistiert → nach Restart tot | `handlers/tasks.rs` vs `persist_timer` | S |

**Tests dazu (Pflicht):** Inclusive Split 1-von-N + Join; XOR-Merge ohne `@default`; `R3/PT…` stoppt nach 3; zwei Event-Based-Timer gleiche Duration; fetchAndLock skippt Incidents; bpmnError ohne Boundary → Incident; Boundary-Timer Restore.

### Welle 2 — Camunda-/Modeler-Interop

| ID | Problem | Effort |
|----|---------|--------|
| P1 | Parser liest `data-assignee`/`data-topic`, ignoriert `camunda:assignee`/`camunda:topic` | S |
| P2 | Desktop hat **kein** Assignee-Properties-Provider; KB-Claim „Desktop schreibt `data-assignee`“ ist falsch | S |
| P3 | `sendTask` liest nur nested `messageEventDefinition`, nicht `@messageRef` | S |
| P4 | Leeres `<timerEventDefinition/>` → `Duration(0)` (sofort feuern) | S |
| P5 | Timer-Felder als `String`; bpmn-js `xsi:type` FormalExpression kann Deploy killen | S |
| P6 | Stille Mappings (`receiveTask`/`manualTask`/`businessRuleTask`/Catch-ohne-Trigger) → ServiceTask-Queue | M |
| P7 | Flatten droppt `sendTask`/Inclusive/EventBased/Intermediate im Subprozess → `NoSuchNode` | M |

P1+P2 zusammen: sonst bleibt Camunda-XML und Desktop-XML unterschiedlich kaputt.

### Welle 3 — Production-Bar (LAN zuerst)

Internet-Exposure bleibt **Blocker** bis Auth+TLS. Für LAN reicht Welle 3.

| ID | Problem | Effort |
|----|---------|--------|
| S1 | CORS `allow_origin(Any)` + Bind `0.0.0.0` = Drive-by aus dem Browser | S |
| S2 | API-Auth v1 (Bearer/API-Key); Client-Hook `Authorization` vorbereiten | M |
| S3 | Body-Limit eine Zahl: `DefaultBodyLimit` 5 MiB vs `MAX_XML_BYTES` 10 MiB vs OpenAPI „10 MB“ | S |
| S4 | Compose: NATS 4222/8222 und cobra-nats :3000 ohne Credentials am Host | M |
| S5 | `lockDuration`: OpenAPI/Camunda **ms**, Code **Sekunden** (`TimeDelta::seconds`) | S |
| S6 | External-Task-Client Default `localhost:8080`, Engine `:8081` | S |

Ohne S1+S4 ist auch „nur lokal“ unsicher, sobald ein Browser andere Origins lädt.

### Welle 4 — Betrieb / Scale / Restore

| ID | Problem | Effort |
|----|---------|--------|
| R1 | `list_instances()` klont alles; API ohne Pagination | M |
| R2 | Topic-/Timer-Full-Scan (M4) | M |
| R3 | Restore re-parst XML, ignoriert Definitions-KV; `keys()`-Fehler → leere Liste | M |
| R4 | Deploy: XML-Save-Fehler nur geloggt, HTTP 200 | S |
| R5 | M2 war überzeichnet: 4 Adapter-Tests, kein `StartupCoordinator`, kein Partial-Failure, History-E2E skippt in CI | M |
| R6 | Call-Activity: Spawn-Fehler verschluckt; parallele Calls werden nie resumed | M |
| R7 | Multi-Instance: Parser ja, Runtime erzeugt `MultiInstanceFork` **nirgends** | L oder S (explizit unsupported) |

### Welle 5 — Hygiene / DX (jederzeit, niedrige Priorität)

- OpenAPI `0.7.19` → 0.7.23; TypeSpec in CI kompilieren; Pages-Drift prüfen
- `cargo deny` in CI (M6)
- Agent-Spec 23 vs 29 Elemente; Graphify-Report 2026-04-17
- Tote Dateien `test_cron.rs`, `engine-core/fix_tests.py`; stale `TASK_MEMORY.md` / `implementation_plan.md`
- DialogDescription in 5 Dialogen; `index.html` Title `temp`
- Desktop: CSP `null`, Dummy-Updater-Pubkey, Identifier `com.minibpm.dev`, SSE-URL nach Settings-Wechsel eingefroren
- `unit_tests.rs` 5154 Zeilen splitten (M5)
- Mutation-Job `continue-on-error`; Docker-Publish ohne `needs: CI`
- agent-orchestrator ohne `example.bpmn` — archivieren, nicht ausbauen

---

## Was die Sub-Agenten zusätzlich gefunden haben (kurz)

### Engine (neu gegenüber HANDOFF)

- XOR-Merge ohne `@default` failt (E2) — in echten bpmn-js-Diagrammen der Normalfall.
- Recurring-Timer ignorieren `R3` (E3) und werden nach Fire nicht persistiert.
- Event-Based: ein fehlender Geschwister-Timer killt die ganze Timer-Runde (E4).
- Incidents sind kein Gate für fetch-and-lock (E5).
- `bpmnError` ohne Boundary → Zombie-Token (E6).
- Multi-Instance und SendTask-Korrelation sind tot bzw. nur Log.
- Compensation: `complete_service_task` registriert keinen Handler.
- EscalationEnd sucht keinen Boundary; scoped Finder ist `dead_code`.
- Call-Activity parallele Instanzen + Spawn-Fehler (R6).
- `JoinBarrier.insert` überschreibt; Complex fehlt in `same_gateway_type`.

### Parser

- Desktop schreibt Topic als `data-topic`, **kein** Assignee-Feld (P2).
- Camunda-Modeler-XML deployed oft **erfolgreich** mit falscher Semantik (unassigned, Topic=Node-Id).
- Event-Subprozesse: `triggeredByEvent` gelesen, `ScopeEventListener` nie gebaut.
- Conditions: Desktop erlaubt Rhai-Script; Engine nur `==/>/<`/`truthy`, kein `${}`-Strip.

### Persistence

- Tokens leben in der Instance-JSON; Token-KV ist tot.
- Multi-KV-Writes nicht atomar → Orphans (Wait-State ohne Instance oder umgekehrt).
- History-Publish ohne `PublishAck`; History-Query Drain 100 ms kann abschneiden.
- Instance > 900 KiB: Persist skip, nur RAM.

### Desktop / Server

- SSE-Consumer friert Start-URL ein; Settings ändern REST, nicht SSE.
- Playwright `app.spec.ts` 2452 Zeilen, mockt Incidents/SSE/Settings nicht; Fixture nutzt `camunda:assignee`.
- Prometheus-Cardinality: path-params außer UUID nicht normalisiert.
- OpenAPI: PUT-Variablen-Shape, 200 vs 204, XML-Response-Typ, `lockDuration`-Einheit.

---

## Vorgeschlagener erster Auftrag (nach GO)

**Welle 1, Slice A (S-Items, ~1–2 Tage):** E2, E3, E4, E5, E6, E7 + die zugehörigen Tests.

Danach Slice B: E1 Inclusive-Join (M, braucht Barrier-Semantik).

Welle 2/3 nur nach expliziter Wahl. Hygiene-Rest (Welle 5) als Füllauftrag, wenn kein Produkt-Schritt gewählt wird.

---

## Nicht Teil dieses Plans

- Rust 1.95–1.99 erneut recherchieren
- Phase 0 neu aufrollen
- `async-trait` / Tauri Edition 2024 (eigene PRs, HANDOFF 8–9)
- `HANDOFF.md` committen (nur `.gitignore`-Zeile, wenn gewünscht)
- Dependabot-PRs mergen (eigenes „führe die PRs aus“)
