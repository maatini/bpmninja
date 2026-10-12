# bpmn-parser

Parses BPMN 2.0 XML into `ProcessDefinition` structs used by engine-core. Handles element mapping, ISO 8601 timer parsing, listener extraction, and sub-process flattening.

**Crate path:** `bpmn-parser/`  
**Source:** `bpmn-parser/src/` (4 files: lib.rs, parser.rs, models.rs, tests.rs, ~3,000 LoC incl. tests)  
**Tests:** ~46 tests in `tests.rs`

- [responsibility.md](responsibility.md)
- [dependencies.md](dependencies.md)
- [interfaces.md](interfaces.md)
- [gotchas.md](gotchas.md)
