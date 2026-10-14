# BPMN 2.0.2 Element Matrix

Status of every BPMN 2.0.2 element in BPMNinja (v0.7.24, updated 2026-10-14). Derived from `bpmn-parser/src/parser.rs` (mapping + `reject_unsupported_elements`) and `engine-core`.

| Symbol | Meaning |
|---|---|
| ✅ | Parsed **and executed** |
| 🟡 | Accepted, **no execution semantics** (visual / documentation / data element — the flow is unaffected) |
| ❌ | **Rejected at deploy** with `InvalidDefinition` naming the element — backlog item in brackets |
| ⚠️ | **Partial** — implemented subset or non-standard modelling form (see notes) |

Only the deployed process is checked (first `isExecutable="true"` process, else the first process). Elements of other pools are ignored.

## Flow Objects — Events

| Element | Start | Intermediate catch | Intermediate throw | Boundary | End |
|---|---|---|---|---|---|
| None | ✅ | — | ❌ (milestone not executed) | — | ✅ |
| Message | ✅ | ✅ | ✅ (as send) | ✅ interrupting / non-interrupting | ❌ [X1/X12] |
| Timer (duration, date, cycle) | ✅ | ✅ | — | ✅ interrupting / non-interrupting | — |
| Error | ❌ in event sub-process [X9] | — | — | ✅ | ✅ |
| Escalation | ❌ in event sub-process [X9] | — | ✅ | ✅ | ✅ |
| Compensation | ❌ in event sub-process [X9] | — | ✅ | ✅ | ✅ |
| Conditional | ❌ [X2] | ❌ [X2] | — | ❌ [X2] | — |
| Signal | ✅ (latest version; not in sub-processes) | ✅ | ✅ (broadcast after the step) | ✅ interrupting / non-interrupting | ✅ (not in sub-processes) |
| Link | — | ✅ (resolved at parse time) | ✅ (resolved at parse time) | — | — |
| Cancel | — | — | — | ❌ [X7] | ❌ [X7] |
| Terminate | — | — | — | — | ✅ |
| Multiple / Parallel multiple | ❌ [X10] | ❌ [X10] | ❌ [X10] | ❌ [X10] | ❌ [X10] |

## Flow Objects — Activities

| Element | Status | Notes |
|---|---|---|
| User Task | ✅ | Assignee from `data-assignee` / `camunda:assignee` |
| Service Task | ✅ | External task (fetch-and-lock); topic from `data-topic` / `camunda:topic` |
| Script Task | ✅ | Rhai |
| Send Task | ✅ | Throws a message, continues |
| Receive Task | ✅ | As message catch; `instantiate="true"` ✅ as message start (no incoming flows, top-level process only) |
| Manual Task | ✅ | As user task |
| Business Rule Task | ✅ / ❌ | With topic → external task; without topic ❌ (no DMN) [X14] |
| Task (abstract) | ❌ | Use a concrete task type |
| Call Activity | ✅ | Variables propagated both ways, error boundaries |
| Embedded Sub-Process | ✅ | Flattened at parse time (ADR-003); entered at the internal start event's successor. Boundary events on the sub-process are not spec-exact [X6] |
| Event Sub-Process | ❌ [X9] | Was parsed but never triggered |
| Transaction | ❌ [X7] | |
| Ad-hoc Sub-Process | ❌ [X8] | |
| Loop (`standardLoopCharacteristics`) | ✅ | Tasks + call activities; `loopCondition`, `testBefore`, `loopMaximum`. On sub-processes ❌ [X6] |
| Multi-instance (parallel / sequential) | ✅ | Tasks + call activities; cardinality or `camunda:collection`/`elementVariable`, `completionCondition`, `outputCollection`/`outputElement`; loop variables local. On sub-processes ❌ [X6]; non-interrupting boundary events on loop activities ❌ |
| Compensation activity (`isForCompensation`) | ⚠️ [G1] | The engine finds the handler via a **sequence flow** leaving the compensation boundary event. The standard BPMN form (an **association**, as drawn by bpmn-js) is not parsed yet, so such handlers are never called |

## Flow Objects — Gateways

| Element | Status | Notes |
|---|---|---|
| Exclusive (XOR) | ✅ | Conditions + default flow |
| Parallel (AND) | ✅ | Fork / join |
| Inclusive (OR) | ✅ | Join counts taken branches of the matching split |
| Complex | ✅ | `activationCondition` |
| Event-based (exclusive) | ✅ | |
| Event-based instantiating / parallel | ❌ [X13] | |

## Connecting Objects

| Element | Status | Notes |
|---|---|---|
| Sequence flow | ✅ | Conditional and default flows |
| Message flow | 🟡 | Between pools; only one process is deployed [X12] |
| Association | 🟡 | Compensation associations not yet interpreted [G1] |
| Data association (input / output) | ⚠️ | One source/target; process data ↔ task IO/property; dangling references, assignments, transformations rejected. [Contract](../../../data-mappings.md) |

## Swimlanes

| Element | Status | Notes |
|---|---|---|
| Pool (participant) | 🟡 | One executable process per deployment [X12] |
| Lane / lane set | 🟡 | Kept on export (original XML is stored verbatim). Lane → candidate group needs candidate groups on user tasks [X16] |

## Artifacts

| Element | Status |
|---|---|
| Group | 🟡 |
| Text annotation | 🟡 |

## Data

| Element | Status | Notes |
|---|---|---|
| Data object / data object reference | ✅ | Process variable declarations; explicit JSON defaults; references alias the object variable [X11] |
| Data object collection | ⚠️ | JSON arrays supported as values; no XML Schema collection typing |
| Data store reference | 🟡 | |
| Data input / data output / `ioSpecification` | ⚠️ | Scalar task IO and validated input/output sets; process/subprocess IO and collection IO rejected [X11] |
| Property | ⚠️ | Process declarations/defaults and task association endpoints; subprocess declarations rejected pending X6 [X11] |
| Camunda `inputOutput` / Call Activity `in` / `out` | ⚠️ | Task-local input and success output mappings; literals, variable references, nested lists/maps; explicit child transfer; scripts/compound expressions rejected. [Contract](../../../data-mappings.md) |
| Message, Error, Escalation, Signal definitions (root elements) | ✅ | Referenced by event definitions |

## Maintenance

- Adding support for an element: implement it, remove it from `reject_unsupported_elements` (or `supported_event_definitions`) in `bpmn-parser/src/parser.rs`, and update this matrix + the README.
- Backlog items `[Xn]` are described in `improvements.md`, section 4f.
