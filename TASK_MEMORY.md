# X2c – Explizite Top-Level-Conditional-Starts

Ziel: Den genehmigten Plan in docs/x2c-top-level-conditional-starts-plan.md einschließlich dauerhafter Idempotenz und Wiederanlauf umsetzen.

Graph-Communities: ProcessDefinition, Token, event_subprocess, persistence, X2c; Einstieg über graphify query erfolgt.

- [x] Core: Element, Validierung, Quittungen, reservierter Start, Recovery und Tests
- [x] Parser: ausgewählter Prozess, strikte Ausdrücke und Negativtests
- [ ] Memory/NATS: atomare Quittungen, eigenes Bucket und reale Recovery-Tests
- [x] REST/Startup/TypeSpec/OpenAPI
- [x] Modeler/Browser und Dokumentation
- [ ] Abnahme, fmt, Clippy, Graphify

Letzter Stand: Nutzer hat nach ausdrücklicher X2c-Freigabefrage erneut „continue with tasks“ angewiesen; als Freigabe gewertet. Bestehende X2b-Änderungen bleiben erhalten. Dauerhafte Start-Quittungen sollen die vorhandenen ConditionalWait-Fences und Continuations nutzen. Core, Parser, Memory/NATS, REST/Startup, TypeSpec und Modeler implementiert. Tests für Idempotenz, Recovery-Cutpoints, REST und JetStream ergänzt; Abnahme läuft. Separater NATS-Testprozess PID 26180, Port 4422, Daten unter test-results/nats-x2c-c60d4e04d09f46708527b0f659876045. Fremde/automatische Commits während der Arbeit beobachtet (zuletzt 021b21c), nicht selbst committet.

Abnahme: 402 Core- und 87 Parser-Tests bestanden, zehn X2/X9-Browsertests mit lokalem Chrome bestanden, neun Event-Tests bestanden. TypeSpec und Build/Lint erfolgreich (Lint-Warnungen bestehen). Gesamter Workspace-Test mit REQUIRE_NATS_TESTS=1 läuft, Log test-results/x2c-workspace-tests.log. Reiner Initialstate-Builder wird für Payloadprüfung vor Reservierung wiederverwendet.
