# X2a — Intermediate conditional catches

Status: completed and verified after user GO, 2026-10-15.
The backlog requires a separate plan and GO before each Phase 4 increment;
the [X2/X9b plan](x2-conditional-events-plan.md) also requires a concrete
acceptance plan for each remaining conditional-event increment.

## Scope and observable behavior

Support `conditionalEventDefinition` on intermediate catch events in ordinary
process flow and X9a/X9b handler bodies, including alternatives after an existing
exclusive, non-instantiating event-based gateway. Reuse X9b's validated condition
grammar and compiled expression cache. Boundary events, top-level starts,
variable-event filters, nested runtime scopes, multiple events, and parallel or
instantiating event gateways remain rejected.

Example: `start → wait(amount > 100) → review`. Reaching the catch with amount 150
continues immediately. Reaching it with amount 90 parks that token; a successful
process-variable commit to 150 releases it once. Further updates cannot release
that completed wait again. A later loop visit creates a new waiting occurrence,
which can pass immediately even if the condition has remained true.

This differs from a repeated event-subprocess subscription: the catch belongs to
one token/wait occurrence, consumes once, and does not retain a rearm latch after
consumption. Task-local inputs are not global conditional triggers.

## Current code and required integration

| Area | Current behavior and change |
| --- | --- |
| Parser | `models.rs::BpmnIntermediateCatchEvent` has timer/message/signal/link definitions. Add conditional extraction/validation in ordinary and flattened-subprocess parsing; share strict XML validation with X9b, including prefixed attributes. Extend only the intermediate-catch allowlist in `parser.rs::supported_event_definitions`. |
| Domain | Add an executable conditional-catch variant and include its expression in the separate conditional cache. Permit it as an event-based gateway target. Validate gateway-target ownership and reject extra incoming flows on those targets. |
| Initialization | `initialize_event_subprocesses` currently exits when no handlers exist. Initialize conditional execution metadata for definitions containing catches even without event subprocesses, including API, timer, message/signal, and child-call starts. Preserve legacy snapshots without this metadata. |
| Shared evaluator | `evaluate_conditionals` currently exits unless a conditional handler exists. Dispatch both handler subscriptions and live catch records under the same process-variable commit lock and revision. Installing a new wait must evaluate it even when global values/revision have not changed. |
| Execution | Add a conditional waiting action/status. Park the central token atomically with its wait record; a live catch keeps its branch/process active. Reuse the existing durable ready-work drain, generation fences, and execution-chain limit. |
| Gateway | `handlers/gateways.rs` builds timer/message/signal alternatives sharing a token. Add a common persisted group/winner claim used by all alternative kinds; extending the target match alone is insufficient. |
| Lifecycle | Extend `clear_wait_states_for_token`, loop cancellation, interruption, termination, deletion, suspension/resume, and reconciliation to include conditional waits and gateway groups. |
| Presentation | Add conditional waiting state to server/SSE/desktop status mappings and instance tracking. Expose pending conditional waits with their owning instance/node; reuse the condition editor for intermediate catches. |

## Persisted state and atomic claims

Keep the existing `EventSubProcessState` JSON field compatible; extend it with
serde-defaulted conditional-wait and gateway-group maps. Its execution metadata
can exist with no handler subscriptions. Document this wider responsibility
rather than silently changing old serialized field names or resetting X9b latches.

Each wait record needs a stable occurrence ID, token ID, catch node ID, original
definition key, cancellation generation, optional gateway-group ID, registration
revision, and phase (`waiting` or `claimed`). A loop revisit or a distinct
parallel token gets a distinct occurrence. Do not key waits by node ID alone.

1. Register and evaluate a catch while holding the instance write lock. Evaluate
   against committed process globals. Initially true means claimed immediately;
   false means waiting. Reconcile registration with commits under that same lock
   so a change cannot fall between the arrival check and subscription creation.
2. Evaluate waiting catches on successful global commits through the existing
   hook. Observe the same snapshot as X9b handlers. Claim interrupting handlers
   first in their existing order; do not release catches in an invalidated
   generation. Then consider surviving catches in stable occurrence order.
3. Transition a catch/group to claimed and enqueue durable continuation work in
   the same instance transaction. Save before publishing the next waiting task.
   A later value change does not revoke a successful claim.
4. On claimed-catch execution, run its end-listener chain against a candidate.
   Commit listener writes, completion history/outbox, the next central-token
   position, and wait consumption together before exposing continuation work.
   Recovery must not repeat a committed listener chain. Failed chains discard
   partial variables and follow the documented scope execution-error behavior.
5. Start listeners run once per arrival before registration/evaluation; persist
   their successful marker and writes with the parked/claimed occurrence. A
   listener that makes the condition true must not leave a false parked wait.
6. Refresh token process values from the committed scope while retaining existing
   local/loop isolation rules. Do not overwrite concurrent globals with a stale
   parked token when resuming it.

## Event-based gateway competition

Introduce a persisted wait-group identity per gateway arrival. Timer/message/
signal pending jobs carry an optional, serde-defaulted group ID; conditional
alternatives reference the same group. Preserve current formats for ordinary
non-gateway waits.

- Install the group and all alternatives atomically before any alternative can
  publish work. Evaluate conditional alternatives on arrival. If several are
  already true, choose the first in definition sequence-flow order, documented
  as the engine's tie-break rule.
- Each timer, message, signal, or conditional delivery atomically claims the same
  unclaimed group under the instance lock before consuming its token. Whichever
  claim succeeds first wins; late deliveries do not create another continuation.
  Do not claim that this is distributed exactly-once delivery.
- Persist the winner and ready continuation before asynchronous loser cleanup.
  Keep loser job IDs pending until cleanup succeeds, allowing crash replay.
  Cleanup is scoped to that group occurrence so an old timer cannot cancel a
  newer loop visit that reused the same token/node.
- Run start/end listeners only on the winning target; losing catch alternatives
  must not produce callback writes or normal catch-completion history.
- Retain gateway/fork/join ownership and event-subprocess activation identity.
  Do not turn every losing alternative into an independently completable branch.

For existing gateway snapshots/jobs without group IDs, reconstruct a group only
when the central token, original definition, and restored alternatives identify
one unambiguous current wait. Persist the upgraded group before new evaluation.
An ambiguous restore fails explicitly; it must not guess, select the latest
definition, or discard waiting work. Include legacy gateway fixtures in tests.

## Recovery, suspension, and administrative operations

- Reconciliation must recognize conditional waits as parked work rather than
  replaying the catch as a fresh arrival. A saved claim resumes without waiting
  for another variable change, even if the current condition is now false.
- Resume missing claimed continuations and retry loser cleanup with stable IDs.
  Bind expressions to the saved definition, preserving X9b's version behavior.
- Suspended instances install no new live work. Committed variable changes are
  coalesced; resume evaluates the current globals and drains saved claims before
  normal continuation. Include restart while suspended, not only live resume.
- Interruption/termination/deletion remove waits and groups, invalidate queued
  continuations, and cancel related child/loop work. Stale worker/timer/message
  deliveries cannot revive them. Completion counts parked and claimed catches.
- Reject token movement or migration involving conditional catches before any
  mutation until wait ownership and group remapping have a supported contract,
  consistent with current X9b restrictions.
- Reuse durable history/outbox IDs for wait registration/consumption; include
  occurrence, token, node, definition, generation, group, and revision context.
  Pending-state/SSE payloads should not copy process variables unnecessarily.
- External model checking explicitly skips the new conditional semantics rather
  than returning an unsupported success result.

## Acceptance and delivery

1. **Parser/domain:** ordinary and handler-body catches, flattened ordinary
   subprocesses within existing limits, valid/invalid expressions and qualified
   attributes, gateway targets/extra incoming flows, unchanged XML export, old
   snapshots, and continued rejection of unsupported conditional event forms.
2. **Runtime:** initially true/false, no-op writes, deletion, every shared commit
   origin from X9b, single consumption, loop revisits while true, independent
   parallel tokens/handler activations, listener ordering and failure atomicity,
   cancellation, generation fencing, completion waiting, and bounded chains.
3. **Gateway races:** condition versus message/signal/timer, competing initially
   true conditions, concurrent deliveries, lost cleanup acknowledgement, stale
   jobs after loop revisit, loser listeners, and fork/join completion identity.
4. **Recovery:** real NATS restart before publication and after claim/history
   append, true/false state after a saved claim, missing continuation task,
   repeated reconciliation, legacy wait groups, original-definition binding,
   and restart/resume while suspended.
5. **REST/modeler:** direct/native/Camunda commits release the same wait once;
   pending/status/SSE visibility; editor undo and export/reimport browser tests.
   Existing timer/message/signal gateway REST tests remain passing.
6. Update the runtime contract, README, parser element matrix, and backlog.
   Run affected Rust suites, real NATS tests, REST and browser acceptance,
   frontend build/lint, formatting, workspace Clippy with warnings denied, and
   `graphify update .`. Keep Cargo builds/checks serial and temporary stores
   isolated/ignored. Mark X2a complete only after these checks pass.

## Semantic reference

[OMG BPMN 2.0.2](https://www.omg.org/spec/BPMN/2.0.2/PDF), Clauses 10.5, 10.6 and
13.5.2, establishes intermediate waiting/consumption and includes conditional
triggers among valid event-gateway targets (printed page 297). The grammar,
tie-break order, persistence format, and suspension policy above are explicit
engine choices. This plan does not claim full BPMN or Camunda conformance.

Boundary conditional catches and top-level conditional starts remain separate
X2 increments after X2a; nested/compensation event subprocesses remain X9c/X6/G1.

## Implementation checkpoint (2026-10-15)

- Implemented the conditional-catch variant, shared XML/grammar validation,
  independent compiled conditions, catch-only execution initialization, and
  listener preservation in flattened ordinary subprocesses.
- Persisted occurrence identity, registration/claim revision, generation, phase,
  optional group identity, and token marker. Callback writes and continuation
  state commit together. A runtime execution lock plus occurrence validation
  prevents replay from replacing newer work or repeating committed callbacks.
- Gateway groups own the original timer/message descriptors and job-ID membership
  rather than adding fields to every pending job. Existing pending-job formats
  remain unchanged. All delivery kinds use the same atomic winner transaction.
  Losing conditional records are removed in that transaction; cleanup remains
  scoped to saved job IDs and original timer deadlines.
- Recovery distinguishes parked catches, saved claims, missing continuation work,
  and unambiguous legacy gateway groups. Suspension/resume can recover a consumed
  catch whose next waiting task was not yet published. Cancellation clears its
  queued claims as well as wait records.
- Added pending/status/SSE visibility, the intermediate condition editor, and
  `GET /api/conditional-catches` with synchronized TypeSpec/OpenAPI output.
- Acceptance passed: 374 core, 84 parser, 18 real NATS, 29 server, 6 REST,
  8 browser, and 9 frontend event tests. Frontend build/lint, API-spec compilation,
  Rust formatting, and strict workspace Clippy also pass. Graphify was refreshed.

Boundary catches and top-level conditional starts remain separate X2 increments.
