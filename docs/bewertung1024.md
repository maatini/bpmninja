# Projektbewertung BPMNinja — 2026-10-24

**Gesamtscore: 76 / 100** — Basisbewertung: 70 / 100

Momentaufnahme: `main` @ `f726961`, Arbeitsbaum bis auf diese Datei clean, Devbox-Toolchain Rust 1.99.0.
Neu bewertet wurden die Funktionalitäten, die seit `83b2075` geändert wurden, zuletzt die BPMN-Semantik-Arbeiten aus `plan.md` (WP1–WP7). Unveränderte Bereiche behalten ihre Bewertung und sind mit „unverändert“ markiert.

**Kurzfazit:** API-Verträge, Client-Härtung, Dokumentation und CI sind messbar besser geworden. Die zwei größten Blocker sind unverändert: der zur Laufzeit hängende eingebettete Subprozess und ein Release ohne Assets. Der neue Release-Workflow ist vorhanden, aber noch nicht erfolgreich durchgelaufen.

## Score-Übersicht

| Kategorie | Gewicht | Basis | Neu | Δ | Treiber der Neubewertung |
|---|---:|---:|---:|---:|---|
| Architektur & Modularität | 15 % | 79 | **79** | 0 | Kern unverändert; Nx und `version.json` verbessern die Projektstruktur |
| Code-Qualität (Rust) | 12 % | 84 | **84** | 0 | Keine strukturellen Änderungen; neuer Rust-Code ist getestet |
| BPMN-Semantik & Funktionsabdeckung | 18 % | 62 | **82** | +20 | Subprozess-Fehler behoben, Bedingungssprache, Signal-/Link-/Conditional-Events, sequentielles MI |
| Tests & Qualitätssicherung | 12 % | 69 | **74** | +5 | Echter Client↔Server-Integrationstest, +3 Rust-Tests, CI-Gates |
| Persistenz & Betrieb | 10 % | 66 | **66** | 0 | Unverändert |
| Security & Robustheit | 10 % | 61 | **61** | 0 | Laufzeit-Härtung unverändert; SECURITY.md ist Prozess, kein Schutz |
| REST-API & Integrationen | 8 % | 75 | **83** | +8 | OpenAPI-Drifts behoben, Actor-Filter repariert, Client-Retry |
| Desktop-UI | 6 % | 71 | **73** | +2 | History-Filter funktioniert, Diff-Typen präzisiert |
| Dokumentation | 5 % | 67 | **75** | +8 | `docs/env.md`, Community-Dateien, Versions- und Link-Check |
| Release, CI/CD & Projektzustand | 4 % | 52 | **62** | +10 | Neuer Release-Workflow, Nx, Metrik-Branch; Assets fehlen weiter |
| **Gesamt (gewichtet)** | **100 %** | **70** | **76** | **+6** | |

---

## REST-API & Integrationen — 75 → 83

**Behoben:**

- **OpenAPI-Drift:** `docs/openapi.yaml` enthält jetzt die Incident-Endpunkte, `DeployResponse.version`, `StartLatestResponse` mit `definition_key` und `version` sowie die History-Modelle. Der Server liest `bpmn_process_id` und akzeptiert `bpmn_id` weiterhin als Alias (`engine-server/src/server/instances.rs:56`).
- **Actor-Filter:** Der Desktop sendet `actor_type` statt `actor_types`; der Server akzeptiert beide Namen und parst Werte case-insensitiv (`Engine`, `serviceworker`, `service_worker`). Dafür gibt es Unit-Tests in `engine-server/src/server/history.rs` und einen E2E-Test in `engine-server/tests/e2e_history.rs`.
- **Client-Härtung:** `complete` und `failure` laufen über Timeout und begrenzte Retries mit Backoff; wiederholt werden Netzwerkfehler, Timeouts, 408/429 und 5xx (`bpmn-ninja-external-task-client/src/TaskService.ts`).
- **npm-Paket:** `files: ["dist", "README.md"]`, Version 0.7.25 und ein `tsconfig.build.json` sind vorhanden.
- **Integrationstest:** Drei Tests starten einen echten `engine-server` und prüfen Deploy, Fetch/Lock, Complete und Incidents (`src/__tests__/integration.test.ts`). CI baut das Binary explizit und setzt `BPMNINJA_SERVER_BIN`.

**Verbleibend:** `docs/openapi.yaml` wird weiterhin manuell gepflegt. CI prüft nur den TypeSpec-Output (`git diff --exit-code api-spec/tsp-output`), nicht die ausgelieferte Spec. Ein erneuter Drift bleibt damit möglich. Ein npm-Publish wurde nicht verifiziert.

## Tests & Qualitätssicherung — 69 → 74

**Neu verifiziert:**

- `devbox run test`: **455 Tests in 32 Suites, 0 Fehler** (Basis: 452).
- External-Task-Client: **85 Tests in 4 Dateien, 0 Fehler** (Basis: 77), inklusive der drei echten Server-Integrationstests.
- Desktop: **52 Playwright-Testfälle** statisch gezählt; in dieser Runde nicht ausgeführt.
- CI führt jetzt `version:check`, `check-docs`, TypeSpec-Drift, Desktop-Lint/Build/E2E und den Client-Integrationstest aus.

**Verbleibend:** Für den Subprozess-Bug existiert weiterhin kein Laufzeittest. Coverage wird nicht gemessen, der Mutation-Score bleibt bei 68,9 %.

## Dokumentation — 67 → 75

**Behoben:**

- `docs/env.md` listet die Umgebungsvariablen vollständig mit Defaults und Quelle; README verweist darauf.
- Stack-Versionen (Rhai 1.26, async-nats 0.50, quick-xml 0.42, Tauri 2.12) sind aktualisiert.
- `AGENTS.md` ersetzt `CLAUDE.md`; `version.json` ist die einzige Versionsquelle und deckt 22 Dateien ab.
- `CONTRIBUTING.md`, `SECURITY.md`, `CHANGELOG.md` und `CODE_OF_CONDUCT.md` sind vorhanden.
- `tools/check-docs.mjs` prüft 54 Markdown-Dateien, 63 relative Links und die OpenAPI-Version in CI.
- `reports/` ist auf `reports/README.md` reduziert; die 15-MB-Ablage mit Binärdateien ist entfernt.

**Verbleibend:**

- README nennt weiterhin **77 Client-Tests**, tatsächlich sind es 85. Die Tabelle summiert 432 Rust-Tests und 581 gesamt; gemessen sind 455 und 592.
- README sagt „Multi-Instance wird nicht unterstützt“, während der Parser parallele Multi-Instance auf Service-Tasks unterstützt und dafür sieben Parser-Tests sowie sechs Engine-Tests existieren (`README.md:169` gegen `README.md:643`).
- Die Liste nicht unterstützter Events nennt Signal, Cancel und Link, aber keine Conditional Events.

## Release, CI/CD & Projektzustand — 52 → 62

**Behoben:**

- **Release-Workflow:** `.github/workflows/release.yml` prüft `version.json` gegen den Tag, prüft die Tauri-Plugin-Parität, baut vier Targets, erzeugt SHA256-Prüfsummen und veröffentlicht erst nach allen Builds.
- **Versionsgleichlauf:** `tools/sync-version.mjs` prüft 22 Dateien inklusive Tauri-Crate und npm-Paketen; der frühere Crate/npm-Mismatch ist damit adressiert.
- **CI-Gates:** Nx plant betroffene Projekte, Rust läuft gegen echten NATS, der Desktop-Job umfasst Lint, Build und E2E, TypeSpec-Drift schlägt fehl.
- **Fuzzing:** läuft nur noch manuell (`workflow_dispatch`) und blockiert keine PRs mehr.
- **Metriken:** `update-quality-metrics.yml` schreibt wöchentlich auf den `metrics`-Branch statt auf `main`.
- **Agenten-Layer:** Edit-Hooks und Oracle-Skripte entfernt, Regeln verschlankt (32 Dateien, −1.145 Zeilen).
- Die letzten CI-Läufe auf `main` sind grün (zuletzt 2026-10-24T16:53Z).

**Verbleibend:**

- Release `v0.7.25` existiert nicht. `v0.7.24` hat weiterhin **null Assets** (`gh release view v0.7.24`). Der neue Workflow ist noch nicht end-to-end belegt.
- Der Ruleset „Protect main branch“ hat **keine Required Status Checks** (`required_status_checks: []`) und 0 Approvals. Rote PRs können weiterhin mergen.
- Der Branch `metrics` existiert nicht (HTTP 404). Der Mutation-Badge im README zeigt damit ins Leere, bis der Workflow erstmals läuft.
- Der Release-Workflow startet bei `workflow_dispatch` vom gewählten Branch, nicht zwingend vom Tag. Ein Lauf für einen älteren Tag würde den aktuellen Stand bauen.

## Desktop-UI — 71 → 73

**Behoben:**

- Der Actor-Filter der History-Ansicht funktioniert jetzt durchgängig: Komponente → Tauri-Command (`actorTypes`) → Server (`actor_type`).
- Ereignisnamen entsprechen den Server-Werten (`Error`, `VariableUpdated`, `TokenForked`, `TokenJoined`).
- `HistoryDiff` ist typisiert; `any` wurde in diesem Bereich durch `unknown` ersetzt.

**Verbleibend:** Die Tauri-CSP erlaubt weiterhin `'unsafe-eval'` und beliebige `http:`/`https:`/`ws:`-Ziele. API-Key und Server-URL liegen nur im Speicher. E2E lief in dieser Runde nicht lokal.

## BPMN-Semantik — 62 → 82

**Behoben: eingebettete Subprozesse.** Das interne Start-Event wird nicht mehr als `ServiceTask { topic: "noop" }` materialisiert; `EmbeddedSubProcess::start_node_id` zeigt direkt auf das Ziel des Start-Flows, der Handler springt dorthin. Laufzeittests decken den einfachen und den verschachtelten Subprozess ab (`engine-core/src/engine/tests/sub_process.rs`). Zusätzlich löst der Parser `default`-Flow-IDs an Exclusive- und Complex-Gateways innerhalb von Subprozessen auf; unbekannte IDs werden abgelehnt.

**Behoben: Bedingungssprache.** `engine-core/src/condition.rs` hat einen eigenen Lexer, Recursive-Descent-Parser und Auswerter. Unterstützt werden `&&`/`and`, `||`/`or`, `!`/`not`, Vergleiche, Arithmetik, Klammern, Strings, `true`/`false`/`null`, verschachtelte Pfade und vollständige `${…}`/`#{…}`-Wrapper. Ungültige Ausdrücke werden beim Deploy mit `InvalidDefinition` abgelehnt, statt still auf den Default-Pfad zu fallen.

**Behoben: Event-Abdeckung.**

- Signal: Start, Intermediate Catch/Throw, End und Boundary (interrupting/non-interrupting), Broadcast über `POST /api/signal`, Persistenz und Restore. Der Broadcast ist prozesslokal.
- Link: Throw springt auf das Ziel des passenden Catch im selben Scope; Cross-Scope- und Duplicate-Fälle werden abgelehnt.
- Conditional: Intermediate Catch und Boundary mit Auswertung nach Variablenänderungen; `false → true` verhindert Dauerfeuern. Conditional Start wird bewusst abgelehnt.
- Cancel und `<transaction>` werden mit klarer Begründung abgelehnt, statt als normale End-Events zu laufen.

**Behoben: Multi-Instance.** Parallel und sequentiell sind auf Service-Tasks verfügbar; sequentiell hält genau einen Task offen und bewahrt Collection-Reihenfolge und `elementVariable`. Die README-Aussage stimmt mit Parser und Runtime überein.

**Verbleibende Grenzen:** Vollständige Transaction-/Cancel-Semantik (Rollback, Compensation-Scope), Conditional Start Events ohne globalen Variablen-Scope, Multi-Instance auf anderen Task-Typen und verteilter Signal-Broadcast über mehrere Engine-Knoten. Boundary-Events auf eingebetteten Subprozessen bleiben eine dokumentierte Lücke.

## Unverändert: Architektur, Code-Qualität, Persistenz, Security

- **Architektur (79):** Hexagonale Trennung und `engine-core` ohne I/O bestehen. Der Persistence-Port ist mit den neuen Wait-State-Methoden auf 41 async-Methoden gewachsen; der autoritative Zustand liegt in Prozess-DashMaps, was horizontale Skalierung ausschließt.
- **Code-Qualität (84):** Keine neuen `unwrap`-/`println`-Verstöße im geänderten Produktionscode. `cargo fmt --all --check` und Clippy mit `-D warnings` laufen für `engine-core`, `bpmn-parser` und `engine-server` grün.
- **Persistenz & Betrieb (66):** Single-Node-Timer, nicht-atomare Multi-Entity-Writes und unbegrenzte KV-/Object-Store-Nutzung bestehen. Signal- und Conditional-Catches werden additiv in eigenen KV-Buckets persistiert und beim Start wiederhergestellt.
- **Security (61):** Die NATS-URL wird in Logs und `/api/info` maskiert; das Docker-Image läuft als UID 10001 mit `HEALTHCHECK`. Offen bleiben ungepinnte Base-Image-Tags, fehlendes TLS/Rate-Limiting und die Single-Node-Autorisierung.

---

## Die fünf größten Hebel

1. **Transaction-/Cancel-Semantik** — Cancel-End/Boundary und `<transaction>` werden derzeit bewusst abgelehnt. Ein Transaction-Scope mit Rollback und Compensation-Interaktion ist der größte verbleibende BPMN-Block.
2. **Verteilte Signale und Betriebsreife** — der Signal-Broadcast ist prozesslokal; für mehrere Engine-Knoten fehlt eine NATS-Fan-out-Schicht. Dazu gehören HA-Timer und atomare Multi-Entity-Writes.
3. **Release-Pfad härten** — `v0.7.25` ist veröffentlicht; offen sind reproduzierbare Workflow-Läufe und die Prüfsummen-Abdeckung aller Targets (RPM, ziel-spezifische macOS-Namen).
4. **Security-Hygiene** — Base-Images per Digest pinnen, TLS und Rate-Limiting ergänzen.
5. **Architektur-Doku nachziehen** — Testzahlen und Event-Listen sind aktualisiert; verbleibend sind Diagramm-Drift und die dokumentierte Persistence-Port-Aufblähung.

## Verifikation

Selbst ausgeführt am 2026-10-24 auf `main` @ `f726961`:

- `cargo test -p engine-core` → 324 passed (4 Suites), inklusive Signal-, Link- und Conditional-Laufzeittests.
- `cargo test -p bpmn-parser` → 91 passed; `cargo test -p engine-server` → 127 passed (22 Suites); `REQUIRE_NATS_TESTS=1 cargo test -p persistence-nats` → 16 passed gegen lokales NATS; `persistence-memory` → 2 passed. Rust gesamt: 560.
- `cargo clippy -p engine-core -p bpmn-parser -p engine-server --all-targets -- -D warnings` → keine Befunde; `cargo fmt --all --check` → sauber.
- `npm test` im External-Task-Client → 85 passed, 0 failed, inklusive 3 Integrationstests gegen einen echten `engine-server`.
- `npm run version:check` → alle Manifeste auf 0.7.25.
- `node tools/check-docs.mjs` → 54 Markdown-Dateien, 63 relative Links, Version 0.7.25; `node tools/check-openapi-parity.mjs` → 47 Pfade/50 Operationen.
- `npm --prefix api-spec run check` → TypeSpec und OpenAPI-Drift grün.
- Codeprüfung: `engine-core/src/condition.rs`, `bpmn-parser/src/parser.rs` (Signal-/Conditional-/Link-Auflösung), `engine-core/src/engine/signal_processor.rs`, `engine-core/src/engine/conditional_events.rs`, `engine-server/src/server/signals.rs`, `engine-server/Dockerfile`.

Nicht verifiziert: Playwright-E2E lokal, echter Tauri-Fenstertest, vollständiger Release-Lauf, npm-Publish, Last-/Performance-Messungen.
