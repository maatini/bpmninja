# SKILL: NATS Persistence Layer für mini-bpm (Rust)

**Ziel:** Vollständig persistente BPMN-Engine mit NATS (KV + Object Store + JetStream).

## 1. Neue Dateien / Änderungen (genaue Reihenfolge)

### A. Cargo.toml (ergänzen)
```toml
async-nats = { version = "0.38", features = ["jetstream"] }
quick-xml = { version = "0.37", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }