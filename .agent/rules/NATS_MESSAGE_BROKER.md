---
trigger: always_on
---

# NATS Message Broker Rules (mini-bpm Projekt)

Du bist Experte für NATS + Rust Persistence in Workflow-Engines.
Immer wenn der User NATS-Integration, Persistenz oder verteilte State-Speicherung verlangt, folge diesen Regeln **exakt**.

## 1. Technologie-Stack (fest)
- Crate: `async-nats = { version = "0.38", features = ["jetstream"] }` (aktuellster Stand 2026)
- Connection: `nats://localhost:4222` (nats-server läuft via devbox)
- Immer `tokio::spawn` für Hintergrund-Tasks (Watch, Timer, etc.)
- Error-Handling: erweitere `EngineError` mit `NatsError` Varianten

## 2. Perfekte Aufteilung der NATS-Features (NIE anders!)
| Feature          | Bucket/Stream-Name       | Verwendung in mini-bpm                              | Warum genau diese Wahl? |
|------------------|--------------------------|-----------------------------------------------------|-------------------------|
| **Object Store** | `bpmn_xml`              | Original BPMN 2.0 XML (unveränderlich)             | Für große Artefakte, Chunking, Versionierung |
| **KV Store**     | `definitions`           | ProcessDefinition (JSON)                           | Schnelle Reads/Writes, Watch möglich |
| **KV Store**     | `instances`             | ProcessInstance + Token + Variables + Audit-Log    | State der laufenden Prozesse |
| **KV Store**     | `user_tasks`            | PendingUserTask                                    | Pending-Tasks für externe Completion |
| **JetStream**    | Stream `WORKFLOW_EVENTS`| Subjects: `workflow.deploy`, `workflow.start`, `workflow.complete`, `workflow.timer` | Audit, Monitoring, zukünftige Event-Sourcing |

## 3. Wichtige Prinzipien
- **Bei jedem Schreibzugriff** → sofort in KV + Event in JetStream publishen (atomar).
- **Beim Engine-Start** → vollständiger State-Restore aus KV + Object Store.
- **BPMN 2.0 XML** immer im Object Store + Metadaten im KV `definitions`.
- **In-memory Cache** nur für Tests (Feature-Flag `in-memory`).
- **Keine breaking Changes** am bestehenden `WorkflowEngine` Public API.
- **Minimal & idiomatisch**: `Arc<NatsPersistence>`, async überall, `serde_json` für Serialisierung.
- **Tests** müssen weiterhin 100 % funktionieren (in-memory fallback).

## 4. Verboten
- Kein Redis, PostgreSQL oder andere DBs.
- Kein manuelles Stream-Management ohne `jetstream::new(client)`.
- Kein Blockieren des Tokio-Runtime (immer `.await`).
- Keine großen Binaries im KV-Store (→ Object Store!).

Diese Rules überschreiben alle anderen Rules, wenn NATS erwähnt wird.
