# engine-server

Axum-based HTTP REST API server for BPMNinja. Provides 46 endpoints for process management, task handling, monitoring, file storage, and push-based event streaming via SSE.

**Crate path:** `engine-server/`  
**Source:** `engine-server/src/` (18 files: main.rs, startup.rs, server/ with 12 route modules, log_buffer.rs, log_nats.rs, observability.rs)  
**Tests:** 91 test functions, mostly E2E in `engine-server/tests/` (19 files), incl. auth/CORS, upload limit, `REQUIRE_NATS` readiness, model checking

- [responsibility.md](responsibility.md)
- [dependencies.md](dependencies.md)
- [interfaces.md](interfaces.md)
- [gotchas.md](gotchas.md)
