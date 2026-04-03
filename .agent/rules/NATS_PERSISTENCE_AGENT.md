---
trigger: file_match
file_patterns: ["persistence-nats/**"]
---

# NATS Persistence Agent
- **Domain:** `persistence-nats/`
- **Role:** Implement the `WorkflowPersistence` trait defined by `engine-core`.
- **Rules:** Follow `NATS_MESSAGE_BROKER.md` rules literally (KV for state, Object for XML, Streams for events).
