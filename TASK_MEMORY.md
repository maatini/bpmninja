# P10: Feingranulare SSE-Events

Ziel: Instanz-/Task-IDs und kompakte Zustands-Deltas vom Core bis zur Desktop-UI übertragen; unnötige globale Reloads reduzieren und Verlust/Reconnect synchronisieren.

Graph: EngineEvent, server/events.rs, use-engine-events.ts, InstanceDetailDialog.tsx, InstancesPage.tsx (SSE-/Instanz-Communities).

- [x] Core: Event-Payloads und Emission bei Batch-/Admin-Änderungen.
- [x] Server: JSON-Daten, Resync bei Broadcast-Verlust.
- [x] Tauri: vollständige SSE-Blöcke weiterleiten, Reconnect-Resync.
- [x] React: IDs filtern, Deltas anwenden, globale Reloads bündeln.
- [ ] Core-/SSE-/Frontend-Tests, Builds und Clippy.
- [ ] Backlog/Dokumentation und Graph aktualisieren.

Letzter Stand: Implementierung und SSE-Dokumentation vorhanden. Acht Frontend-Regressionstests und zwei Tauri-Parser-Tests bestanden. Core-Event-Test wartet auf Cargo-Lock, Server-Tests werden gebaut. npm ci läuft mit langsamen, erfolgreichen Downloads; Frontend-Build und Clippy folgen. Build-generierte Tauri-Schemaänderungen zurückgenommen.
