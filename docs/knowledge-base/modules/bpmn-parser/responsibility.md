# bpmn-parser — Responsibilities

## What bpmn-parser Owns

1. **@tag:bpmn-xml-parsing** — Parsing BPMN 2.0 XML strings into typed Rust structs using `quick-xml` + `serde`.
2. **@tag:element-mapping** — Mapping XML elements to `BpmnElement` enum variants (29 variants).
3. **@tag:iso8601-timer-parsing** — Parsing all ISO 8601 timer formats: Duration (`PT30S`), AbsoluteDate, CronCycle (`0 9 * * MON-FRI`), RepeatingInterval (`R3/PT10M`).
4. **@tag:subprocess-flattening** — Flattening embedded sub-processes into the main process graph at parse time.
5. **@tag:listener-extraction** — Extracting Camunda-style execution listeners (`camunda:executionListener`) from extension elements.
6. **@tag:condition-extraction** — Extracting gateway conditions from sequence flow condition expressions.
7. **@tag:namespace-handling** — Handling Camunda namespace prefixes on BPMN extension elements.

## Invariants

1. **Parser never panics on invalid XML** — Returns `EngineResult::Err(EngineError::InvalidDefinition(...))` for all parsing errors.
2. **All 29 BpmnElement variants must be parseable** — Parser must handle all element types defined in engine-core.
3. **Sub-processes are flattened** — Embedded sub-processes are resolved to `EmbeddedSubProcess` + `SubProcessEndEvent` nodes in the main graph.
4. **Timer definitions are validated** — Invalid ISO 8601 strings must produce clear error messages.
5. **Camunda namespace compatibility** — Both `camunda:executionListener` and standard `bpmn:extensionElements` are supported.

## Key Entry Points

| Entry Point | Returns | Description |
|------------|---------|-------------|
| `parse_bpmn_xml(xml: &str) -> EngineResult<ProcessDefinition>` | `ProcessDefinition` | Main entry point: parses BPMN XML into a definition |

## Internal Structure

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 8 | Public exports: `parse_bpmn_xml` |
| `parser.rs` | ~1,800 | Main parsing logic: element dispatch, timer parsing, sub-process flattening, listener extraction |
| `models.rs` | ~150 | Intermediate XML deserialization structs (`BpmnDefinitions`, `BpmnExtensionElements`, etc.) |
| `tests.rs` | ~900 | 32 tests covering all element types, gateways, timers, listeners, namespaces |

## What It Does NOT Own

- **Not:** Domain type definitions — those live in `engine-core::domain`
- **Not:** Execution logic — only parsing
- **Not:** XML validation beyond structural correctness — BPMN compliance checking is engine-core's job
