# X9 — Event subprocesses

Status: X9a and X9b implemented and accepted, 2026-10-15. Full X9 remains partial: nested/compensation scopes are the X9c follow-up.
Next task after X11 in `improvements.md`.

## Starting point before X9a

- `bpmn-parser/src/parser.rs::reject_unsupported_elements` rejects every
  `subProcess` with `triggeredByEvent="true"`. XML event subprocesses are not
  currently supported, including timer/message/error variants.
- `ScopeEventListener` has timer, message, and error variants for programmatic
  definitions. `process_start.rs` ignores timer/message interruption flags and
  does not register error listeners.
- Timer/message scope triggers use a nil token ID and loosely spawn a child
  Call Activity. That mechanism does not establish correct event-handler
  completion, cancellation, repeated subscriptions, or definition-version binding.
- X11 provides task activation mappings, not subprocess runtime scopes.

## Proposed first implementation: X9a

Implement event subprocesses directly under a process for timer, message, signal,
error, and escalation starts. Support legal interrupting/non-interrupting forms;
reject illegal event/flag combinations at deployment. Conditional starts and
compensation starts remain explicit deployment errors during this increment.

Example: a process waits on an external task while a non-interrupting message
handler creates a user task. The original task remains pending. Completing the
original task does not complete the process while the handler is active. A second
message can start another handler activation while the containing scope is live.
With an interrupting start, the original task is cancelled and only the handler
continues; its successful completion ends the interrupted process scope.

### Definition and parser

1. Add event-subprocess definitions carrying stable subprocess/start IDs, event
   kind/reference, interruption mode, and handler body. Separate handlers from
   ordinary process starts, flows, and Call Activity child definitions.
2. Parse handler bodies with the existing supported-task/gateway parser machinery;
   resolve references and execution-listener IDs consistently. Require one start
   event and reject incoming/outgoing sequence flows across the handler boundary.
3. Persist handler bodies with their owning definition. Running instances use
   their original deployed version; never resolve a handler by latest process ID.
4. Initially reject event subprocesses inside embedded subprocesses or other
   handlers, handler-level data declarations/mappings, transaction scopes, and
   compensation/conditional starts. Retain X11 mappings on tasks inside handlers.
5. Adapt model checking to include handler bodies without treating their starts
   as additional ordinary process starts. Unsupported checker semantics must be
   reported explicitly instead of producing a false validation result.

### Runtime lifecycle

1. Add persisted handler activation identity to tokens and distinguish handler
   execution from main-flow execution within the same `ProcessInstance`.
   Do not resume handlers through `outstanding_calls` as if they were Call Activities.
2. Add subscriptions bound to instance, definition, handler, and containing scope.
   Use an explicit subscription identity rather than making nil token IDs the new
   execution contract. Persist subscriptions before exposing pending work.
3. Register subscriptions on all process-start paths. Keep non-interrupting
   message/signal subscriptions available for repeated activation; reschedule
   recurring timers according to their existing timer definition.
4. Atomically claim each interrupting subscription and invalidate the scope's
   main tokens and other handler activations. Cancel their tasks, timers, catches,
   joins, loop progress, and child Call Activities. Late worker completion or
   late child completion must not revive cancelled execution. Reuse cancellation
   machinery and close any gaps before enabling interrupting handlers.
5. Keep non-interrupting handlers alongside existing tokens. Complete the process
   only when main execution, active handlers, and tracked calls have finished;
   subscriptions alone must not keep an otherwise completed process alive.
6. Remove subscriptions on completion, deletion, termination, and interruption.
   Respect suspension for message/signal delivery as well as timers.
7. Route BPMN errors/escalations from tasks, throw/end events, and child calls to
   eligible handlers using one deterministic selection rule. Preserve existing
   boundary handling; specify nearest-scope/handler precedence and reject
   ambiguous matching handlers where selection cannot be deterministic.
8. Execute with the process variable context. Maintain task-local X11 isolation
   and define conflict behavior for concurrent handler writes. Handler-level
   local variables and input/output mappings require X6 and are rejected for now.
9. Record activation/interruption/completion in history and emit existing scoped
   SSE changes. Expose handler waiting tasks under the existing instance ID.

### Recovery and compatibility

- New definition/instance/token fields have serde defaults for old snapshots.
- Restore subscriptions and handler activations without reapplying task inputs,
  duplicating handlers, or selecting a newer handler body.
- Handle interrupted snapshots consistently: cancelled pending records cannot
  become fetchable again. Test crash points around subscription claim, activation
  registration, and cancellation; do not claim distributed exactly-once delivery.
- Preserve compatibility for existing programmatic `ScopeEventListener`
  definitions through a documented adapter or explicit validation/migration
  error. Do not reinterpret their target-definition strings silently.

### Modeler, contract, and acceptance

- Use bpmn-js event subprocess/start-event modelling and expose supported event
  references and interruption flags. Clearly report unsupported deployments.
- Update the element matrix, README, API/worker contract where affected, and
  `improvements.md`. X9 stays partial after X9a.
- Parser tests: all five start kinds, invalid structures/flags, nested rejection,
  reference resolution, task mappings, and XML export preservation.
- Engine tests: repeated non-interrupting activation, main/handler completion
  ordering, interrupting cancellation including parallel loops and calls,
  error/escalation precedence, suspension, termination, and late completions.
- Persistence tests: old snapshots, definition/activation/subscription roundtrip,
  restart with main and handler tasks pending, interruption recovery, and version
  binding. Exercise real NATS reconnect as well as in-memory serialized recovery.
- REST tests: deployed XML execution through message/signal/task APIs and restart.
- Browser tests: edit supported start attributes and retain them through export.
- Run affected Rust tests, required workspace checks, frontend build/lint/tests,
  then `graphify update .`. Run Cargo builds/checks serially to avoid replacing
  dependency artifacts during doctests. Keep temporary stores/configs ignored.

## Later increments and dependencies

- **X9b — Conditional starts:** introduce a shared variable-change evaluation
  hook for native/Camunda task completion, direct updates, scripts/listeners,
  messages/signals, Call Activity results, and handler writes. Persist evaluation
  state and specify retrigger behavior to prevent infinite activation while a
  condition stays true. Reuse this mechanism for X2; do not mark other conditional
  event forms implemented as a side effect. [Separate plan and completed acceptance](x2-conditional-events-plan.md), implemented after user GO.
- **X9c — Nested scopes and compensation starts:** requires the X6 runtime-scope
  design and compensation lifecycle/snapshots, coordinated with G1. Validate
  compensation-start semantics against BPMN 2.0.2 before implementing them.
  Compensation must never be treated as an ordinary asynchronous subscription.
  Needs its own plan and GO after those prerequisites are concrete.

## Semantic references

Normative target: [OMG BPMN 2.0.2](https://www.omg.org/spec/BPMN/2.0.2).
For the first increment, [Camunda event-subprocess lifecycle documentation](https://docs.camunda.io/docs/components/modeler/bpmn/event-subprocesses/)
also describes repeated non-interrupting activation, cancellation of other active
handlers on interruption, and completion waiting for triggered handlers. This is
an implementation reference; it does not establish full BPMN or Camunda conformance.

X9a is authorized. X9b/X9c are follow-ups; full X9 remains incomplete.

## Implementation checkpoint (2026-10-15)

- Implemented process-level timer/message/signal/error/escalation definitions and XML parsing, interruption flags, task mappings, repeated activation, shared process writes, completion waiting, cancellation of child calls, and generation fences.
- Persisted subscriptions and activations are bound to the original definition. Startup reconciliation prunes cancelled work, recreates missing subscription jobs, and resumes claimed activations.
- External model checking explicitly reports skipped event-subprocess semantics. Legacy ScopeEventListener definitions require migration.
- Body-node execution listeners remain supported. Container execution listeners are explicitly rejected pending a container lifecycle implementation.
- Validation: 323 core tests (10 event-subprocess tests) and 80 parser tests pass; formatting and Clippy for core/parser/server pass. Further acceptance work remains: additional loop/race/termination coverage, REST and real NATS recovery tests, modeler properties and browser roundtrip tests, and final workspace/frontend checks. X9a is not complete.

## Modeler and integration checkpoint (2026-10-15)

- Added modeler fields for all five supported start types: shared references and names/codes, legal interruption flags, and timer type/expression. Declaration creation plus reference binding is one undo step; conflicting element IDs are rejected.
- Five Edge browser tests cover message undo/export/reimport, invalid reference types, timer replacement, error interruption rules, signal references, and escalation references. Frontend build and lint pass (lint retains repository warnings).
- Two REST tests cover unchanged XML export, explicit model-check skipping, repeated handlers, restart, shared writes, and interrupting signal cancellation.
- The real JetStream adapter suite passes all 15 tests, including handler/subscription recovery and original-definition binding after redeploying a clone.
- Fixed definition-key reuse on cloned redeployment: it now allocates a fresh key when the key already exists, preserving running instances and persisted old bodies. Distinct restored keys remain stable.
- Expanded runtime acceptance to 18 event-subprocess tests: parallel joins, multi-instance isolation/cancellation, other-handler cancellation, concurrent interrupting claims, termination cleanup, exact/wildcard error precedence, activity-boundary precedence, old snapshots, and invalid programmatic metadata before instance creation.

Validation for this increment: 331 core tests, 80 parser tests, 15 real-NATS adapter tests, 9 targeted REST tests (2 event-subprocess, 3 versioning, 4 start-error), and 5 Edge browser tests pass. Frontend build/lint, Rust formatting, and workspace Clippy also pass. The knowledge graph has been refreshed.

### Remaining X9a acceptance

- [x] Dedicated handler activation/interruption/completion history and scoped lifecycle notifications (implemented in the lifecycle checkpoint below).
- [x] Container start/end execution-listener lifecycle (implemented in the container checkpoint below).
- [x] Real NATS crash-point coverage for interruption claims with stale pending jobs (implemented in the lifecycle checkpoint below).

X9a acceptance is complete. Full X9 remains open: conditional starts and nested/compensation scopes remain the separate X9b/X9c increments.

## Lifecycle and interruption recovery checkpoint (2026-10-15)

- Added handler activation, completion, cancellation, and containing-scope interruption history. Context carries activation identity, definition key/version, generation, and reason. Cancellation is distinct from normal completion.
- Lifecycle entries are queued in a serde-defaulted instance outbox with stable history IDs. Writes acknowledge only successfully stored IDs. Failed writes retain pending records; startup reconciliation replays them. In-memory history now deduplicates entry IDs, matching the persistence retry contract and NATS behavior.
- Added `event_subprocess_changed` SSE notifications with stable event/activation IDs, owning instance/definition/subprocess IDs, phase, and reason. The desktop timeline refreshes for matching instance lifecycle events. SSE remains best effort; persisted history is the reload source after resync.
- Added coverage for repeated handler identity, history/SSE correspondence, stable, deduplicated history IDs during replay, persistence failure retention, and cancellation without false completion. The real JetStream interruption test restores a claimed handler with its task missing and a stale main task still stored; reconciliation removes stale work, recreates the handler task, and deduplicates unacknowledged history.
- Runtime/parser tests, real NATS adapter tests, frontend event tests/build/lint, and server REST/SSE checks validate this increment. Container execution listeners remain the final X9a implementation item; X9b/X9c remain separate increments.

Lifecycle checkpoint validation: 335 core tests, 80 parser tests, 16 real-NATS adapter tests, 29 server unit tests, 2 event-subprocess REST tests, and 9 frontend event tests pass. Frontend build/lint, Rust formatting, and workspace Clippy pass. Knowledge graph refreshed.

## Container listener checkpoint (2026-10-15)

- Support inline Rhai `start` and `end` execution listeners on event-subprocess containers. Start runs once per activation before body-node listeners and mappings. End runs after all handler branches and child calls finish; cancellation, termination, and failure skip normal end callbacks.
- Commit each callback chain's process-variable changes atomically. Persist the successful start marker with its variable changes before exposing body work; recovery also settles finished bodies with no runnable token left. Listener history carries the activation identity, phase, and variable diff.
- Reject unsupported callback events, external scripts/delegates, and non-Rhai languages. The modeler edits both container phases and retains them through XML export/reimport.
- Correct Rhai result extraction so the newest visible `let` binding wins over a shadowed process variable. Regression coverage includes repeated callbacks and multiple declarations.
- Added acceptance coverage for ordering, repeated activations, joins, child calls, cancellation, failure without partial writes, recovery before the body and after body completion, and original-definition callback binding across real JetStream reconnects.

X9a's supported process-level subset and its acceptance items are complete. Full X9 remains partial; X9b/X2 and X9c/X6/G1 remain follow-ups.

Container checkpoint validation: 342 core tests, 82 parser tests, 16 real-NATS adapter tests, 29 server unit tests, 2 event-subprocess REST tests, and 6 Edge browser tests pass. Frontend build/lint, Rust formatting, whitespace checks, and workspace Clippy with warnings denied pass. Knowledge graph refreshed.

## Conditional-start checkpoint (2026-10-15)

X9b is implemented and verified after user GO. Conditional handlers use validated
inline expressions and persisted false-to-true latches at successful global
variable commits, including task outputs, scripts/listeners, direct/file updates,
messages/signals, child outputs, and loop collections. Interruption and recovery
reuse X9a lifecycle machinery; modeler editing and native/Camunda REST paths are
covered. [Full decisions and validation](x2-conditional-events-plan.md).

X9a/X9b are complete within their process-level subset. Full X9 still requires
X9c/X6/G1; X2 intermediate/boundary/top-level conditional events remain open.
