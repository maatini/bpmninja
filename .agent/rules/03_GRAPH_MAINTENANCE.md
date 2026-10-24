# Graph Maintenance & Metriken

Der Knowledge-Graph kann nach Code-Änderungen veralten. Halte ihn mit inkrementellen Updates aktuell; der installierte Git-Hook aktualisiert ihn zusätzlich nach Commits und Branch-Wechseln.

### Wann `graphify update .` ausgeführt werden MUSS:
- Nach jedem abgeschlossenen Task-Intervall (≥3 betroffene Dateien).
- Falls der Git-Hook nicht verfügbar war oder fehlschlug.
- Vor einem `/compact` Kontext-Reset.
- Bei der Anlage von neuen Funktionen/Architekturelementen.
*(Dies führt nur ein AST-Update durch und generiert keine API-Kosten.)*

### Nach jedem Update – Metriken in `GRAPH_REPORT.md` prüfen:
Überprüfe zwingend die Auswirkungen deiner Arbeit:
1. **God Nodes (>50 Edges):** Sind neue God Nodes entstanden? -> Erstelle ein "Refactor"-Issue.
2. **Isolierte Nodes (≤1 Edge):** Hast du toten Code geschrieben? -> Fehlende Integration prüfen.
3. **Community-Kohäsion (< 0.05):** Hat sich die Modulgrenze verschlechtert? -> Modulschnitt überdenken.

### Manueller Full Re-Build
Führe `graphify .` (voller Rebuild inklusive Semantik/Inferenz) **niemals** eigenmächtig aus, da dies API-Kosten verursachen kann. Nur nach explizitem Nutzerbefehl auslösen. Für normale Codefragen immer eine gezielte `graphify query`-Abfrage statt eines Rebuilds verwenden.
