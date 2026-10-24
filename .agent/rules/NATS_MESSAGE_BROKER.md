---
trigger: file_match
file_patterns: ["persistence-nats/**"]
---

# NATS-/Persistenz-Regeln

Technologie ist fest: `async-nats = "0.50"` mit `jetstream`. Lokal läuft `nats-server` über Devbox
oder Compose (`nats://bpmninja:bpmninja@localhost:4222`); unauthentifiziert schlägt gegen diesen Stack fehl.

## Buckets und Streams (nicht ändern)
| Feature | Name | Inhalt |
|---|---|---|
| Object Store | `bpmn_xml` | unveränderliches BPMN-2.0-XML |
| Object Store | `instance_files` | binäre Datei-Variablen |
| KV | `definitions` | `ProcessDefinition` als JSON |
| KV | `instances` | `ProcessInstance` + Tokens + Variablen + Audit-Log |
| KV | `user_tasks` | ausstehende `PendingUserTask` |
| KV | `service_tasks` | `PendingServiceTask` (Camunda-Stil) |
| KV | `timers` | `PendingTimer` (Catch- und Boundary-Timer) |
| KV | `messages` | `PendingMessageCatch` |
| JetStream | Stream `HISTORY` | Subjects `history.instance.*` |

## Prinzipien
- Jeder Schreibvorgang persistiert sofort in KV und publiziert das Event in JetStream.
- Engine-Start stellt den Zustand aus KV und Object Store vollständig wieder her.
- BPMN-XML liegt im Object Store, Metadaten in KV `definitions`.
- In-Memory-Cache nur für Tests (Feature `in-memory`).
- Öffentliche `WorkflowEngine`-API bleibt kompatibel.
- Idiomatisch: `Arc<NatsPersistence>`, async überall, `serde_json`.
- Fehler erweitern `EngineError` um `PersistenceError`.
- `list_kv_entries` reicht `keys()`-Fehler weiter, liefert nie still ein leeres Ergebnis.

## Verboten
- Keine Redis-, PostgreSQL- oder andere Datenbank.
- Kein manuelles Stream-Management ohne `jetstream::new(client)`.
- Kein Blockieren der Tokio-Runtime.
- Keine großen Binaries in KV (→ Object Store).
- Kein Multi-Entity-Write ohne definiertes Fehlerverhalten (Teilfehler sichtbar machen).
