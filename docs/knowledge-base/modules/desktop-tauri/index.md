# desktop-tauri

Tauri 2 desktop application — a **thin client** that connects to engine-server via HTTP REST and receives push-based real-time updates via SSE. Provides a bpmn-js modeler, instance/definition tracking, and monitoring dashboards.

**Path:** `desktop-tauri/` (separate Cargo project + npm)  
**Source:** `desktop-tauri/src/` (React + TypeScript, ~55 files) + `desktop-tauri/src-tauri/src/` (Rust, ~900 LoC)  
**Tests:** 48 Playwright E2E tests

- [responsibility.md](responsibility.md)
- [dependencies.md](dependencies.md)
- [interfaces.md](interfaces.md)
- [gotchas.md](gotchas.md)
