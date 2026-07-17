# engine-server

Axum-based HTTP REST API server for BPMNinja. Provides 38 endpoints for process management, task handling, monitoring, file storage, and push-based event streaming via SSE.

**Crate path:** `engine-server/`  
**Source:** `engine-server/src/` (18 files: main.rs, startup.rs, server/ with 12 route modules, log_buffer.rs, log_nats.rs, observability.rs)  
**Tests:** 55 E2E tests in `engine-server/tests/` (15 files)

- [responsibility.md](responsibility.md)
- [dependencies.md](dependencies.md)
- [interfaces.md](interfaces.md)
- [gotchas.md](gotchas.md)
