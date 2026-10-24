---
trigger: file_match
file_patterns: ["bpmn-parser/**"]
---

# Parser-Regeln

- **Domain:** `bpmn-parser/`
- **Rolle:** BPMN-2.0-XML → `engine_core::ProcessDefinition`.
- **Technik:** `quick-xml` mit `serde`-Deserialisierung; Einstieg `parse_bpmn_xml(xml: &str) -> EngineResult<ProcessDefinition>`.

## Parsing
- Struktur vor `Ok()` validieren: jede Flow-Ziel-ID existiert, Start-Event vorhanden, Pflichtattribute gesetzt.
- Ausgabe ausschließlich über `ProcessDefinitionBuilder` bauen.
- Alle 29 `BpmnElement`-Varianten aus `RUST_ENGINE_AGENT.md` abdecken.
- Unbekannte Elemente mit `tracing::warn!` überspringen; Vendor-Extensions sind kein Fehler.
- Embedded Sub-Prozesse beim Parsen flatten; Send-, Inclusive-, EventBased- und Intermediate-Events dabei erhalten.
- Multi-Instance wird abgelehnt (`InvalidDefinition`), nicht still ignoriert.

## Attribute und Namespaces
- `quick-xml` entfernt das `camunda:`-Präfix; Attribute kommen als `@assignee`/`@topic` an.
  `serde` nutzt `rename = "@topic"` plus `alias = "@camunda:topic"` (analog `assignee`).
- Topic-Priorität: `data-topic` → `data-handler` → `camunda:topic` → Node-ID.
- Assignee-Priorität: `data-assignee` → `camunda:assignee` → unassigned.

## Fehler
- Fehler als `EngineError::InvalidDefinition` mit Elementname und Kontext zurückgeben.
- Bei kaputtem XML nie panicken.
- `receiveTask` mit `messageRef` wird MessageCatchEvent, `manualTask` wird UserTask,
  `businessRuleTask` nur mit explizitem Topic gültig; leere Timer-/Event-Definitionen sind ungültig.

## Tests
- Jede Elementart mit minimalem XML-Snippet testen.
- Fehlerfälle: kaputtes XML, fehlende Pflichtattribute, unbekannte Elemente.
- Roundtrip: parse → serialize → parse ergibt dieselbe `ProcessDefinition`.
