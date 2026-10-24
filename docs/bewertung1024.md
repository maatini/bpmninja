# Projektbewertung BPMNinja — 2026-10-24

**Gesamtscore: 70 / 100**

Momentaufnahme: `main` @ `83b2075`, Arbeitsbaum clean, Toolchain 1.99.0 (Devbox).
Bewertet wurden Architektur, Code-Qualität, BPMN-Semantik, Tests, Persistenz/Betrieb, Security, API/Integrationen, Desktop-UI, Dokumentation sowie Release-/Projektzustand.

**Kurzfazit:** Technisch überdurchschnittlich sauber gebaut und funktional breit, aber mit einem kritischen BPMN-Laufzeitfehler, fehlender Betriebsreife für mehrere Knoten und einem Release-/Governance-Zustand, der den Produktionsanspruch noch nicht trägt.

## Score-Übersicht

| Kategorie | Gewicht | Score |
|---|---:|---:|
| Architektur & Modularität | 15 % | **79** |
| Code-Qualität (Rust) | 12 % | **84** |
| BPMN-Semantik & Funktionsabdeckung | 18 % | **62** |
| Tests & Qualitätssicherung | 12 % | **69** |
| Persistenz & Betrieb | 10 % | **66** |
| Security & Robustheit | 10 % | **61** |
| REST-API & Integrationen | 8 % | **75** |
| Desktop-UI | 6 % | **71** |
| Dokumentation | 5 % | **67** |
| Release, CI/CD & Projektzustand | 4 % | **52** |
| **Gesamt (gewichtet)** | **100 %** | **70** |

---

## Architektur & Modularität — 79/100

**Stark:** Klare hexagonale Trennung. `engine-core` enthält keinerlei I/O (verifiziert: kein `std::fs`, `reqwest`, `async_nats`), sechs Crates mit eindeutiger Abhängigkeitsrichtung, ADRs dokumentieren die Entscheidungen inklusive ihrer Nachteile. Executor, Handler, Gateways und Events sind sauber in eigene Module geschnitten. Kein `unsafe` im gesamten Produktionscode.

**Schwach:** Der Persistence-Port hat **35 async-Methoden**; jede neue Persistenzoperation trifft beide Adapter. `persistence-memory` ist nur ein 122-Zeilen-Re-Export des In-Memory-Adapters. Der autoritative Zustand liegt in Prozess-DashMaps — das schließt horizontale Skalierung architektonisch aus. Das Subprozess-Flattening weicht bewusst vom Standard ab und ist aktuell fehlerhaft (siehe BPMN).

## Code-Qualität (Rust) — 84/100

**Stark:** `cargo fmt --check` sauber, `cargo clippy --workspace --all-targets -- -D warnings` ohne Befund, 452 Tests grün. Typisiertes `thiserror`-Fehlermodell, 973 Doc-Kommentare, keine `TODO`/`FIXME`, kein `println!` im Produktionscode, verschwindend wenige `unwrap`/`expect` außerhalb von Tests (Server: 6, alle Boot-/Header-Pfade).

**Schwach:** 26 `#[allow(...)]`, überwiegend `dead_code` in den Parser-Modellen. Im Frontend 111 `any`/`@ts-ignore`-Stellen. Das JS-Bundle ist 2,56 MB (722 kB gzip) ohne Code-Splitting.

## BPMN-Semantik & Funktionsabdeckung — 62/100

**Stark:** Token-Executor mit Schrittlimit, XOR/AND/OR/EventBased/Complex, Timer inklusive Cron und ISO-Wiederholung (`R3/PT…`), Boundary-Events interrupting/non-interrupting, Error/Escalation/Compensation mit LIFO, Call Activities, Incident-Lifecycle, parallele Multi-Instance auf Service-Tasks. 324 Engine-/Parser-Tests decken viele Randfälle ab.

**Kritisch:** Ein eingebetteter Subprozess **bleibt zur Laufzeit hängen**. Der Parser ersetzt das interne Start-Event durch `ServiceTask { topic: "noop" }` (`bpmn-parser/src/parser.rs:851`), der Handler springt genau dorthin (`engine-core/src/engine/handlers/sub_processes.rs:49`), und es gibt keine Sonderbehandlung. Eigene Reproduktion: Zustand `WaitingOnServiceTask`, kein User-Task. Dafür existiert kein Laufzeittest.

**Weitere Lücken:**
- Event-Subprozesse werden ohne Trigger geflattet.
- Signal-, Conditional-, Link- und Cancel-Events fehlen; Signal am Start-Event wird still zum normalen Start degradiert.
- Die Bedingungssprache kennt kein `&&`/`||` und kein `${…}` (`engine-core/src/condition.rs`), wodurch Camunda-Exporte still auf den Default-Pfad laufen.
- Multi-Instance nur parallel auf Service-Tasks.
- Script-Fehler erzeugen keinen Incident.
- `ScopeEventListener` ist toter Code, obwohl das README ihn bewirbt.

## Tests & Qualitätssicherung — 69/100

**Stark:** 452 Rust-Tests über 32 Suites, 77 Vitest-Tests im TS-Client, 52 Playwright-Tests, 9 Fuzz-Targets (9/9 grün), Mutation-Score 68,9 % (406 gefangen, 182 überlebt). Testcode 14,9k LOC gegen 21,7k LOC Produktionscode — gutes Verhältnis.

**Schwach:** Mutation 68,9 % ist mittelmäßig, Coverage wird nicht gemessen. NATS-Tests kehren ohne laufenden Server still zurück, statt zu skippen — lokal wirken sie grün, obwohl sie nicht liefen. Für den Subprozess-Bug existiert kein Test. Playwright mockt `__TAURI_INTERNALS__` vollständig, es gibt keinen Contract-Test Client↔Server. Lokal schlagen **2 von 52** E2E-Tests fehl (`Deployed`-Toast leer, 2-Sekunden-Wartezeit im Helper) — Testrobustheit, kein belegter Produktfehler.

## Persistenz & Betrieb — 66/100

**Stark:** Fail-closed NATS-Bootstrap, idempotentes Stream-Reconciling, bounded Retry-Queue mit Backoff und Persistence-Incident, Prometheus-Metriken, rollierender Log-Buffer, `/api/health` und `/api/ready`, Graceful Shutdown mit Drain-Budget.

**Schwach:** Single-Node ohne Leader-Election — der Timer-Scheduler läuft pro Prozess, was bei Skalierung Doppelverarbeitung erzeugt. Keine Backup-/Restore-Strategie. KV-Buckets und Object Store sind unrepliziert und unbegrenzt. Multi-Entity-Writes (Instanz, Task, Timer, History) sind nicht atomar, Löschfehler werden teils ignoriert. History-Queries erzeugen einen Consumer pro Request.

## Security & Robustheit — 61/100

**Stark:** API-Key mit Worker-Rolle, konstantzeitnaher Vergleich, fail-closed bei Nicht-Loopback-Bind, CORS-Allowlist, 10-MiB-Body-Limit, Header-Injection-Schutz, `cargo-deny` mit Lizenz-Allowlist und leerer Advisory-Ignore-Liste.

**Schwach:** Ausschließlich HTTP, ein statischer Key ohne Rotation, kein Rate-Limiting. **NATS-Zugangsdaten werden im Klartext geloggt** (`engine-server/src/main.rs:96`) und landen über den Log-Buffer in `/api/logs`. Compose nutzt Default-Credentials `bpmninja/bpmninja`. Das Docker-Image läuft als root, ohne `HEALTHCHECK`, mit ungepinnten Base-Tags. Die Tauri-CSP erlaubt `unsafe-eval` sowie beliebiges `http:`/`https:`/`ws:`.

## REST-API & Integrationen — 75/100

**Stark:** 45 Routen inklusive Suspend/Resume, Migration, Token-Move, Files, History und Incidents. Einheitliches `{error}`-Format, Paginierung mit `X-Total-Count`, SSE mit Keep-Alive. Der TS-Client kann Long-Polling, Multi-Topic, Retry/Backoff und Lock-Extension.

**Schwach:** OpenAPI driftet trotz TypeSpec: Incident-Liste fehlt, `DeployResponse.version`, `startLatest`-Felder und History-Modelle stimmen nicht. Der Desktop sendet `actorTypes`, der Server liest `actor_type` — der Filter greift nie. Der TS-Client hat kein Timeout/Retry auf `complete`/`failure`, keinen Integrationstest gegen den echten Server und ein unvollständiges npm-Paket (kein `files`-Feld, Version 1.0.0 abweichend vom Workspace).

## Desktop-UI — 71/100

**Stark:** Neun Seiten, bpmn-js-Modeler mit fünf eigenen Property-Providern, SSE plus Polling, Offline-Banner, ErrorBoundary, Skeleton-/Empty-Zustände, 2.444 Zeilen Playwright-Tests. Build und Typecheck laufen durch.

**Schwach:** 111 Typ-Lockerungen, IPC-Commands liefern meist `serde_json::Value`, Frontend-Typen kennen Zustände wie `ParallelExecution` oder `CompletedWithError` nicht. Incidents werden clientseitig aus `retries <= 0` abgeleitet statt über die API. API-Key und URL leben nur im Speicher.

## Dokumentation — 67/100

**Stark:** 726-Zeilen-README mit Elementtabelle, Abweichungen und Nicht-Support-Liste; Knowledge Base mit 3.412 Zeilen über 52 Dateien, pro Modul Responsibility/Dependencies/Interfaces/Gotchas, plus ADRs und Mermaid-Diagrammen.

**Schwach:** Messbarer Drift. Testzahlen 217/222 gegenüber 452 tatsächlich. Multi-Instance ist als „nicht unterstützt“ dokumentiert und teilweise implementiert. Die Env-Tabelle nennt 8 von rund 20 Variablen. Stack-Versionen (Rhai, async-nats, quick-xml, Tauri) sind veraltet. `devbox run graphify` ist weiterhin als Befehl dokumentiert und funktioniert nicht.

## Release, CI/CD & Projektzustand — 52/100

**Stark:** Breite CI (fmt, clippy, Tests gegen echten NATS, TS-Lint/Tests, TypeSpec, cargo-deny, Playwright), Fuzzing mit Sanitizern, Mutation-Job, native Multi-Arch-Images. CI auf `main` ist grün.

**Schwach:** **Release `v0.7.24` hat null Assets** (verifiziert via `gh release view`); alle vier Tauri-Build-Jobs scheitern am Crate/npm-Versionsmismatch. Der Branch-Schutz hat **keine Required Status Checks** und 0 Approvals — rote PRs können mergen. `Update Quality Metrics` schlägt seit Stunden wiederholt fehl. 121 der letzten 200 Commits sind Bot-Metrik-Commits. 15 MB / 479 Dateien liegen in `reports/`, darunter 20 `.lance`-Binärdateien. CONTRIBUTING, SECURITY, CHANGELOG und CODE_OF_CONDUCT fehlen. Actions sind auf Major-Tags statt SHA gepinnt.

---

## Die fünf größten Hebel

1. **Subprozess-Laufzeit reparieren** — internes Start-Event nicht als `noop`-ServiceTask materialisieren, plus Laufzeittest. Das blockiert ein Kernelement, das das README als unterstützt ausweist.
2. **Release-Pipeline grün bekommen** — Tauri-Crate und npm-Pakete auf gleiche Major.Minor bringen, Desktop-Build als CI-Gate, `v0.7.25` mit Assets.
3. **Branch-Schutz scharf stellen** — Required Checks (CI, Desktop, cargo-deny) eintragen, Metrik-Bot über PR statt Direkt-Push.
4. **Security-Hygiene** — NATS-URL ohne Credentials loggen, Non-Root-Container, TLS/Rate-Limit dokumentieren, SECURITY.md.
5. **Doku und Code synchronisieren** — Testmetriken generieren, Env-Tabelle vervollständigen, Multi-Instance-Status und Graphify-Befehl korrigieren.

## Verifikation

Selbst ausgeführt am 2026-10-24:

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (452 grün) unter Devbox-Rust 1.99.0.
- `npm test` im External-Task-Client (77 Vitest-Tests grün).
- `npm run test:e2e` im Desktop (50/52 grün), `npm run build` und Typecheck.
- Subprozess-Reproduktion über ein separates Probe-Projekt in `/tmp`.
- `gh release view v0.7.24`, Ruleset-API, CI-Läufe auf `main`.
- Git- und Dateistatistiken (LOC, Testverhältnis, tracked files, `reports/`).

Nicht verifiziert: Last-/Performance-Messungen, echter Tauri-Fenstertest, NATS-Integration lokal (nur CI).
