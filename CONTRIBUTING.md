# Contributing zu BPMNinja

Danke für dein Interesse an BPMNinja — einer BPMN-2.0-Workflow-Engine in Rust mit
Camunda-kompatiblen External-Task-Workern, NATS-JetStream-Persistenz, Axum-REST-API
und Tauri-Desktop-UI. Projektsprache ist Deutsch (Doku, Issues, PRs, Commits);
Code-Bezeichner bleiben Englisch.

## Voraussetzungen

| Werkzeug | Version | Zweck |
|---|---|---|
| [devbox](https://www.jetify.com/devbox) | aktuell | reproduzierbare Toolchain (Rust, Node, NATS, Python) |
| Rust | 1.99.0 (`rust-toolchain.toml`) | Engine und alle Crates |
| Node.js | 22+ | Desktop, TS-Client, TypeSpec |
| Docker | aktuell | NATS und Compose-Setup |

## Setup

```bash
devbox shell                 # Toolchain: Rust 1.99, Node, NATS
devbox run nats:up           # NATS via Docker Compose
devbox run engine:run        # engine-server auf :8081
npm ci && cd desktop-tauri && npm ci   # Nx-Root und Desktop
```

## Build und Tests

```bash
devbox run build             # cargo build --workspace
devbox run lint              # cargo clippy --workspace -- -D warnings
devbox run fmt               # cargo fmt --all --check
devbox run test              # cargo test --workspace
cargo deny check             # Lizenzen und Advisories (deny.toml)

# Desktop und Client
cd desktop-tauri && npm ci && npm run lint && npm run build && npm run test:e2e
cd bpmn-ninja-external-task-client && npm ci && npm test
node tools/check-openapi-parity.mjs   # docs/openapi.yaml gegen TypeSpec-Output
```

Prüf-Targets laufen zusätzlich über Nx (Cargo bleibt Build-Engine für Rust). Aufruf über
`npm exec -- nx …`; der Aufruf über `npx` scheitert in der Devbox mit `Missing script: nx`.

```bash
npm exec -- nx run-many -t fmt clippy test deny   # Rust-Targets (test braucht cargo-nextest)
npm exec -- nx affected -t lint test build        # Rust, Desktop, Client und API-Spec
npm exec -- nx affected -t fmt clippy test deny   # nur Rust
```

`cargo-nextest` ist in der Devbox enthalten und nach `devbox shell` verfügbar; die CI
installiert es über `taiki-e/install-action`. Außerhalb der Devbox installierst du es mit
`cargo install cargo-nextest --locked`.

## Testpflichten pro Änderungsbereich

| Bereich | Pflicht |
|---|---|
| `engine-core` | Unit-Test für jede Verhaltensänderung (Gateways, Timer, Incidents, Events); Regressionstest bei Bugfixes |
| `bpmn-parser` | Test für jedes akzeptierte und jedes abgelehnte XML-Konstrukt, `ProcessDefinitionBuilder` statt Raw-XML; Fuzz-Input bei Parser-Änderungen ergänzen |
| `persistence-nats` / `persistence-memory` | Adapter-Test gegen den Trait-Vertrag; keine Mock-Datenbanken |
| `engine-server` | HTTP-Test über Deploy → Start → Abschluss; Fehlerpfade inklusive Statuscode prüfen |
| `desktop-tauri` | `npm run test:deps` (Tauri-Parität) und Playwright-E2E bei UI-Flows |
| `bpmn-ninja-external-task-client` | Vitest für neue Optionen und Fehlerfälle |
| `api-spec` | TypeSpec kompilieren, generierte Dateien ohne Diff committen |

## Branch- und Commit-Konventionen

- Feature-Branch von `main`, kurze Namen: `fix/timer-r3`, `feat/cors-allowlist`.
- Commits konventionell auf Deutsch: `feat:`, `fix:`, `docs:`, `refactor:`, `chore:`, `perf:`, `test:`, `ci:`.
- Kein `git push --force` auf `main`. `main` bleibt stabil.

## Code-Stil

- Rust 2024 Edition, `cargo fmt` und Clippy mit `-D warnings`.
- Kein `unwrap()` in Produktionscode; Fehler über `thiserror` (Domäne) und `anyhow` (Binaries).
- `tracing` statt `println!`/`eprintln!`, keine unnötigen `.clone()`.
- Keine neuen `#[allow(...)]` als Abkürzung — Ursache beheben.
- Hexagonale Grenzen respektieren: Cross-Crate nur über Traits aus `engine-core/src/port/`.

## Pull Requests

1. Issue verlinken (`Closes #123`) — PRs ohne Issue werden nur bei Trivialien angenommen.
2. Klein und thematisch geschlossen halten; Refactorings nicht mit Features mischen.
3. Testnachweis im PR: ausgeführte Befehle plus Ergebnis.
4. Doku aktualisieren, wenn sich Verhalten, Konfiguration oder API ändern.

Checkliste vor dem Öffnen:
- [ ] `cargo fmt --all --check` grün
- [ ] `cargo clippy --workspace -- -D warnings` grün
- [ ] `cargo test --workspace` grün (NATS-Tests mit laufendem Server)
- [ ] Betroffene Desktop-/Client-Tests grün
- [ ] Doku bzw. Knowledge Base angepasst
- [ ] Keine `unwrap()`/`println!` in Produktionscode
- [ ] Commit-Messages konventionell auf Deutsch

## Weiterführende Doku

- [AGENTS.md](AGENTS.md) — Projektleitfaden, Architektur-Regeln, Workflow
- [docs/knowledge-base/index.md](docs/knowledge-base/index.md) — Module, Abhängigkeiten, Gotchas
- [SECURITY.md](SECURITY.md) — Schwachstellen melden

## Lizenz

Beiträge fallen unter die Projektlizenz: Rust-Crates `MIT OR Apache-2.0`,
TypeScript-Client MIT. Mit dem PR bestätigst du, dass du den Code unter diesen
Bedingungen beisteuern darfst.
