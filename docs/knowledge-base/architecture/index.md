# Architecture

Core architectural documentation for BPMNinja.

- [components.md](components.md) — Major logical components and their responsibilities
- [dependencies.md](dependencies.md) — Crate-level dependency graph + Mermaid diagram
- [data-flows.md](data-flows.md) — Key data flows: deployment, execution, persistence, SSE, startup, health/ready
- [decisions.md](decisions.md) — ADRs (hexagonal, token model, flattening, lock-free, Rhai, REQUIRE_NATS, bounded retry)

## Quick Reference

| Component | Crate | Role |
|-----------|-------|------|
| Core Engine | `engine-core` | State machine, token execution, gateways, scripting, history — **no I/O** |
| BPMN Parser | `bpmn-parser` | XML → `ProcessDefinition` (quick-xml) |
| REST Server | `engine-server` | Axum HTTP API, SSE, timer scheduler, log buffer, durability gates |
| NATS Persistence | `persistence-nats` | `WorkflowPersistence` impl via NATS JetStream (production) |
| Memory Persistence | `persistence-memory` | In-memory impl for tests / optional local dev |
| Desktop App | `desktop-tauri` | Thin client: React + bpmn-js + Tauri |
| Task Client | `bpmn-ninja-external-task-client` | TypeScript Camunda-compatible worker |
