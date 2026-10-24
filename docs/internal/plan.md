# Restplan — nach Welle 1–5 (`improve1026.md`)

> **Historisch (2026-10-10).** Ersetzt durch `HANDOFF.md`; offene Punkte stehen dort. Nicht ausführen.

Stand: 2026-10-10 (Handoff-Refresh). Quelle: `improve1026.md`. **Welle 1–5 sind lokal fertig, nicht committed.** Lokal 1 hinter origin/main (`069af0b` quality-metrics).

Nächste Session: `HANDOFF.md` → `AGENTS.md` → dieses File. Commit-Schnitt crate-basiert in `HANDOFF.md` (Wellen in engine-core/parser/server vermischt).

---

## Welle 1 — erledigt (uncommitted)

Engine-Korrektheit E1–E7. Gate: `cargo test -p engine-core --lib` 233 passed; `cargo clippy -p engine-core -- -D warnings` clean.

| ID | Fix |
|----|-----|
| E2 | XOR nimmt unconditional Flows mit `target != default`; sonst Default; sonst `NoMatchingCondition` |
| E3 | `TimerDefinition::initial_remaining_repetitions()`; `R3` = 3 Fires (`Some(1)` letzter); `persist_timer` nach Reschedule |
| E4 | `process_timers`: fehlender Timer → `continue`, kein Abbruch der Runde |
| E5 | `fetch_and_lock` skippt `retries <= 0` |
| E6 | unhandled `bpmnError` → Incident (`retries=0`), Task bleibt; Boundary-Pfad unverändert |
| E7 | Boundary-Timer/-Messages nach Insert persistiert (User/Service/Call) |
| E1 | Inclusive-Join: Barrier `expected_count` = genommene Pfade; Continue nur bei Inclusive-**Split** (`is_split_gateway`); nested inner-join überschreibt outer nicht |

Tests: `xor_merge.rs`, `recurring_and_event_timer.rs`, `service_incidents.rs`, `boundary_restore.rs`, `inclusive_join.rs` (inkl. Nested). Gotchas: `docs/knowledge-base/modules/engine-core/gotchas.md`, `bpmn-parser/gotchas.md`.

**Limit:** Unstructured Inclusive-Join ohne `find_downstream_join` fällt weiter auf `incoming_flow_count` zurück.

**Erstes GO der nächsten Session, falls gewählt:** Welle 1 committen (nicht in diesem File; User muss es sagen). `.gitignore`-Zeile `HANDOFF.md` nicht mitcommitten, außer explizit.

---

## Frozen (alle Wellen)

Hexagon, Token-SSOT, Subprocess-Flattening, Rhai, Phase 0, Rust 1.99.0 / Resolver 3, Docker native Multi-Arch. Kein OIDC/Multi-Node/FEEL/DMN/Batch. Kein `git push --force`. Kein Commit ohne Auftrag.

---

## Welle 2 — Camunda-/Modeler-Interop (lokal fertig, nicht committed)

P1–P7 umgesetzt 2026-10-10. Gate: `cargo test -p bpmn-parser --lib` 45 passed; `cargo clippy -p bpmn-parser -- -D warnings` clean.

P1+P2 zusammen, sonst bleiben Camunda-XML und Desktop-XML unterschiedlich kaputt.

| ID | Problem | Effort | Cluster |
|----|---------|--------|---------|
| P1 | Parser liest `data-assignee`/`data-topic`, ignoriert `camunda:assignee`/`camunda:topic` | S | Parser |
| P2 | Desktop hat kein Assignee-Properties-Provider | S | Desktop, mit P1 |
| P3 | `sendTask` liest nur nested `messageEventDefinition`, nicht `@messageRef` | S | Parser |
| P4 | Leeres `<timerEventDefinition/>` → `Duration(0)` | S | Parser |
| P5 | Timer-Felder als `String`; bpmn-js `xsi:type` FormalExpression kann Deploy killen | S | Parser |
| P6 | Stille Mappings (receive/manual/businessRule/Catch-ohne-Trigger) → ServiceTask-Queue | M | Parser |
| P7 | Flatten droppt sendTask/Inclusive/EventBased/Intermediate im Subprozess → `NoSuchNode` | M | Parser |

**Sub-Agenten (nach GO für Welle 2):**

1. **P1+P2** (`grok-4.6`) — `bpmn-parser` Camunda-Attribute + Desktop Properties-Provider. Dateien: `bpmn-parser/src/parser.rs`, `models.rs`, Desktop `properties/`.
2. **P3+P4+P5** (`grok-4.7-build-fast`) — sendTask `@messageRef`, leere Timer rejecten, FormalExpression.
3. **P6** (`grok-4.6`) — nach 1+2 oder sequentiell (gleicher Parser). Unbekannte Tasks rejecten oder explizit typisieren, nicht still queue-n.
4. **P7** (`grok-4.7`) — Flatten-Semantik, nach P3 (sendTask im Subprozess).

Return-Schema wie Welle 1: nur STATUS/IDS/FILES/TESTS/VERIFY/NOTES. `tests/mod.rs` nur Orchestrator.

---

## Welle 3 — Production-Bar (LAN) — lokal fertig, nicht committed

Internet-Exposure bleibt Blocker bis Auth+TLS. Für LAN reicht Welle 3. Ohne S1+S4 ist auch „nur lokal“ unsicher, sobald ein Browser andere Origins lädt.

Umgesetzt 2026-10-10: CORS-Allowlist + `BIND_ADDR`; `BPMNINJA_API_KEY` optional; Body-Limit 10 MiB; Compose NATS user/pass + Loopback-Ports; HTTP `lockDuration` ms; Client Default `:8081`.

| ID | Problem | Effort |
|----|---------|--------|
| S1 | CORS `allow_origin(Any)` + Bind `0.0.0.0` | S |
| S2 | API-Auth v1 Bearer/API-Key; Client `Authorization` | M |
| S3 | Body-Limit eine Zahl (5 vs 10 MiB vs OpenAPI) | S |
| S4 | Compose: NATS 4222/8222, cobra-nats :3000 ohne Credentials | M |
| S5 | `lockDuration`: OpenAPI/Camunda **ms**, Code **Sekunden** | S |
| S6 | External-Task-Client Default `:8080`, Engine `:8081` | S |

Cluster: S1+S3+S5 Server; S2 Server+Client; S4 Compose; S6 npm-Client. S2 nach S1.

---

## Welle 4 — Betrieb / Scale / Restore — lokal fertig, nicht committed

R7-Entscheidung: Parser reject, kein Runtime-MI. Timer-Due-Index nicht gebaut. R5 über R3/R4.

| ID | Problem | Effort |
|----|---------|--------|
| R1 | `list_instances()` klont alles; API ohne Pagination | M |
| R2 | Topic-/Timer-Full-Scan (M4) | M |
| R3 | Restore re-parst XML, ignoriert Definitions-KV; `keys()`-Fehler → leere Liste | M |
| R4 | Deploy: XML-Save-Fehler nur geloggt, HTTP 200 | S |
| R5 | M2 überzeichnet (kein StartupCoordinator, History-E2E skippt CI) | M |
| R6 | Call-Activity: Spawn-Fehler verschluckt; parallele Calls nie resumed | M |
| R7 | Multi-Instance: Parser ja, Runtime erzeugt `MultiInstanceFork` nirgends | L oder S (explizit unsupported) |

R6 ist Engine, kann unabhängig von R1–R5. R7 Entscheidung zuerst: implementieren vs. Unsupported dokumentieren.

---

## Welle 5 — Hygiene / DX — lokal fertig, nicht committed

Offen geblieben: M5 `unit_tests.rs` split, Desktop CSP/Identifier, Graphify committen, Lockfile-Bump der deny-Ignores.

- OpenAPI `0.7.19` → 0.7.23; TypeSpec in CI; Pages-Drift
- `cargo deny` in CI (M6)
- Agent-Spec 23 vs 29; Graphify-Report 2026-04-17
- Tote Dateien `test_cron.rs`, `engine-core/fix_tests.py`; stale `TASK_MEMORY.md` / `implementation_plan.md`
- DialogDescription in 5 Dialogen; `index.html` Title `temp`
- Desktop: CSP `null`, Dummy-Updater-Pubkey, Identifier `com.minibpm.dev`, SSE-URL nach Settings eingefroren
- `unit_tests.rs` splitten (M5) — Welle 1 hat bereits neue Testdateien neben der God-File
- Mutation-Job `continue-on-error`; Docker-Publish ohne `needs: CI`
- agent-orchestrator ohne `example.bpmn` — archivieren
- README Inclusive-Join-Deviation: KB ist neu, README kann noch AND-style sagen

---

## Nicht tun

- Rust 1.95–1.99 erneut recherchieren
- Phase 0 neu aufrollen
- `async-trait` / Tauri Edition 2024 (eigene PRs)
- `HANDOFF.md` committen
- Dependabot-PRs mergen ohne eigenen Auftrag
- Welle 1-Fixes anfassen, außer Regression

---

## Einstiegs-Prompt nächste Session

```
Du bist Agent für BPMNinja (Deutsch). Lies HANDOFF.md, AGENTS.md, plan.md.
Welle 1–5 liegen uncommitted. Nicht neu analysieren, nicht neu implementieren.
Nächster Schritt nur nach explizitem Auftrag: Commit (crate-Schnitt in HANDOFF).
Frozen: Hexagon, Phase 0, Rust 1.99, Docker native Multi-Arch.
```
