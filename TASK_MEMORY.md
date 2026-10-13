# P10: Feingranulare SSE-Events

Ziel: Instanz-/Task-IDs und kompakte Zustands-Deltas vom Core bis zur Desktop-UI übertragen; unnötige globale Reloads reduzieren und Verlust/Reconnect synchronisieren.

Graph: EngineEvent, server/events.rs, use-engine-events.ts, InstanceDetailDialog.tsx, InstancesPage.tsx (SSE-/Instanz-Communities).

- [ ] Core: Event-Payloads und Emission bei Batch-/Admin-Änderungen.
- [ ] Server: JSON-Daten, Resync bei Broadcast-Verlust.
- [ ] Tauri: vollständige SSE-Blöcke weiterleiten, Reconnect-Resync.
- [ ] React: IDs filtern, Deltas anwenden, globale Reloads bündeln.
- [ ] Core-/SSE-/Frontend-Tests, Builds und Clippy.
- [ ] Backlog/Dokumentation und Graph aktualisieren.

Letzter Stand: SSE sendet leere Daten; Desktop verwirft Payloads. Kompakte Deltas enthalten nur Listenfelder; Variablen/Token-Details werden gezielt nachgeladen.
