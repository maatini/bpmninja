## Zusammenfassung

<!-- Was ändert dieser PR und warum? Zwei bis vier Sätze. -->

## Verlinktes Issue

Closes #

## Art der Änderung

- [ ] `feat` — neues Verhalten
- [ ] `fix` — Fehlerbehebung
- [ ] `refactor` — Umbau ohne Verhaltensänderung
- [ ] `perf` — Performance
- [ ] `docs` — Dokumentation
- [ ] `test` — Tests
- [ ] `ci` / `chore` — Infrastruktur, Abhängigkeiten, Hygiene

Betroffene Bereiche:

- [ ] `engine-core`
- [ ] `bpmn-parser`
- [ ] `persistence-nats` / `persistence-memory`
- [ ] `engine-server`
- [ ] `desktop-tauri`
- [ ] `bpmn-ninja-external-task-client`
- [ ] `api-spec`
- [ ] CI, Doku, Tooling

## Testnachweis

<!-- Ausgeführte Befehle plus Ergebnis, z. B. "cargo test -p engine-core --lib: 233 passed". -->

```text
Befehl:
Ergebnis:
```

## Checkliste

- [ ] `cargo fmt --all --check` grün
- [ ] `cargo clippy --workspace -- -D warnings` grün
- [ ] `cargo test --workspace` grün (NATS-Tests mit laufendem Server)
- [ ] Betroffene Desktop-/Client-Tests grün (`npm run test:deps`, Vitest, Playwright)
- [ ] Doku bzw. `docs/knowledge-base/` angepasst
- [ ] Keine `unwrap()` oder `println!` in Produktionscode
- [ ] Neue Dateien nur, wo sie fachlich nötig sind
- [ ] Commit-Messages konventionell auf Deutsch
- [ ] Kein `git push --force` auf `main`
