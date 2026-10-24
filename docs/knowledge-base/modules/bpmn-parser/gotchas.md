# bpmn-parser — Gotchas

### ⚠️ Sub-process flattening changes graph structure

Embedded sub-processes are not preserved at runtime. The parser inlines them:
- Sub-process end events become `SubProcessEndEvent` nodes in the parent graph; the internal start event is not materialised — the `EmbeddedSubProcess` node continues at the start event's target
- Sub-process boundaries are lost — this means boundary events on sub-processes won't work as expected in BPMN 2.0 spec

If you need runtime sub-process scoping, you're looking for Call Activities (`callActivity` → `CallActivity`), not embedded sub-processes.

### ⚠️ Boundary-Events auf Embedded Sub-Processes werden nicht registriert

Boundary-Events, die an einem `subProcess` hängen (Timer, Message, Error, Escalation, Compensation), werden beim Betreten des Sub-Prozesses nicht als Wait-State registriert. `setup_boundary_events` läuft nur für User-Tasks, Service-Tasks und Call-Activities; ein Timer oder eine Message kann den eingebetteten Scope daher nicht unterbrechen. Das ist eine bekannte Lücke (Scope-Cancel und Token-Invalidierung fehlen) und nicht Teil des Default-Flow-Fixes — dafür ist ein eigener Folgeplan nötig.

Default-Flows an `exclusiveGateway` und `complexGateway` werden auch innerhalb eines Sub-Prozesses von der Flow-ID auf den Zielknoten aufgelöst. Eine `default`-Flow-ID, die im Sub-Prozess nicht existiert, wird mit `EngineError::InvalidDefinition` abgelehnt (Meldung enthält Sub-Prozess- und Gateway-ID); ein stiller Fallback auf `None` findet nicht statt.

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

### ⚠️ Multi-instance: parallel and sequential service tasks only

`multiInstanceLoopCharacteristics` is supported **only** on `serviceTask`, in both **parallel** and **sequential** (`isSequential="true"`) form, with either `collection` + `elementVariable` or `loopCardinality` (positive integer). The runtime tracks `MultiInstanceProgress` and advances the token once after the last item completes. Parallel MI creates all pending tasks up front; sequential MI keeps exactly one task open, stores the remaining items in `MultiInstanceProgress.items` and reuses the token for the next iteration. Collection order and the `elementVariable` value per iteration are preserved; `loopCounter` starts at 1.

Everything else is rejected at parse time with `EngineError::InvalidDefinition`:

- `multiInstanceLoopCharacteristics` on `userTask`, `scriptTask`, `sendTask`, `receiveTask`, `manualTask`, or `businessRuleTask`
  (`Multi-instance is only supported for service tasks; <kind> '<id>' is not supported`)
- missing both `collection` and `loopCardinality`, or defining both
- non-positive/non-integer `loopCardinality`

Deploy must fail. Do not reintroduce a silent single-instance fallback.

### ⚠️ Link events are scope-local and drop the catch node

`intermediateThrowEvent`/`intermediateCatchEvent` with `<linkEventDefinition name="…"/>` are resolved per scope (process or each sub-process):

- The parser builds a map `link name → target of the catch's single outgoing flow`. The catch node is **not** materialised and its outgoing flow is dropped; the throw becomes `LinkThrowEvent { target }` with a synthetic edge to that target (the engine needs the edge for graph validation, the handler jumps directly).
- Exactly one outgoing flow per link catch; zero or several are rejected. A catch must not have incoming flows.
- Duplicate link names in the same scope, throws without a matching catch, and throws with outgoing flows are rejected with `InvalidDefinition`.
- Link pairing never crosses a scope boundary: a throw inside a sub-process cannot reach a catch in the parent (or vice versa). The error names both the link name and the scope.

Nested sub-processes each get their own map, so equal link names in sibling scopes do not collide.

### ⚠️ Cancel events and transactions are rejected, not ignored

`cancelEventDefinition` is modelled on `endEvent` and `boundaryEvent` only to fail deployment:

| XML | Result |
|-----|--------|
| `endEvent` with `cancelEventDefinition` | `InvalidDefinition`: cancel end events require a transaction sub-process |
| `boundaryEvent` with `cancelEventDefinition` | `InvalidDefinition`: same reason, with event ID |
| `<transaction>` in a process or sub-process | `InvalidDefinition`: transaction sub-processes are not supported |

Without these checks a cancel end event silently degraded to a plain `EndEvent`, which is semantically wrong: cancel only interrupts a transaction scope. Full transaction/cancel semantics (scope rollback, compensation interaction) are a separate plan — do not re-add silent fallbacks.

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
| Sub-process internal start events (flattening) | pass-through: `EmbeddedSubProcess.start_node_id` points at the start event's target; several outgoing flows become a `ParallelGateway` (implicit split) |

`sendTask` reads nested `messageEventDefinition/@messageRef` and the `sendTask/@messageRef` attribute. Empty `<timerEventDefinition/>` is rejected (no longer `Duration(0)`). Timer fields accept `xsi:type` FormalExpression wrappers.

Embedded sub-process flattening includes `sendTask`, `inclusiveGateway`, `eventBasedGateway`, and intermediate catch/throw events. Missing those used to yield `NoSuchNode` at runtime.

An embedded sub-process must contain exactly one start event; zero or several are rejected at parse time (`has no startEvent` / `must have exactly one startEvent`).

`complexGateway` is a first-class `ComplexGateway { join_condition, default }` (`activationCondition` → `join_condition`). Compensation and escalation events (`compensateEventDefinition` / `escalationEventDefinition` on throw, end, and boundary) are first-class elements, not pass-through tasks.

### ⚠️ Only one `<process>` per file

`parse_bpmn_xml` keeps a single process: the first with `isExecutable="true"`, otherwise `processes[0]`. Additional `<process>` elements in the same definitions file are dropped.

### ⚠️ InclusiveGateway has no join metadata in the model

The parser maps `inclusiveGateway` to `InclusiveGateway` with no extra join metadata. Engine-core still infers the join partner at runtime (`find_downstream_join`) and sets `JoinBarrier.expected_count` to the number of paths actually taken at the matching split (including 1-of-N). Unstructured diagrams without a same-type downstream join still wait on structural `incoming_flow_count` and can stall.

### ⚠️ Bedingungen werden beim Deploy validiert

`conditionExpression` an Sequenzflüssen und `activationCondition` an `complexGateway` werden beim Bau der `ProcessDefinition` über `engine_core::validate_condition` geprüft. Ein Syntaxfehler bricht den Deploy mit `EngineError::InvalidDefinition` ab; die Meldung nennt Quelle und Ziel des Flusses bzw. die Gateway-ID und die 1-basierte Zeichenposition.

Akzeptiert werden `&&`/`and`, `||`/`or`, `!`/`not`, `==`, `!=`, `>`, `<`, `>=`, `<=`, Arithmetik `+ - * / %`, Klammern, Zahlen, Strings mit einfachen oder doppelten Anführungszeichen (inkl. Escapes), `true`/`false`/`null` sowie Pfade mit Punkten und Array-Indizes (`order.amount`, `items[0].id`). Eine `${…}`-/`#{…}`-Umhüllung ist nur vollständig um den gesamten Ausdruck erlaubt.

Das ist eine bewusste Breaking Change: zuvor still akzeptierte, kaputte Ausdrücke (z. B. `${a &&}`) liefen in den Default-Pfad und werden jetzt abgelehnt. `=` als Vergleichsoperator, Funktionsaufrufe und Rhai-Syntax sind ungültig.

### ⚠️ Invalid XML returns EngineError, never panics

All parsing errors produce `EngineError::InvalidDefinition(msg)` with a descriptive message. The parser uses `Result` throughout — no `.unwrap()` calls in the parser path.

### ⚠️ Adding a new BpmnElement variant

When adding to engine-core's `BpmnElement` enum, you must:
1. Add the XML mapping in `bpmn-parser/src/parser.rs` (the new element won't be parseable otherwise)
2. Add a test in `bpmn-parser/src/tests.rs`
3. Update the execution handler in `engine-core/src/engine/handlers/`

### ⚠️ quick-xml version sensitivity

The parser uses `quick_xml::de::from_str` for deserialization. Changes to the XML structure (new attributes, renamed elements) require updating `models.rs` and potentially `parser.rs` dispatch logic.
