# Graph-First Development & Navigation (Graphify)

Der Knowledge-Graph liegt unter `graphify-out/graph.json`. Nutze ihn zuerst für
Fragen zu Code, Architektur, Abhängigkeiten und Abläufen – nicht für unabhängige
allgemeine Fragen.

1. Bei Codebase-Fragen zuerst `graphify query "<Frage>"` ausführen. Für konkrete
   Beziehungen `graphify path "<A>" "<B>"`, für ein Symbol/Konzept
   `graphify explain "<Begriff>"` verwenden. Wenn MCP verfügbar ist, sind
   `query_graph`, `get_node`, `get_neighbors` und `shortest_path` gleichwertige
   Alternativen.
2. Graph-Ergebnisse sind der Einstieg, kein Ersatz für Quellcode-Verifikation.
   Danach nur die relevanten Dateien lesen, wenn Implementierung, Debugging oder
   Details es erfordern oder die Graph-Antwort nicht ausreicht.
3. `GRAPH_REPORT.md` nur für einen breiten Architekturüberblick verwenden.
   Community-IDs können sich ändern; keine fest codierten IDs als dauerhaft
   gültige Crate-Zuordnung voraussetzen.
4. Nach Codeänderungen `graphify update .` ausführen; der Git-Hook aktualisiert
   den Graphen zusätzlich nach Commits und Branch-Wechseln. Keinen vollständigen
   semantischen Rebuild für normale Fragen starten.

**WARNUNG:** Vor Änderungen an stark vernetzten Knoten (z. B. `deploy_definition()`)
zuerst Aufrufer und Abhängigkeiten über den Graphen untersuchen.
