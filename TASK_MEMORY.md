# TASK_MEMORY — Phase 4a Teil 2 (Korrektheit + P6 + P2) & Bewertung externer Architekturvorschläge

## Ziel
History-Ack + DashMap-Guard-Fix, Rhai-Engine/AST wiederverwenden (P6), History pro Batch (P2);
eingefügte Architektur-Vorschläge (Kafka-Saga-Log, WS-BPEL, AOT/FFI, Zeebe, Model-Checking) bewerten und Backlog ergänzen.

## Graph
- God Nodes: `start_instance()`, `NatsPersistence`; betroffen: executor/mod.rs, persistence_ops.rs, scripting/runner.rs, service_task.rs, trait_impl.rs
- graphify lokal nicht installiert

## Checkliste
- [ ] Baseline `before2` (engine_bench + nats_bench)
- [ ] Fix A: History-Publish wartet auf JetStream-Ack (+ Batch-Methode `append_history_entries`, Default-Impl im Trait)
- [ ] P2: History-Einträge pro `run_instance_batch` sammeln, einmal flushen (auch auf Fehlerpfaden); Snapshot nur bei Persistenz
- [ ] Fix B: `fail_service_task` / `retry_incident` ohne DashMap-Guard über `.await`
- [ ] P6: `ScriptRunner` (geteilte `rhai::Engine` + AST-Cache), Aufrufer umstellen, Tests
- [ ] fmt, clippy, test --workspace (NATS), Benches gegen `before2`
- [ ] Docs/README/improvements.md (+ Bewertung der Vorschläge)
- [ ] TASK_MEMORY.md löschen

## Letzter Stand
NATS läuft. Baseline läuft.
