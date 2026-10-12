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

## Baseline
(folgt)

## Letzter Stand
NATS gestartet, Beginn P0.
