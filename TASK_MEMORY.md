# TASK_MEMORY — Phase 4a Performance-Quick-Wins (P0, P1, P3, P4, P5)

## Ziel
Vermeidbare Wartezeiten und NATS-Roundtrips aus dem Hot Path entfernen — gemessen vorher/nachher.

## Graph
- God Nodes: `deploy_definition()`, `start_instance()`, `NatsPersistence`, `fetch_and_lock_service_tasks()`
- graphify/devbox lokal nicht installiert → Graph nicht neu gebaut

## Umgebung
- Docker nicht installiert → `nats-server` v2.11.9 via `go install` (~/go/bin), läuft auf 127.0.0.1:4222 ohne Auth
- Datenverzeichnis im Scratchpad

## Checkliste
- [ ] P0: Benches erweitern (engine-core: Gateway/Script/XOR; persistence-nats: nats_bench) + Baseline
- [ ] P1: NATS KV/ObjectStore-Handles beim Connect cachen
- [ ] P3: persist_instance ohne Instanz-Lock über Netzwerk-Await; Persist-Lock pro Instanz; Tests
- [ ] P4: Notify für Service-Tasks; Long-Poll in server/tasks.rs; Test
- [ ] P5: next_due + Notify; Scheduler in main.rs; Test
- [ ] fmt, clippy, test --workspace; Benches nachher
- [ ] KB + improvements.md aktualisieren
- [ ] TASK_MEMORY.md löschen

## Baseline (criterion `--save-baseline before`, lokal Windows, NATS lokal)
| Bench | Median |
|---|---|
| Linear 10 / 50 / 100 Service-Tasks (in-memory) | 328 µs / 2.45 ms / 7.43 ms |
| Parallel fork/join 4 / 16 Script-Branches | 2.54 ms / 10.5 ms |
| Script chain (10) | 6.61 ms |
| XOR routing | 15.6 µs |
| NATS save_instance | 496 µs |
| NATS save_token | 467 µs |
| NATS append_history_entry | 16.6 µs (Publish ohne Ack-Await!) |
| NATS engine: start → 5 Scripts → end | 7.78 ms |

Auffällig: Script-Tasks ~0.6 ms/Stück (Engine-Neubau + spawn_blocking) → P6 lohnt sich.

## Letzter Stand
P0 erledigt (Benches + Baseline). Beginn P1.
