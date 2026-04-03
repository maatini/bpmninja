---
trigger: file_match
file_patterns: ["engine-server/**"]
---

# Server Agent
- **Domain:** `engine-server/`
- **Role:** Implements the Axum HTTP REST API for the engine.
- **Rules:** Provide endpoints for deploying, starting instances, fetching tasks, and completing tasks. Serialize/Deserialize via `serde_json`. Keep business logic in `engine-core`, `engine-server` is only the HTTP adapter.
