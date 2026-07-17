# Architecture

Core architectural documentation for BPMNinja.

- [components.md](components.md) — Major logical components and their responsibilities
- [dependencies.md](dependencies.md) — Crate-level dependency graph + Mermaid diagram
- [data-flows.md](data-flows.md) — Key data flows: deployment, execution, persistence, SSE
- [decisions.md](decisions.md) — Key architectural decisions (hexagonal, token model, flattening, lock-free)

## Quick Reference

| Component | Crate | Role |
|-----------|-------|------|
| Core Engine | `engine-core` | State machine, token execution, gateways, scripting, history — **no I/O** |
| BPMN Parser | `bpmn-parser` | XML → `ProcessDefinition` (quick-xml) |
| REST Server | `engine-server` | Axum HTTP API, SSE, timer scheduler, log buffer |
| NATS Persistence | `persistence-nats` | `WorkflowPersistence` impl via NATS JetStream |
| Memory Persistence | `persistence-memory` | In-memory impl for testing/development |
| Desktop App | `desktop-tauri` | Thin client: React + bpmn-js + Tauri |
| Task Client | `bpmn-ninja-external-task-client` | TypeScript Camunda-compatible worker |
