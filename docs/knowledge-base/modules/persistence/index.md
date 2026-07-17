# persistence

Storage backends implementing the `WorkflowPersistence` trait via the hexagonal port in engine-core. Two implementations: NATS JetStream (production) and in-memory (testing/development).

**Crate paths:** `persistence-nats/` and `persistence-memory/`

- [responsibility.md](responsibility.md) — What both backends own, their invariants
- [dependencies.md](dependencies.md) — Inbound/outbound, trait relationship
- [interfaces.md](interfaces.md) — Public API for each backend
- [gotchas.md](gotchas.md) — Common pitfalls, NATS-specific concerns
