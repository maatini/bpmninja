# bpmn-parser — Gotchas

### ⚠️ Sub-process flattening changes graph structure

Embedded sub-processes are not preserved at runtime. The parser inlines them:
- Sub-process start/end events become `EmbeddedSubProcess` + `SubProcessEndEvent` nodes in the parent graph
- Sub-process boundaries are lost — this means boundary events on sub-processes won't work as expected in BPMN 2.0 spec

If you need runtime sub-process scoping, you're looking for Call Activities (`callActivity` → `CallActivity`), not embedded sub-processes.

### ⚠️ ISO 8601 duration parsing is strict

The parser handles these formats:
- `PT30S`, `PT5M`, `PT1H30M`, `P1D`, `P1DT12H`, `P1W`, `P1M`, `P1Y6M`, `P1DT2H30M15S`
- Absolute date: `2026-04-06T14:30:00Z`
- Cron: `0 9 * * MON-FRI`
- Repeating: `R3/PT10M`
- Compact repeating: `R3/PT10M` (same format)

Months/years are approximated to 30/365 days. This is NOT ISO 8601 compliant for calendar-sensitive durations.

### ⚠️ Camunda namespace handling

The parser supports `camunda:executionListener` (with `camunda:` namespace prefix). If standard `bpmn:extensionElements` are used, the parser also extracts them. Both are mapped to `ExecutionListener` structs.

### ⚠️ Invalid XML returns EngineError, never panics

All parsing errors produce `EngineError::InvalidDefinition(msg)` with a descriptive message. The parser uses `Result` throughout — no `.unwrap()` calls in the parser path.

### ⚠️ Adding a new BpmnElement variant

When adding to engine-core's `BpmnElement` enum, you must:
1. Add the XML mapping in `bpmn-parser/src/parser.rs` (the new element won't be parseable otherwise)
2. Add a test in `bpmn-parser/src/tests.rs`
3. Update the execution handler in `engine-core/src/engine/handlers/`

### ⚠️ quick-xml version sensitivity

The parser uses `quick_xml::de::from_str` for deserialization. Changes to the XML structure (new attributes, renamed elements) require updating `models.rs` and potentially `parser.rs` dispatch logic.
