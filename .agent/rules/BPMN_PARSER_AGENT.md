---
trigger: file_match
file_patterns: ["bpmn-parser/**"]
---

# Parser Agent
- **Domain:** `bpmn-parser/`
- **Role:** Takes raw BPMN 2.0 XML string/bytes and returns `engine_core::ProcessDefinition`.
- **Parser:** `quick-xml` with `serde` deserialization.

## Parsing Rules
- Validate BPMN structural integrity before returning `Ok()` (e.g., every flow target must exist, start events must be present).
- Use `ProcessDefinitionBuilder` from `engine-core` to construct the output — never build `ProcessDefinition` directly.
- Parse all 21 `BpmnElement` variants documented in `BPMN_WORKFLOW_ENGINE.md`.
- Gracefully handle unknown elements: log via `tracing::warn!` and skip — do not error on vendor extensions.

## Error Handling
- Return `anyhow::Result<ProcessDefinition>` with descriptive error context.
- Include XML element name and line context in error messages where possible.
- Never panic on malformed XML — always return an error.

## Testing
- Test each BPMN element type with minimal XML snippets.
- Test error cases: malformed XML, missing required attributes, unknown elements.
- Round-trip tests: parse → serialize → parse should produce identical `ProcessDefinition`.
