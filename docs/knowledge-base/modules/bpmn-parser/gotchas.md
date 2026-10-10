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

Assignee and topic are read from desktop `data-*` attributes first, then Camunda Modeler attributes:

| BPMN / Camunda attribute | Parser field | Fallback |
|--------------------------|--------------|----------|
| `data-assignee` on `userTask` | `UserTask(assignee)` | then `camunda:assignee`, then `"unassigned"` |
| `camunda:assignee` | `UserTask(assignee)` | `"unassigned"` if no `data-assignee` |
| `data-topic` on `serviceTask` | `ServiceTask.topic` | then `data-handler`, then `camunda:topic`, then node id |
| `camunda:topic` | `ServiceTask.topic` | node id if no `data-*` |

The desktop modeler writes both `data-assignee`/`data-topic` and `camunda:assignee`/`camunda:topic`. XML exported from Camunda Modeler deploys with the Camunda attributes. `quick-xml` strips the `camunda:` prefix on attributes, so the parser binds `camunda:assignee` as `@assignee` (with alias `@camunda:assignee`).

### ⚠️ Multi-instance is rejected at parse time

`multiInstanceLoopCharacteristics` is **not supported**. The runtime never emits `MultiInstanceFork`; mapping MI to `MultiInstanceDef` used to let diagrams run silently as single-instance.

If the element is present on a task (`userTask`, `serviceTask`, `scriptTask`, `sendTask`, `receiveTask`, `manualTask`, `businessRuleTask`), parse fails with `EngineError::InvalidDefinition`:

`Multi-instance is not supported (element '<id>')`

Deploy must fail. Do not reintroduce a silent single-instance fallback.

### ⚠️ Unsupported tasks are rejected, not queued

Unknown or empty activity types used to become `ServiceTask` and enter the fetch-and-lock queue. They now fail parse with `InvalidDefinition`:

| XML | Becomes |
|-----|---------|
| `receiveTask` with `messageRef` | `MessageCatchEvent` |
| `receiveTask` without `messageRef` | error |
| `manualTask` | `UserTask` (assignee from `data-assignee` / `camunda:assignee`) |
| `businessRuleTask` with topic | `ServiceTask` (DMN is not supported) |
| `businessRuleTask` without topic | error |
| generic `task` | error — convert to a concrete type |
| `scriptTask` with empty `<script>` and no `data-script` | error |
| `intermediateCatchEvent` without timer or message | error |
| Sub-process internal start events (flattening) | `ServiceTask` topic `"noop"` |

`sendTask` reads nested `messageEventDefinition/@messageRef` and the `sendTask/@messageRef` attribute. Empty `<timerEventDefinition/>` is rejected (no longer `Duration(0)`). Timer fields accept `xsi:type` FormalExpression wrappers.

Embedded sub-process flattening includes `sendTask`, `inclusiveGateway`, `eventBasedGateway`, and intermediate catch/throw events. Missing those used to yield `NoSuchNode` at runtime.

`complexGateway` is a first-class `ComplexGateway { join_condition, default }` (`activationCondition` → `join_condition`). Compensation and escalation events (`compensateEventDefinition` / `escalationEventDefinition` on throw, end, and boundary) are first-class elements, not pass-through tasks.

### ⚠️ Only one `<process>` per file

`parse_bpmn_xml` keeps a single process: the first with `isExecutable="true"`, otherwise `processes[0]`. Additional `<process>` elements in the same definitions file are dropped.

### ⚠️ InclusiveGateway has no join metadata in the model

The parser maps `inclusiveGateway` to `InclusiveGateway` with no extra join metadata. Engine-core still infers the join partner at runtime (`find_downstream_join`) and sets `JoinBarrier.expected_count` to the number of paths actually taken at the matching split (including 1-of-N). Unstructured diagrams without a same-type downstream join still wait on structural `incoming_flow_count` and can stall.

### ⚠️ Invalid XML returns EngineError, never panics

All parsing errors produce `EngineError::InvalidDefinition(msg)` with a descriptive message. The parser uses `Result` throughout — no `.unwrap()` calls in the parser path.

### ⚠️ Adding a new BpmnElement variant

When adding to engine-core's `BpmnElement` enum, you must:
1. Add the XML mapping in `bpmn-parser/src/parser.rs` (the new element won't be parseable otherwise)
2. Add a test in `bpmn-parser/src/tests.rs`
3. Update the execution handler in `engine-core/src/engine/handlers/`

### ⚠️ quick-xml version sensitivity

The parser uses `quick_xml::de::from_str` for deserialization. Changes to the XML structure (new attributes, renamed elements) require updating `models.rs` and potentially `parser.rs` dispatch logic.
