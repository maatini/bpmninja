# BPMNinja — Priorisierte Verbesserungen

**Datum:** 2026-07-17  
**Audit-Version:** Crates/Desktop `0.7.18` (Claude.md nannte teils noch `0.7.10`)  
**Gesundheit gesamt:** 7.2 / 10  

Quelle: Architektur- und Codebase-Audit (statische Analyse). Unabhängig von `docs/PROJECT_EVALUATION.md` (8.8/10); hier bewusst stärker auf Produktionsrisiken gewichtet.

Dieses Dokument hält die **konkreten, priorisierten Verbesserungsmöglichkeiten** fest. Es ersetzt keine Feature-Roadmap, sondern bündelt technische Schulden, Risiken und High-Leverage-Maßnahmen aus dem Audit.

---

## Executive Summary

BPMNinja ist eine gut architekturierte, disziplinierte schlanke BPMN-2.0-Engine: hexagonaler Core, Token-Ausführung, Camunda-ähnliche External Tasks, NATS-Persistenz, SSE, solides Test-/Fuzz-Setup.

**Nicht** produktionsgehärtet als Multi-Tenant- oder internet-exponierte Workflow-Plattform: REST-API ohne Auth, CORS offen, File-Uploads ohne Größenlimit, stiller In-Memory-Fallback bei NATS-Ausfall (Datenverlust nach Restart), Rhai-`max_memory` konfiguriert aber nicht angewandt.

| Sicht | Score (ca.) |
|-------|-------------|
| Library / Engine-Core | ~8.0 |
| Deploy als Process-Platform „morgen“ | ~6.0 |
| Gesamt (dieses Audit) | **7.2** |

---

## Top 5 (höchster Hebel zuerst)

1. **Offene Control-Plane schließen:** Auth-Middleware + CORS-Allowlist + Upload-Limits.
2. **Fail-closed bei Durability:** `REQUIRE_NATS` / Persistence Pflicht außerhalb Dev; Readiness muss Durability spiegeln.
3. **Rhai `max_memory` wirklich anwenden** — kleinster Code-Fix mit echtem Security-Impact.
4. **`persistence-nats` integrationstesten** — der Produktionspfad ist die dünnste kritische Schicht (~2 Tests).
5. **Retry-Queue begrenzen + Quality-Metrics reparieren** — Betriebsstabilität und vertrauenswürdige Qualitäts-Gates.

---

## Kritische Issues (Kurzreferenz)

| ID | Problem | Ort (Beispiele) | Severity |
|----|---------|-----------------|----------|
| C1 | Keine AuthN/AuthZ auf der gesamten API | `engine-server/src/server/mod.rs` | Blocker bei Exposure |
| C2 | Silent in-memory Fallback bei NATS-Ausfall | `engine-server/src/main.rs` | Datenverlust-Footgun |
| C3 | Unbegrenzter File-Upload (DoS) | `engine-server/src/server/files.rs` | High |
| C4 | CORS `Any` | `engine-server/src/server/mod.rs` | High (mit C1) |
| C5 | Rhai `max_memory` tot / nicht angewandt | `engine-core/src/scripting/runner.rs` | High (Sandbox) |
| C6 | Single-Node; kein Multi-Instance-Locking | Engine + NATS; Roadmap planned | HA-Blocker |
| C7 | `engine_logs.jsonl` in Git getrackt | Repo-Root | Medium (Hygiene/Leak) |
| C8 | Unsanitized Filename in `Content-Disposition` | `files.rs` | Medium |
| C9 | Mutation-Badge 0.0% / kaputte Metrics | `docs/quality-badges.json` | Medium (Signal) |

### Details

**C1 — Keine Auth:** Wer `:8081` erreicht, kann deployen, starten/stoppen/migraten, Tasks complete, Files uploaden, Tokens bewegen, Definitionen löschen, History/Logs/Metrics lesen. OIDC steht in der README als „Planned“.

**C2 — Silent In-Memory:** Bei `NatsPersistence::connect`-Fehler → `WorkflowEngine::new()` ohne Persistenz. `/api/health` bleibt OK; nach Restart ist alles weg. Docker-Compose ohne Fail-Fast.

**C3 — Upload DoS:** Multipart wird per `field.bytes()` vollständig gepuffert; JSON-Body-Limit 5 MB gilt nicht für Multipart.

**C5 — Rhai Memory:** `ScriptConfig.max_memory` / `RHAI_MAX_MEMORY_BYTES` existieren und sind dokumentiert; `build_engine()` setzt Ops/Timeout/Collection-Limits, ruft aber kein Memory-Cap auf.

---

## Quick Wins

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| Q1 | Rhai `max_memory` in `build_engine()` verdrahten + Unit-Test | High | Low | Low | Docs versprechen 2 MiB Cap; aktuell nicht angewandt (`scripting/runner.rs`). |
| Q2 | Fail-fast-Flag (`REQUIRE_NATS` / `REQUIRE_PERSISTENCE`), in Docker-Docs default-on | High | Low | Low | Verhindert stillen Datenverlust-Pfad. |
| Q3 | Hartes Größenlimit für Multipart-Uploads + Ablehnung | High | Low | Low | Stoppt trivialen DoS über `files.rs`. |
| Q4 | CORS per Env-Allowlist (kein `Any` im Prod-Profil) | High | Low | Low | Browser-Missbrauch zusammen mit fehlender Auth. |
| Q5 | `engine_logs.jsonl` aus Git entfernen, `.gitignore`, Rotation dokumentieren | Medium | Low | Low | Runtime-Log (~1.4 MB) getrackt; Leak-/Noise-Risiko. |
| Q6 | Claude.md / README-Version auf `0.7.18` syncen (oder generieren) | Medium | Low | None | Docs-Drift → Vertrauensverlust. |
| Q7 | Mutation-Badge / quality-metrics-Pipeline reparieren | Medium | Medium | Low | Öffentliches Signal 0.0% ist falsch. |

---

## Medium-term Improvements

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| M1 | API-Auth (v1: API-Key oder JWT; danach OIDC) | High | Medium | Medium | Ohne Auth kein ehrlicher „Production API“-Anspruch. |
| M2 | NATS-Integrationstests (testcontainers): Restore, Retry, Partial Failure | High | Medium | Low | `persistence-nats` hat nur ~2 Tests. |
| M3 | Retry-Queue bounden + Dead-Letter + Alert-Metric bei Job-Verwurf | High | Medium | Medium | `mpsc::unbounded_channel` in `retry_queue.rs` → OOM bei NATS-Ausfall + Last. |
| M4 | Topic-Index für Service Tasks + Timer-Prioritätsstruktur | Medium | Medium | Medium | Full-Scan von DashMaps skaliert nicht auf Zehntausende Tasks/Timer. |
| M5 | `unit_tests.rs` (~5076 Zeilen) nach Domänen splitten | Medium | Medium | Low | Wartbarkeit, Merge-Konflikte, Navigation. |
| M6 | `cargo deny` / audit in CI; Line-Coverage publizieren | Medium | Low–Med | Low | Supply-Chain + transparente Qualitätsmetriken. |
| M7 | `Content-Disposition`-Filenames sanitizen | Medium | Low | Low | Header-Injection-Risiko in `files.rs`. |
| M8 | Health vs Readiness: Readiness failt, wenn required Persistence fehlt | High | Low | Low | K8s-tauglich; silent in-memory bleibt sonst „healthy“. |

---

## Strategic / Larger Refactors

| # | Maßnahme | Impact | Effort | Risiko | Beschreibung / Warum |
|---|----------|--------|--------|--------|----------------------|
| S1 | Multi-Node-Cluster (Leader für Timer + distributed Task-Lock/Fencing) | High | High | High | Zwei Server-Instanzen gegen dasselbe NATS = Split-Brain. |
| S2 | `WorkflowPersistence` splitten (Core / History / Files / Monitoring) | Medium | High | Medium | 35 async Methods auf einem Trait — ISP-Druck, teure Backends. |
| S3 | Stärkeres Konsistenzmodell (Batch/Journal oder Event Sourcing) | High | High | High | Multi-KV-Writes sind nicht atomar; Crash → Orphans. |
| S4 | FEEL/Expression-Pfad oder explizite Camunda-Kompatibilitätsmatrix | Medium | High | Medium | `condition.rs` = simple Comparisons; komplexe Logik über Rhai. |
| S5 | Volles OIDC + RBAC (Deploy vs Operate vs Work) | High | High | Medium | Multi-Tenant / Enterprise. |

---

## Empfohlener Aktionsplan (phasiert)

### Phase 0 — Sofort (1–3 Tage) — ✅ erledigt (2026-07)

1. ~~**Q1** Rhai Memory-Limit verdrahten + Tests~~ ✅  
2. ~~**Q3** Upload-Größenlimit~~ ✅  
3. ~~**Q2 / M8** Fail-fast Persistence + Readiness-Semantik~~ ✅  
4. ~~**Q5** Getrackte Logs entfernen / gitignore~~ ✅  
5. ~~**Q6** Versions-Docs syncen~~ ✅  

### Phase 1 — Production Minimum Bar (1–2 Wochen)

1. **M1** API-Authentifizierung (API-Keys als v1 OK; OIDC danach) — offen  
2. **Q4** CORS per Env absichern — offen  
3. ~~**M3** Bounded Retry + Metrics~~ ✅  
4. ~~**M2** NATS-Integrationstests (Restore/Retry)~~ ✅  
5. ~~**Q7** Mutation-Metrics-Pipeline reparieren~~ ✅ (Parser fix; Badge aktualisiert sich beim nächsten Mutation-CI-Lauf)

### Phase 2 — Scale & Maintainability (2–6 Wochen)

1. **M4** Indexierung Timer/Topics  
2. **M5** Mega-Tests splitten  
3. **M6** Supply-Chain-CI  
4. Optional: OpenTelemetry-Traces  
5. `agent-orchestrator`: ausbauen oder archivieren  

### Phase 3 — Platform (Quartal+)

1. **S1** Multi-Node Design & Implementation  
2. **S2 / S3** Persistence-Modell, falls Multi-Node es erzwingt  
3. **S5** RBAC  
4. Batch-Operationen (bereits auf Roadmap)  

---

## Weitere Audit-Befunde (Kontext)

### Stärken (kurz)

- Echte hexagonale Architektur (`WorkflowPersistence`-Port)
- Klares Token-Modell (`NextAction`, closed `BpmnElement`)
- Breite BPMN-Abdeckung für schlanke Engine
- Retry-Queue, SSE, Prometheus, Fuzz, Dependabot, Knowledge Base
- External-Task-Client mit guter Testabdeckung

### Score-Dimensionen (Audit)

| Dimension | Score |
|-----------|-------|
| Architecture & Design | 8.0 |
| Code Quality & Maintainability | 7.5 |
| Performance & Scalability | 6.5 (Single-Node OK) |
| Security (exposed) | 3.5 |
| Security (local/dev tool) | 6.5 |
| Testing Strategy | 8.0 (NATS-Adapter dünn) |
| Error Handling / Observability | 8.0 |
| Dependency / Tech Stack | 7.5 |
| Documentation / DX | 7.5 |

### Performance-Hotspots

1. `fetch_and_lock_service_tasks` — Full-Scan `pending_service_tasks`
2. `process_timers` — Full-Scan aller Timer pro Intervall (default 1s)
3. `list_instances` / Startup-Restore — große Mengen in Memory
4. History-Queries — Filter/Sort in Memory
5. Unbounded Retry-Queue
6. SSE Broadcast Capacity 256 — langsame Clients verlieren Events (by design)

### Was das Audit nicht vollständig bewiesen hat

- Load-Benchmarks (Benches existieren, nicht neu gelaufen)
- Vollständiges Desktop-Tauri CSP/Capability-Review
- Live Multi-Process-Race gegen NATS
- Feldgenaue OpenAPI-Vollständigkeit jeder Route
- Playwright-E2E in CI auf allen Branches

---

## Was bewusst nicht in diesem Dokument steht

- Feature-Wünsche ohne technischen Schuld-/Risikobezug (außer Roadmap-Verweise)
- Reine Lob-Abschnitte im Detail — siehe Audit-Stärken oben
- Implementierungsdetails der Lösungen (Backlog, keine Spec)

---

## Appendix — Größen-Snapshot

| Komponente | Ca. Größe |
|------------|-----------|
| engine-core | ~17.4k LOC / 56 rs files |
| engine-server | ~5.3k LOC / 33 files |
| bpmn-parser | ~2.5k LOC |
| persistence-nats | ~1.3k LOC |
| persistence-memory | ~0.4k LOC |
| agent-orchestrator | ~0.1k LOC |
| desktop-tauri (src) | ~10k TS/TSX/RS |
| external-task-client | ~2–2.6k TS |
| Größte Testdatei | `unit_tests.rs` ~5076 Zeilen |
| Größte Prod-Dateien | `trait_impl.rs` ~984, `parser.rs` ~902, `instance_ops.rs` ~848 |

---

**Nächster sinnvoller Schritt:** Rest von Phase 1 — **M1** API-Auth und **Q4** CORS-Allowlist; danach Phase 2 (Indexierung, Test-Split, Supply-Chain-CI).
