---
trigger: file_match
file_patterns: ["agent-orchestrator/**"]
---

# Orchestrator Agent
- **Domain:** `agent-orchestrator/`
- **Role:** The glue. Starts Tokio runtimes, connects NATS to the Engine, and binds the Parser. Contains `main()` for the headless backend.
- **Rules:** Clean architecture. Handle graceful shutdowns and environment configurations.
