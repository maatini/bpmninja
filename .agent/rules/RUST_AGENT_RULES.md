---
trigger: file_match
file_patterns: ["**/*.rs", "**/Cargo.toml", "**/package.json", "**/tauri.conf.json"]
---

# Rust- und Manifest-Regeln

## Workspace-weit
- Rust 2024 für alle Crates; Ausnahme `desktop-tauri/src-tauri` (Edition 2021 für Tauri).
- `Result<T, E>` mit `?`; `thiserror` für Fehlertypen, `anyhow` für Kontext an Crate-Grenzen.
- `EngineError` in `engine-core/src/domain/error.rs` ist der zentrale Fehlertyp.
- Async mit Tokio; im async-Kontext `.await`, kein `block_on`.
- Logging über `tracing`, nie `println!`/`eprintln!`.
- Zustände über Enums (`BpmnElement`, `InstanceState`, `NextAction`) und exhaustives Matching.
- Geteilter Zustand: `Arc<dyn WorkflowPersistence>`, `Arc<ProcessDefinition>`, DashMap im Instance-Store.
- Tests: `#[test]`/`#[tokio::test]`, `ProcessDefinitionBuilder` statt Raw-XML, keine Mock-Datenbanken in Integrationstests.
- Gate: `cargo clippy --workspace --all-targets -- -D warnings` und `cargo fmt --all --check`.

## Dependencies
- Gemeinsame Dependencies stehen in `[workspace.dependencies]` der Root-`Cargo.toml`; Crates referenzieren `workspace = true`.
- Crate-spezifische Dependencies dürfen direkt deklariert werden.
- Feature-Flags minimal halten (`in-memory` in `persistence-nats`).
- Verboten: vendored Dependencies, Git-Dependencies in Produktion, exakte Pins (`=1.2.3`) ohne dokumentierten Grund.

## Versionen
- Quelle ist `version.json`; `npm run version:check` prüft alle Manifeste, `npm run version:sync` schreibt sie.
- Aktueller Stand: 0.7.25. Crates, `src-tauri`, `tauri.conf.json`, npm-Projekte und `.agent/manifest.json` bleiben synchron.
- SemVer: MAJOR bei Breaking API, MINOR bei Features, PATCH bei Fixes.
- Tauri-Crate und npm-Paket müssen in Major.Minor übereinstimmen (`tauri` ↔ `@tauri-apps/api`, Plugins ↔ `@tauri-apps/plugin-*`).

## Anti-Patterns
- Kein `.unwrap()` in Produktionspfaden; `.expect("Grund")` nur bei garantierter Invariante, in Tests erlaubt.
- Keine riesige `main.rs`; Module aufteilen.
- Kein geteilter mutierbarer Zustand ohne Synchronisation.
- Keine Compiler-Warnungen ignorieren.
