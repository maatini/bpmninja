---
title: CLI und Build-Befehle
description: Befehle, Argumente und Beispiele für Build, Test, Lint, Format, Engine-Start, Docker Compose, Desktop-UI, External-Task-Client und OpenAPI-Generierung in BPMNinja.
audience:
  - Beitragende und Maintainer
  - Betriebsteams und Plattform-Administratoren
  - Entwickler und Integratoren
prerequisites:
  - Repository lokal ausgecheckt
  - Rust-Toolchain (via rustup) oder Devbox
  - Node.js ≥ 18 für UI und TypeScript-Client
  - Docker und Docker Compose für den Container-Betrieb
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - tutorials/erster-prozess.md
  - how-to/external-task-worker-schreiben.md
  - how-to/konfiguration-anpassen.md
  - reference/konfiguration.md
  - reference/rest-api.md
---

# CLI und Build-Befehle

Diese Seite listet die Befehle, die im Repository belegt sind. Sie ist eine Referenz, kein Tutorial: Für den ersten Durchlauf lies [`tutorials/erster-prozess.md`](../tutorials/erster-prozess.md).

**Beleg:** `bpmninja/README.md` §„Quick Start", §„Build, Test & Lint", §„Starting the Engine Server", §„External Task Client", §„Desktop Application".

---

## 1. Workspace-Struktur (Voraussetzung für alle Befehle)

Der Rust-Root-Workspace (`Cargo.toml`) enthält: `engine-core`, `bpmn-parser`, `persistence-nats`, `persistence-memory`, `engine-server`, `agent-orchestrator`.

Separat verwaltete Module — **nicht** Teil des Root-Workspace:

| Modul | Pfad | Eigenes Build-System |
|---|---|---|
| Desktop-App | `desktop-tauri/src-tauri` | eigenes Cargo-Projekt, gebaut über `desktop-tauri` |
| Fuzzing | `fuzz` | separates `cargo-fuzz`-Workspace |
| TypeScript-Client | `bpmn-ninja-external-task-client` | npm |
| API-Spezifikation | `api-spec` | npm + TypeSpec |

Für eine vollständige lokale Verifikation reichen die Root-Checks **nicht** — du musst die modulspezifischen Checks zusätzlich ausführen.

---

## 2. Build, Test, Lint, Format

| Aktion | Devbox | Direkte Shell |
|---|---|---|
| Build | `devbox run build` | `cargo build --workspace` |
| Test | `devbox run test` | `cargo test --workspace` |
| Lint | `devbox run lint` | `cargo clippy --workspace -- -D warnings` |
| Format prüfen | `devbox run fmt` | `cargo fmt --all --check` |

`devbox shell` installiert Rust, Node.js und NATS automatisch und öffnet eine Shell mit diesen Werkzeugen.

**Hinweise:**

- `cargo clippy … -D warnings` behandelt jede Warnung als Fehler — CI nutzt denselben Aufruf.
- `cargo fmt --all --check` prüft nur; zum Anwenden `cargo fmt --all` ohne `--check`.
- Die NATS-Integrationstests in `persistence-nats/src/tests.rs` **überspringen** sich lokal, wenn NATS nicht erreichbar ist. In CI (`CI=true`) **schlagen sie fehl** statt zu überspringen. GitHub Actions startet NATS mit JetStream vor `cargo test --workspace`.

---

## 3. Engine starten

```bash
# 1. NATS mit JetStream starten
docker compose up -d nats

# 2. Engine-Server starten
cargo run -p engine-server
```

Der Server lauscht auf `http://localhost:8081` (änderbar über `PORT`).

**Mit expliziter Bindung und Keys:**

```bash
BIND_ADDR=127.0.0.1 \
BPMNINJA_API_KEY="$(openssl rand -hex 32)" \
BPMNINJA_WORKER_API_KEY="$(openssl rand -hex 32)" \
cargo run -p engine-server
```

**Prüfen, ob die Engine bereit ist:**

```bash
curl -s http://localhost:8081/api/health
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/ready
```

Alle Variablen mit Default und Wirkung: [`reference/konfiguration.md`](konfiguration.md). Die aufgabenbezogene Einführung mit Prüfschritt nach jedem Block steht in [`how-to/konfiguration-anpassen.md`](../how-to/konfiguration-anpassen.md).

---

## 4. Docker Compose

| Befehl | Zweck |
|---|---|
| `docker compose up -d nats` | Nur NATS starten (lokale Entwicklung). |
| `docker compose up --build` | Gesamten Stack inkl. Engine bauen und starten. |
| `devbox run engine:docker` | Engine-Image über Devbox bauen/starten. |
| `docker compose stop nats` | NATS stoppen — nützlich, um `/api/ready` → `503` zu prüfen. |
| `docker compose start nats` | NATS wieder starten. |

**Portbindung im Compose-Stack:**

| Dienst | Host-Bind | Zweck |
|---|---|---|
| `nats` 4222 | `127.0.0.1:4222` | Client-Protokoll, nur Loopback |
| `nats` 8222 | `127.0.0.1:8222` | HTTP-Monitoring, nur Loopback |
| `cobra-nats` 3000 | `127.0.0.1:3000` | Dashboard, nur Loopback |
| `engine-server` 8081 | `8081:8081` | API, alle Interfaces |

NATS verlangt im Compose-Stack Benutzername/Passwort (lokaler Default `bpmninja`/`bpmninja`, im Compose änderbar). Ein unauthentifiziertes `nats://localhost:4222` schlägt gegen diesen Stack fehl.

**Fertiges Backend-Image:** `ghcr.io/maatini/bpmninja/engine-server:latest` (Multi-Arch: `linux/amd64`, `linux/arm64`).

---

## 5. Desktop-UI

| Aktion | Befehl |
|---|---|
| Dev-Server der UI | `devbox run ui:dev` |
| Tauri-App aus dem Quellcode | `cd desktop-tauri && npm install && npm run tauri dev` |

Die App erwartet das Backend unter `http://localhost:8081` (änderbar über `ENGINE_API_URL`).

**Fertige Binaries:** GitHub Releases (`.dmg` für macOS, `.exe`/`.msi` für Windows, `.AppImage`/`.deb` für Linux).

- macOS: Bei der Meldung „app is damaged" die Quarantäne entfernen: `xattr -cr /Applications/bpmninja-desktop.app`.
- Linux: `.deb` via `sudo dpkg -i package.deb`; `.AppImage` ggf. mit `chmod +x app.AppImage` ausführbar machen.

---

## 6. External-Task-Client (TypeScript)

```bash
cd bpmn-ninja-external-task-client
npm install
npm run lint     # tsc --noEmit
npm run test     # vitest run (68 Tests)
npm run example  # tsx example/simple-worker.ts
```

Das vollständige Beispiel (`send-email`, `validate-order`, `flaky-task`, Shutdown) liegt in `bpmn-ninja-external-task-client/example/simple-worker.ts`.

**TaskService-Methoden pro Handler:**

| Methode | Wirkung |
|---|---|
| `complete(variables?)` | Task erfolgreich abschließen, optional mit Ausgabevariablen |
| `failure(msg, details?, retries?)` | Fehler melden; `retries: 0` erzeugt einen Incident |
| `extendLock(ms)` | Sperre verlängern — **Millisekunden** |
| `bpmnError(errorCode)` | BPMN-Error-Event am angebundenen Boundary auslösen |

Der Client sendet `lockDuration` und `newDuration` unverändert in Millisekunden; Default ist `30_000`. Kanonische Semantik: [`reference/service-task-api.md`](service-task-api.md).

---

## 7. OpenAPI-Spezifikation regenerieren

```bash
cd api-spec
npm install
npx tsp compile .
```

Die generierte Spezifikation `docs/openapi.yaml` ist die **verbindliche Quelle der Wahrheit** für die REST-API (Entscheidung E4 der Informationsarchitektur). Änderungen an Routen oder Schemata gehören in die TypeSpec-Quellen unter `api-spec/`, nicht in die YAML-Datei.

---

## 8. Fuzzing

Das `fuzz`-Modul ist ein separates `cargo-fuzz`-Workspace und wird nicht über `cargo test --workspace` ausgeführt. **Offen:** Die konkreten `cargo fuzz run <target>`-Aufrufe und die Liste der Fuzz-Targets sind in den gelesenen Quellen nicht belegt.

---

## Offen

- **Devbox-Tasknamen:** Belegt sind `devbox shell`, `devbox run build|test|lint|fmt`, `devbox run ui:dev` und `devbox run engine:docker`. **Offen:** Ob `devbox.json` weitere Tasks definiert (z. B. für Fuzzing oder die API-Spezifikation), ist nicht belegt.
- **Fuzz-Targets:** siehe §8.
- **`cargo run -p agent-orchestrator`:** Der Crate ist im Workspace, aber ein dokumentierter Startbefehl oder eine Konfiguration für den Beispiel-Worker fehlt in den gelesenen Quellen. **Offen.**
- **Testzahlen:** `bpmninja/README.md` nennt ~322 Unit-, ~109 E2E- und ~431 Gesamttests; `docs/architecture.md` §10 nennt ~17 062 LoC Rust-Workspace und ~22 871 LoC gesamt. Die Zahlen widersprechen sich (Audit AUD-008) und sind hier bewusst nicht als Zusage wiederholt.
- **Versionsstand:** `bpmninja/README.md` nennt Version `0.7.24` für Rust/Desktop und `1.0.0` für den npm-Client. Eine Zuordnung dieser Seite zu einem Versionsstand steht aus.
