# CLAUDE.md — BPMNinja

## Mission
BPMNinja ist eine schlanke, Camunda-kompatible BPMN-2.0-Workflow-Engine in Rust: token-basierte
Ausführung, lock-free Concurrency (DashMap), NATS JetStream, Tauri-Desktop-UI mit bpmn-js und ein
TypeScript-Client für External-Task-Worker.
Sprache: Deutsch in Kommunikation, Commits, Issues und Dokumentation; Code-Bezeichner englisch.

## Stack
| Ebene | Technologie |
|---|---|
| Rust | 1.99, Edition 2024 (Tauri-Crate 2021) |
| Async/HTTP | Tokio 1.52, Axum 0.8 |
| Parser/Scripting | quick-xml 0.42, Rhai 1.25 |
| Persistenz | NATS JetStream (async-nats 0.50) |
| Desktop | Tauri 2.12, React 19, bpmn-js 18, Tailwind 4 |
| Qualität | cargo-fuzz, cargo-mutants, Playwright, cargo-deny |

## Workspace
`engine-core` (Token-Execution, Gateways, Events, Rhai) · `bpmn-parser` (XML → ProcessDefinition) ·
`persistence-nats` (JetStream) · `persistence-memory` (Tests) · `engine-server` (Axum-REST) ·
`agent-orchestrator` (Demo-Worker) · `desktop-tauri` (Tauri + React) ·
`bpmn-ninja-external-task-client` (TypeScript) · `api-spec` (TypeSpec/OpenAPI) · `fuzz`.

## Architektur-Invarianten
- Hexagonal: `engine-core` besitzt den Port `WorkflowPersistence`; jede Persistenzoperation geht über den Trait.
- Token-SSOT: `ProcessInstance.tokens: HashMap<Uuid, Token>`.
- Embedded Sub-Prozesse werden beim Parsen geflattet; kein Runtime-Nesting.
- `BpmnElement` ist ein geschlossener Enum und wird exhaustiv gematcht; `NextAction` steuert den Executor-Loop.
- `ProcessDefinition` ist nach dem Bau unveränderlich und wird als `Arc<ProcessDefinition>` geteilt.
- Rhai: `let x = 1;` deklariert, `x = 2;` ändert eine bestehende Variable.
- Fehler: `thiserror` + `anyhow`; Logging über `tracing`.
- Versionen: Alle Crates synchron auf Version **0.7.25**. Quelle ist `version.json`; `npm run version:check`
  prüft alle Manifeste. Git-Tags: `v0.7.25`.

## Arbeitsweise
- Ohne expliziten Auftrag: keine Produktänderungen, keine Commits, kein Push.
- Cross-Crate-Reihenfolge: `engine-core` → `bpmn-parser` → `persistence-nats` → `engine-server` → `desktop-tauri`.
- Plan bei unklaren Anforderungen oder Architekturentscheidungen; klare kleine Aufgaben direkt umsetzen.
- Nach Änderungen: `cargo fmt`, Clippy und Tests für die betroffenen Crates.
- Keine Wegwerf-Dateien im Repo; Testcode gehört in benannte Module.

## Kommandos
```bash
# Toolchain (Devbox; MSRV 1.99)
eval "$(devbox shellenv)"

cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p engine-core <test_name>
cargo fmt --all --check

npm run version:check          # Versionen in allen Manifesten
node tools/check-docs.mjs      # Doku-Links und Metriken

cd desktop-tauri && npm run build && npx playwright test
npm exec -- nx affected -t lint test build typecheck check   # optional, statt Einzelkommandos
```

## Nie machen
- Kein `unwrap()` in Produktionspfaden; `expect("Grund")` nur bei garantierter Invariante.
- Kein `println!`/`eprintln!`; immer `tracing`.
- Keine neuen `#[allow(...)]`-Attribute.
- Keine Dependencies direkt in Crate-`Cargo.toml`, wenn sie workspace-weit gelten.
- Kein `git push --force` auf `main`.
- Keine neuen Dateien ohne Auftrag; keine Mock-Datenbanken in Integrationstests.

## Weiterlesen (bei Bedarf)
- `.agent/rules/` — sechs Pro-Crate-Regeln mit `file_match`-Triggern.
- `docs/knowledge-base/index.md` — Verantwortlichkeiten, Abhängigkeiten, Interfaces, Gotchas.
- `HANDOFF.md` — aktueller Arbeitsstand und offene Aufträge (lokal, gitignoriert).
- `plans/` — ausgearbeitete Vorhaben.
- Graph-Navigation auf Zuruf: `graphify update .` (AST) bzw. `graphify query "..."`.
