---
name: bpmninja-parser
description: Skill for the bpmn-parser crate covering BPMN 2.0 XML parsing, model generation, validation, and error recovery.
version: 2.0.0
tags: [rust, bpmn-parser, xml, quick-xml, parsing, validation]
requires: [cargo]
---

# BPMNinja Parser Skill

## When to Activate
Activate whenever you work on the `bpmn-parser` crate:
- Adding new BPMN element parsing support
- Improving XML parsing performance or error recovery
- Extending the parser model (new fields, attributes)
- Writing fuzzing or property-based tests for the parser
- Fixing spec-compliance issues in parsed output

## Scope & File Map

```
bpmn-parser/src/
├── lib.rs        # Public API re-exports
├── models.rs     # BPMN model structs (ProcessDefinition, elements, flows)
├── parser.rs     # XML → Model parsing logic (quick-xml + serde)
└── tests.rs      # Parser unit & integration tests
```

### Key Types
- `ProcessDefinition` – Top-level parsed BPMN process (aus `engine-core`)
- `SequenceFlow` – Flow connections between elements
- `parse_bpmn_xml(xml: &str) -> EngineResult<ProcessDefinition>` – einziger öffentlicher Einstieg

### Relationship to engine-core
The parser produces `ProcessDefinition` which engine-core's `domain/definition.rs` consumes. The parser's model types MUST be compatible with engine-core's domain types. When adding new elements to the parser, engine-core's domain model usually needs a corresponding update first (see `AGENTS.md`).

## Domain Rules & Patterns

1. **quick-xml + serde**: All XML parsing uses `quick-xml`. Do not introduce alternative XML parsers.
2. **Graceful Error Recovery**: Unknown XML elements should be skipped with a warning, not cause parse failures. This enables forward-compatibility with newer BPMN versions.
3. **Validation**: After parsing, validate structural integrity (e.g., all sequence flow targets exist, start events present).
4. **No External I/O**: The parser is a pure function `&str → Result<ProcessDefinition>`. No file system or network access.
5. **Performance**: Parser must handle BPMN files up to 10MB without excessive memory allocation. Prefer streaming/SAX-style parsing over DOM.

## Verifikation

```bash
cargo clippy -p bpmn-parser --all-targets -- -D warnings
cargo test -p bpmn-parser
```

## Common Pitfalls
- Adding parser support without updating engine-core's domain model first
- Panicking on unexpected XML attributes (use `_` catch-all in match)
- Tight coupling between parser internals and engine-core types
- Tests with inline XML that doesn't match real BPMN modeler output
- Forgetting namespace handling in XML elements

