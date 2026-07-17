# persistence

Storage backends implementing the `WorkflowPersistence` trait via the hexagonal port in engine-core. Two implementations: NATS JetStream (production; docker `REQUIRE_NATS=true`) and in-memory (tests + optional local dev fallback).

**Crate paths:** `persistence-nats/` (~4 integration tests, skip without NATS) and `persistence-memory/`

- [responsibility.md](responsibility.md) — What both backends own, their invariants
- [dependencies.md](dependencies.md) — Inbound/outbound, trait relationship
- [interfaces.md](interfaces.md) — Public API for each backend
- [gotchas.md](gotchas.md) — Common pitfalls, NATS-specific concerns
