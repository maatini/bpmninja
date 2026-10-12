# TASK_MEMORY — Doku-Sync + Performance/Cloud-native-Vorschläge

## Ziel
Doku-Drift gegenüber dem Code beseitigen, erledigte Backlog-Punkte schließen und einen
Aufgaben-Backlog für Performance, Messaging (NATS/Redpanda/Kafka), Dapr und Serverless/Mesh vorschlagen.

## Graph
- God Nodes: `deploy_definition()`, `start_instance()`, `parse_bpmn_xml()`, `NatsPersistence`
- graphify/devbox lokal nicht installiert → Graph nicht neu gebaut (Report vom 2026-10-10)

## Checkliste
- [ ] CLAUDE.md: Multi-Instance-Aussage, Dependency-Versionen, Workspace-Liste
- [ ] KB architecture/dependencies.md: Versionen, Mermaid-Knoten `ET`
- [ ] KB architecture/data-flows.md: doppelte „7“, verwaistes Mermaid-Fragment
- [ ] KB cross-cutting/tags.md: Duplikat, fehlende Tags
- [ ] KB cross-cutting/shared-patterns.md: InstanceStore ist DashMap
- [ ] KB overview.md + index.md-Dateien: Versionen, Testzahlen
- [ ] KB engine-core/gotchas.md: Lock-Beispiel
- [ ] KB engine-server/interfaces.md: offener Codeblock
- [ ] KB persistence/*: echte Bucket-Namen + Streams
- [ ] docs/architecture.md: Modulbaum, InstanceStore, Buckets, Statistik
- [ ] improvements.md: M1/Q4/M4/M5/M6 als erledigt, Phase 4 (Vorschläge) ergänzen
- [ ] implementation_plan.md löschen (erledigt)
- [ ] Abschluss: TASK_MEMORY.md löschen

## Letzter Stand
Analyse abgeschlossen, Edits beginnen.
