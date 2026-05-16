# Global Agent Directives & Workflow

## Priorität bei Regelkonflikten

1. System-/Sicherheitsvorgaben
2. Workspace-Regeln (`CLAUDE.md`, diese Datei)
3. User-Anweisungen im aktuellen Chat
4. Dateispezifische Agent-Regeln (`file_match`)

Bei Konflikten gilt immer die höher priorisierte Regel.

## Workflow

1. **Plan nur bei Bedarf:** Plan verpflichtend bei unklaren Anforderungen, Architekturentscheidungen oder Multi-Crate-Änderungen. Bei klaren, kleinen Tasks direkte Umsetzung.
2. **GO-Gate nur für Plan-Tasks:** Wenn ein Plan erstellt wurde, vor Implementierung auf `GO` warten.
2. **Sprache:** Kommunikation, Commits und Dokumentation erfolgen auf Deutsch. API-Bezeichner und Code bleiben auf Englisch.
3. **No Temp Files:** Niemals `tmp/`, `temp/` oder den Desktop nutzen. Code muss in gut benannten Ziel-Modulen oder In-Memory getestet/geschrieben werden.
4. **Architektur & Handoff-Order:** Dependencies zwingend immer zuerst bauen!
   Die Implementierungs-Reihenfolge bei Cross-Crate-Features ist strikt:
   `engine-core` (Traits/pure) → `bpmn-parser` → `persistence-nats` → `engine-server` (Axum) → `desktop-tauri` (Rust/React).
5. **Traits over Types:** Cross-Crate-Kommunikation erfolgt ausschließlich über Rust Traits (z. B. im `port/` Modul). Niemals konkrete Typen aus anderen Crates importieren.
