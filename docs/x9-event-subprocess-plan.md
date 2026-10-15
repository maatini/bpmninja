# X9 — Event subprocesses

Status: X9a authorized by continuation, 2026-10-15. Parser/runtime implementation in progress.
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
  event forms implemented as a side effect. Needs its own plan and GO.
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
- Parser and core suites pass at this checkpoint. Further acceptance work remains: additional loop/race/termination coverage, REST and real NATS recovery tests, modeler properties and browser roundtrip tests, and final workspace/frontend checks. X9a is not complete.
