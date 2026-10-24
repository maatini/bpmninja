# Plan: Release, CI/CD und Projektzustand mit Nx

**Stand:** 2026-10-24
**Basis:** `main` @ `83b2075`
**Status:** Vorschlag, nicht umgesetzt
**Ziel:** Release-Assets zuverlässig erzeugen, CI auf betroffene Projekte zuschneiden, Projektzustand bereinigen — orchestriert mit Nx.

## Ausgangslage

| Befund | Evidenz |
|---|---|
| Release `v0.7.24` hat null Assets | `gh release view v0.7.24` → `assets: []` |
| Tauri-Versionen driften | Crate `tauri-plugin-fs` 2.5.1 gegen npm `@tauri-apps/plugin-fs ^2.7.1` |
| Keine Pflicht-Checks auf `main` | Ruleset „Protect main“: `required_status_checks: []`, 0 Approvals |
| Metrik-Bot kann nicht pushen | Ruleset verlangt PRs; seit 07:44 fünf fehlgeschlagene Läufe, letzter Erfolg 05:27 |
| Drei parallele Release-Workflows | `release.yml` plus `build-linux/macos/windows.yml` mit eigenen Draft-Releases |
| CI läuft immer vollständig | `ci.yml` ohne Pfadfilter, alle Jobs bei jedem PR |
| 11 Manifeste tragen Versionen | 6 Crates, `src-tauri`, `tauri.conf.json`, 3 npm-Projekte, Client abweichend 1.0.0 |
| Repo-Ballast | 479 getrackte Dateien unter `reports/`, davon 20 `.lance`, 2,4 MB |
| Keine Community-Dateien | CONTRIBUTING, SECURITY, CHANGELOG, CODE_OF_CONDUCT fehlen |

## Leitidee

Nx orchestriert alle Projekte (Rust-Workspace, Desktop, TS-Client, TypeSpec). Cargo bleibt die Build-Engine für Rust. Release bekommt eine einzige Versionsquelle. Die drei akuten Blocker — leerer Release, fehlende Pflicht-Checks, blockierter Metrik-Bot — werden zuerst behoben.

Erwartete Wirkung: Kategorie „Release, CI/CD & Projektzustand“ von 52 auf etwa 80, Gesamtscore von 70 auf 73–75.

---

## Phase 0 — Blocker beheben (ca. 1 Tag)

### 0.1 Tauri-Plugin-Parität

`desktop-tauri/src-tauri/Cargo.toml`:

```toml
tauri-plugin-fs = "2.7"   # statt 2.5, passend zu @tauri-apps/plugin-fs ^2.7.1
```

Prüfen, ob `tauri-plugin-dialog` (Crate 2.7.1 / npm ^2.7.3) und `tauri` (Crate 2.11.5 / API ^2.11.1) ebenfalls major.minor-gleich sind.

Neu: `desktop-tauri/scripts/assert-tauri-parity.mjs`
- liest `src-tauri/Cargo.toml` und `package.json`
- vergleicht major.minor für `tauri`, `tauri-plugin-dialog`, `tauri-plugin-fs`
- Exit-Code 1 bei Abweichung
- Aufruf als `npm run test:deps` erweitern und in CI sowie im Release-Prepare-Job ausführen

**Akzeptanz:** `npm run test:deps` ist grün, `cargo check` im Tauri-Projekt läuft durch.

### 0.2 Release-Workflow umbauen

`.github/workflows/release.yml` in drei Jobs teilen:

1. `create-release`
   - `gh release create "$TAG" --draft --generate-notes --title "BPMNinja $TAG"`
   - Output `release_id`
   - Version-Parity-Check vorher ausführen
2. `build` (Matrix Linux, Windows, macOS arm, macOS intel)
   - `tauri-action` mit `releaseId: ${{ needs.create-release.outputs.release_id }}`
   - `releaseDraft: false` in den Build-Jobs
   - Prüfsummen erzeugen und als Asset hochladen
3. `publish`
   - `needs: [create-release, build]`
   - `gh release edit "$TAG" --draft=false`
   - Bei Fehlschlag bleibt der Draft zur Inspektion stehen

Zusätzlich: `concurrency: release-${{ github.ref }}`, `permissions: contents: write`, `fetch-depth: 0`.

### 0.3 Legacy-Workflows entfernen

- `.github/workflows/build-linux.yml`
- `.github/workflows/build-macos.yml`
- `.github/workflows/build-windows.yml`

Sie nutzen `npm install` statt `npm ci` und erzeugen Releases mit `app-v__VERSION__`. Funktion ist vollständig in `release.yml` abgedeckt.

### 0.4 Pflicht-Checks auf `main`

Ruleset „Protect main“ ergänzen:

- Required Status Checks: `Rust clippy and tests`, `External task client tests`, `TypeSpec compile`, `Desktop frontend lint, build, and E2E`, `cargo deny`
- `strict_required_status_checks_policy: true`
- Approvals bleiben bei 0, solange Ein-Personen-Betrieb

Das ist eine Änderung an gemeinsamer Infrastruktur und benötigt Admin-Rechte sowie ein explizites GO.

### 0.5 Metrik-Bot reparieren

Ursache: Das Ruleset verlangt PRs, der Bot pusht direkt auf `main`.

Optionen:

| Option | Aufwand | Wirkung |
|---|---|---|
| A: `github-actions[bot]` als Bypass-Actor | gering | Bot funktioniert wie bisher, viele Commits bleiben |
| B: Bot öffnet PR mit Auto-Merge | mittel | regelkonform, mehr PR-Rauschen |
| C: Metriken wöchentlich auf `metrics`-Branch, Badge zeigt dorthin | mittel | maximal ein Commit pro Woche, `main` bleibt ruhig |

**Empfehlung:** C, zusätzlich Bypass nur für den wöchentlichen Lauf. `update-quality-metrics.yml` von `workflow_run` auf `schedule` (wöchentlich) plus `workflow_dispatch` umstellen; Mutation und Fuzz liefern ihre Artefakte weiter, der Merge läuft gesammelt.

### 0.6 Release neu schneiden

Nach 0.1–0.5: Version auf 0.7.25 anheben, Tag setzen, Release-Workflow beobachten.

**Akzeptanz:** Release `v0.7.25` enthält mindestens vier Assets (Linux deb/AppImage, Windows msi/nsis, macOS arm, macOS intel) plus `.sha256`.

---

## Phase 1 — Nx einführen (2–3 Tage)

### 1.1 Setup-Variante

Package-based: `nx.json` plus `project.json` je Projekt. Keine npm-Workspaces, keine Zusammenführung der Lockfiles.

Begründung: `tauri-action` und der Tauri-Build erwarten `desktop-tauri/package-lock.json`. Hoisting würde diesen Pfad brechen. Ein späterer Wechsel auf Workspaces bleibt möglich.

### 1.2 Projektgraph

| Projekt | Root | Targets | Cache |
|---|---|---|---|
| `rust` | `.` | `fmt`, `clippy`, `test`, `deny` | nein (sccache + cargo nextest) |
| `desktop` | `desktop-tauri` | `lint`, `typecheck`, `build`, `e2e`, `tauri:build` | ja |
| `client` | `bpmn-ninja-external-task-client` | `build`, `lint`, `test` | ja |
| `api-spec` | `api-spec` | `compile`, `check` | ja |
| `workspace` | `.` | `version-check`, `docs:check` | ja |

Rust bleibt ein einziges Nx-Projekt. Targets pro Crate würden parallel um den Cargo-Lock konkurrieren und mehrfach kompilieren.

### 1.3 `nx.json`

```json
{
  "$schema": "./node_modules/nx/schemas/nx-schema.json",
  "targetDefaults": {
    "build": {
      "dependsOn": ["^build"],
      "cache": true,
      "outputs": ["{projectRoot}/dist"]
    },
    "test": { "cache": true },
    "lint": { "cache": true },
    "typecheck": { "cache": true }
  },
  "namedInputs": {
    "rust": [
      "{workspaceRoot}/**/*.rs",
      "{workspaceRoot}/**/Cargo.toml",
      "{workspaceRoot}/Cargo.lock",
      "{workspaceRoot}/rust-toolchain.toml"
    ]
  }
}
```

### 1.4 Beispiel `project.json` (Rust)

```json
{
  "name": "rust",
  "root": ".",
  "projectType": "library",
  "targets": {
    "fmt": { "command": "cargo fmt --all --check", "cache": false },
    "clippy": {
      "command": "cargo clippy --workspace --all-targets -- -D warnings",
      "cache": false
    },
    "test": {
      "command": "cargo nextest run --workspace",
      "cache": false
    },
    "deny": { "command": "cargo deny check", "cache": false }
  }
}
```

### 1.5 Caching

- Lokal: `.nx/cache`
- CI: `actions/cache` mit Hash aus Lockfiles und Quellen
- Rust: `sccache` als `RUSTC_WRAPPER` plus `Swatinem/rust-cache`; **keine** `target/`-Verzeichnisse im Nx-Cache
- Nx Cloud erst später und freiwillig, wenn die Trefferquote gemessen ist

### 1.6 Lokale Befehle

```bash
nx run-many -t lint test build
nx affected -t lint test build
nx graph
```

**Akzeptanz:** `nx affected` überspringt bei einer Docs-Änderung alle Build-Targets; bei einer Rust-Änderung läuft der Rust-Job, der Desktop-Job nicht.

---

## Phase 1b — CI/CD zuschneiden

### 1b.1 Plan-Job

```yaml
plan:
  runs-on: ubuntu-latest
  outputs:
    projects: ${{ steps.affected.outputs.projects }}
  steps:
    - uses: actions/checkout@v7
      with: { fetch-depth: 0 }
    - uses: nrwl/nx-set-shas@v4
    - id: affected
      run: echo "projects=$(npx nx show projects --affected --base=$NX_BASE --head=$NX_HEAD | paste -sd, -)" >> "$GITHUB_OUTPUT"
```

### 1b.2 Jobs

| Job | Läuft wenn | Inhalt |
|---|---|---|
| `rust` | `rust` betroffen | fmt, clippy, nextest mit NATS-Service, sccache |
| `client` | `client` betroffen | npm ci, tsc, Vitest |
| `typespec` | `api-spec` betroffen | compile plus `git diff --exit-code api-spec/tsp-output` |
| `desktop` | `desktop` betroffen | Lint, Typecheck, Build, Playwright mit Browser-Cache |
| `deny` | `rust` betroffen | cargo-deny |
| `desktop-build` | `desktop` betroffen | `cargo check` plus Tauri-Paritätsskript |

Der TypeSpec-Diff fängt den bestehenden OpenAPI-Drift künftig automatisch.

### 1b.3 Härtung

- Actions per SHA pinnen (Renovate oder `pinact`)
- `permissions` je Job minimieren
- `concurrency` beibehalten
- Playwright-Binaries cachen
- `cargo nextest` und `sccache` über `taiki-e/install-action`

**Akzeptanz:** Docs-only-PR unter 2 Minuten; Rust-only-PR ohne Desktop-E2E; Desktop-PR ohne Rust-Job.

---

## Phase 2 — Release und Versionierung (1–2 Tage)

### 2.1 Eine Versionsquelle

`version.json` im Root als Canonical. `tools/sync-version.mjs` schreibt:

- 6 × `*/Cargo.toml`
- `desktop-tauri/src-tauri/Cargo.toml`
- `desktop-tauri/src-tauri/tauri.conf.json`
- `desktop-tauri/package.json`
- `api-spec/package.json`

Der TS-Client bleibt als eigenständiges Paket bei 1.0.0, falls er separat veröffentlicht wird.

`nx run workspace:version-check` prüft im CI auf Drift.

### 2.2 Release-Ablauf

1. `nx release version minor --skip-publish` — npm-Pakete und `CHANGELOG.md`
2. `node tools/sync-version.mjs` — Cargo und Tauri
3. `nx run workspace:version-check` und `npm run test:deps` müssen grün sein
4. Commit, Tag, Push
5. Release-Workflow erzeugt Draft, baut, publiziert

`CHANGELOG.md` entsteht aus den konventionellen Commit-Messages und schließt die bestehende Lücke.

### 2.3 Optional später

- `nx release publish` mit Provenance für den TS-Client
- Signierte Tauri-Bundles, sobald der Updater wieder eingeführt wird

---

## Phase 2b — Projektzustand und Hygiene

| Maßnahme | Umfang |
|---|---|
| `reports/` aus Git nehmen | 479 Dateien, 20 `.lance`, 2,4 MB; `reports/README.md` als Zusammenfassung behalten |
| Scratch-Dokumente verschieben | `plan.md`, `improve1026.md`, `improvements.md` nach `docs/internal/` |
| Community-Dateien | CONTRIBUTING, SECURITY, CODE_OF_CONDUCT, Issue-/PR-Templates, CODEOWNERS |
| Changelog | aus `nx release` |
| Doku-Drift | Testzahlen aus `cargo test -- --list` und Vitest in markierten README-Block; Env-Tabelle aus `docs/env.md` |
| Graphify-Befehl | `devbox run graphify` durch `graphify update .` ersetzen |
| Mutation | `continue-on-error` beibehalten, Schwelle und Trend statt stillem 68,9 % |

**Akzeptanz:** Repo kleiner um rund 470 Dateien und 2,4 MB; `main` enthält höchstens einen Metrik-Commit pro Woche.

---

## Zeitplan

| Woche | Inhalt | Ergebnis |
|---|---|---|
| 1 | Phase 0 | `v0.7.25` mit Assets, Pflicht-Checks aktiv, Legacy-Workflows entfernt |
| 2 | Nx-Bootstrap und affected-CI | ein Befehl lokal, CI überspringt unbeteiligte Projekte |
| 3 | Release-Automation und Hygiene | Version aus einer Quelle, Changelog, `reports/` entlastet, Community-Dateien |

## Erfolgskriterien

- Release mit mindestens vier Assets plus Prüfsummen, alle Build-Jobs grün
- Docs-only-PR unter 2 Minuten, Rust-only-PR ohne Desktop-E2E, Desktop-PR ohne Rust-Job
- Null direkte Bot-Pushes auf `main`, höchstens ein Metrik-Commit pro Woche
- `nx affected -t lint test build` lokal grün und identisch zu CI
- Kein Merge mit rotem Pflicht-Check
- Repo kleiner um rund 470 Dateien und 2,4 MB Binärlast

## Risiken und Gegenmaßnahmen

| Risiko | Gegenmaßnahme |
|---|---|
| Nx-Plugin-Pflege | nur Orchestrierung, Rust bleibt bei Cargo, kein Community-Rust-Plugin als harte Abhängigkeit |
| Cache-Korrektheit | keine Rust-Targets cachen, `namedInputs` pflegen, Cache bei Verdacht leeren |
| Pflicht-Checks blockieren Dependabot | Auto-Merge erst nach grünen Checks, `mergeStateStatus` beobachten |
| Release benötigt Secrets und Tag-Rechte | Draft bleibt bei Fehlschlag stehen, Publish nur nach allen Builds |
| Ruleset-Änderung betrifft gemeinsame Infrastruktur | Admin-Rechte und explizites GO einholen |
| Nx Cloud als Datenabfluss | später und freiwillig, zuerst lokaler und Actions-Cache |

## Offene Entscheidungen

1. Metrik-Bot: Option A (Bypass), B (PR) oder C (wöchentlicher Branch)? Empfehlung C plus Bypass.
2. Pflicht-Checks: sofort alle fünf oder schrittweise zuerst Rust, Client und TypeSpec?
3. TS-Client: eigenständige Version 1.0.0 beibehalten oder in die Produktversion 0.7.x überführen?
4. Nx Cloud: dauerhaft ablehnen oder nach Messung der Cache-Trefferquote prüfen?

## Nächster Schritt

Phase 0 umsetzen: Plugin-Parität, Release-Workflow, Paritätsskript, Legacy-Workflows entfernen, Metrik-Push reparieren. Tag, Release und Ruleset sind gemeinsame Infrastruktur und benötigen ein explizites GO.
