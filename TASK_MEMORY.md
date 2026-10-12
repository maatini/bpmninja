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
- [ ] V1: `rust_bpmn_analyzer` (git, MIT, pinned rev) als Deploy-Modellprüfung (POR, spawn_blocking + Timeout, warn-only default, Env `MODEL_CHECK=off|warn|enforce`); deny.toml allow-git
- [ ] Übrige Tools: als native Features in Backlog (snurr → E1, bpm-engine → F3 Replay, bpmn-engine → JSON-Definitionsformat, ebi_bpmn → Simulation); bpmncheck nicht (Überschneidung)
- [ ] fmt, clippy, test --workspace (NATS), Benches gegen `before2`
- [ ] Docs/README/improvements.md (+ Bewertung der Vorschläge)
- [ ] TASK_MEMORY.md löschen

## Baseline `before2`
Linear 10/50/100: 287 µs / 3.10 ms / 10.1 ms · Parallel 4/16: 2.30 / 10.3 ms · Script chain 10: 6.54 ms · XOR: 16.6 µs
NATS save_instance 224 µs · save_token 224 µs · history 15.9 µs (ohne Ack) · Engine+NATS 5 Scripts: 4.95 ms

## Letzter Stand
Baseline fertig, Bewertung in improvements.md. Beginn Fix A.
