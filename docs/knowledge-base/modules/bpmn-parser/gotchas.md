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

Assignee and topic are **not** read from Camunda Modeler attributes:

| BPMN / Camunda attribute | Parser field | Fallback |
|--------------------------|--------------|----------|
| `data-assignee` on `userTask` | `UserTask(assignee)` | `"unassigned"` |
| `data-topic` on `serviceTask` | `ServiceTask.topic` | then `data-handler`, then node id |
| `camunda:assignee` | ignored | `"unassigned"` |
| `camunda:topic` | ignored | node id |

The desktop modeler writes `data-assignee` / `data-topic`. XML exported from Camunda Modeler typically uses `camunda:assignee` / `camunda:topic` and will deploy with those fallbacks.

### ⚠️ Silent task and event mapping

Unknown or empty activity types become `ServiceTask` and enter the fetch-and-lock queue:

| XML | Becomes | Topic / notes |
|-----|---------|----------------|
| `receiveTask`, `manualTask`, `businessRuleTask`, generic `task` | `ServiceTask` | `name` or node id. No DMN for business-rule tasks. |
| `scriptTask` with empty `<script>` and no `data-script` | `ServiceTask` | `name` or node id |
| `intermediateCatchEvent` without timer or message definition | `ServiceTask` | topic `"event_passthrough"` |
| Sub-process internal start events (flattening) | `ServiceTask` | topic `"noop"` |

`complexGateway` is a first-class `ComplexGateway { join_condition, default }` (`activationCondition` → `join_condition`). Compensation and escalation events (`compensateEventDefinition` / `escalationEventDefinition` on throw, end, and boundary) are first-class elements, not pass-through tasks.

### ⚠️ Only one `<process>` per file

`parse_bpmn_xml` keeps a single process: the first with `isExecutable="true"`, otherwise `processes[0]`. Additional `<process>` elements in the same definitions file are dropped.

### ⚠️ InclusiveGateway join is AND-style at runtime

The parser maps `inclusiveGateway` to `InclusiveGateway` with no extra join metadata. Engine-core join uses the same `JoinBarrier` as `ParallelGateway`: when the gateway has two or more incoming flows, it waits until that many tokens have arrived. Split still takes every outgoing flow whose condition is true. After a partial split the join can stall, because tokens on untaken incoming flows never arrive.

### ⚠️ Invalid XML returns EngineError, never panics

All parsing errors produce `EngineError::InvalidDefinition(msg)` with a descriptive message. The parser uses `Result` throughout — no `.unwrap()` calls in the parser path.

### ⚠️ Adding a new BpmnElement variant

When adding to engine-core's `BpmnElement` enum, you must:
1. Add the XML mapping in `bpmn-parser/src/parser.rs` (the new element won't be parseable otherwise)
2. Add a test in `bpmn-parser/src/tests.rs`
3. Update the execution handler in `engine-core/src/engine/handlers/`

### ⚠️ quick-xml version sensitivity

The parser uses `quick_xml::de::from_str` for deserialization. Changes to the XML structure (new attributes, renamed elements) require updating `models.rs` and potentially `parser.rs` dispatch logic.
