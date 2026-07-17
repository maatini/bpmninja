# BPMNinja Knowledge Base

> **Audience:** coding agents (AI and human). Start here for any architectural or module-level question.

## Quick Start for Agents

1. Read [overview.md](overview.md) for the big picture.
2. Navigate to the relevant module under [modules/](modules/) for detailed responsibilities, dependencies, and interfaces.
3. Check [architecture/dependencies.md](architecture/dependencies.md) for the crate dependency graph.
4. Use [cross-cutting/tags.md](cross-cutting/tags.md) to decode `@tag:slug` references.

## Directory Map

| Path | Purpose |
|------|---------|
| [overview.md](overview.md) | Project mission, tech stack, workspace structure, key design principles |
| [architecture/](architecture/) | High-level architecture: component map, dependency graph, data flows, key decisions |
| [modules/](modules/) | One folder per major logical module — responsibilities, dependencies, interfaces, gotchas |
| [cross-cutting/](cross-cutting/) | Cross-module patterns, tag registry, shared conventions |
| [maintenance.md](maintenance.md) | When and how to update the knowledge base |

## Module Quick Index

| Module | What it does | Key entry points |
|--------|-------------|-----------------|
| [engine-core](modules/engine-core/) | State machine, token execution, gateways, scripting, history | `WorkflowEngine::deploy_definition`, `start_instance`, `process_timers` |
| [bpmn-parser](modules/bpmn-parser/) | BPMN 2.0 XML → `ProcessDefinition` | `parse_bpmn_xml(xml: &str)` |
| [engine-server](modules/engine-server/) | Axum REST API, SSE events, log buffer, timer scheduler | `build_app_with_engine`, route handlers in `server/` |
| [persistence](modules/persistence/) | NATS JetStream + in-memory storage via `WorkflowPersistence` trait | `NatsPersistence::connect`, `InMemoryPersistence::new` |
| [desktop-tauri](modules/desktop-tauri/) | Tauri + React + bpmn-js desktop app (thin client) | `App.tsx`, Tauri commands in `src-tauri/` |
| [external-task-client](modules/external-task-client/) | TypeScript Camunda-compatible worker client | `ExternalTaskClient`, `TaskService` |
| [agent-orchestrator](modules/agent-orchestrator/) | Example Rust external task worker (stub) | `main.rs` |
| [api-spec](modules/api-spec/) | OpenAPI 3.0 specification (TypeSpec source) | `main.tsp`, `docs/openapi.yaml` |
| [fuzz](modules/fuzz/) | Cargo-fuzz targets for parser, Rhai, cron, tokens, server payloads | 9 fuzz targets in `fuzz/` |

## Key Design Principles

1. **@tag:hexagonal-architecture** — `WorkflowPersistence` trait in `engine-core/src/port/` defines the persistence contract; `engine-core` has zero network/IO code.
2. **@tag:token-execution** — BPMN token flow: `execute_step` → `NextAction` → `run_instance_batch` loop. Tokens live in `ProcessInstance.tokens: HashMap<Uuid, Token>` (single source of truth).
3. **@tag:lock-free-concurrency** — Wait-state queues use `DashMap` (lock-free sharding). Per-instance locking via `InstanceStore` (`Arc<RwLock<ProcessInstance>>`).
4. **@tag:subprocess-flattening** — Embedded sub-processes are flattened at parse time into the main graph. No runtime nesting.
