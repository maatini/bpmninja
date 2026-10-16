# X2c – Explizite Top-Level-Conditional-Starts

Ziel: Den genehmigten Plan in docs/x2c-top-level-conditional-starts-plan.md einschließlich dauerhafter Idempotenz und Wiederanlauf umsetzen.

Graph-Communities: ProcessDefinition, Token, event_subprocess, persistence, X2c; Einstieg über graphify query erfolgt.

- [ ] Core: Element, Validierung, Quittungen, reservierter Start, Recovery und Tests
- [ ] Parser: ausgewählter Prozess, strikte Ausdrücke und Negativtests
- [ ] Memory/NATS: atomare Quittungen, eigenes Bucket und reale Recovery-Tests
- [ ] REST/Startup/TypeSpec/OpenAPI
- [ ] Modeler/Browser und Dokumentation
- [ ] Abnahme, fmt, Clippy, Graphify

Letzter Stand: Nutzer hat nach ausdrücklicher X2c-Freigabefrage erneut „continue with tasks“ angewiesen; als Freigabe gewertet. Bestehende X2b-Änderungen bleiben erhalten. Dauerhafte Start-Quittungen sollen die vorhandenen ConditionalWait-Fences und Continuations nutzen. Noch keine Feature-Implementierung.
