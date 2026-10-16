# X2b — Conditional boundary catches

Status: completed and verified after user GO, 2026-10-16. X2 remains partial.

## Scope

Add conditional boundary events attached to ordinary, non-loop user, service,
script, send, receive, manual, and business-rule tasks, including tasks in
supported process-level event-subprocess bodies. Support interrupting
(`cancelActivity=true`, including the default) and non-interrupting boundaries.
Use the existing strict inline condition grammar and process-variable commit
hook. Preserve existing timer/message/signal boundary behavior.

Reject attachments to call activities, embedded/event subprocess containers,
loop or multi-instance activities, and other unsupported targets at deploy.
Those need a separate increment for child cancellation, loop ownership, or X6
runtime scopes. Top-level conditional starts remain separate X2 work.

## Observable contract

- Register a persisted boundary subscription for each activity activation before
  publishing external task work or executing the activity body. An activation
  gets a fresh identity on every loop-free revisit, even if a token ID is reused.
- Evaluate immediately at activation and after each successful process-variable
  commit while the activation remains live. Failed or purely local writes do not
  trigger evaluation. An initially true condition fires immediately.
- An interrupting claim atomically closes the activity activation, invalidates its
  pending work and other boundaries, and records one boundary continuation.
  Completing a cancelled task returns the existing inactive/not-found response.
- A non-interrupting claim creates an independent branch and keeps the activity
  live. It fires once while true and rearms only after an observed false value;
  persisting the latch prevents repeat branches on polling or restart.
- Claim and activity completion compete under the same instance write lock.
  Completion closes subscriptions before evaluating outputs/end-listener writes,
  so those writes cannot interrupt the activity that just completed. A claim
  committed first prevents that activity's completion and output writes.
- For immediate interrupting claims, skip activity body and external work.
  Activity start listeners run before subscription registration; their successful
  writes are visible to initial evaluation. A start-listener failure registers no
  subscription. For synchronous task writes, close the successful activation in
  the same transaction as outputs/end listeners; concurrent commits can claim
  only before that transaction wins. Add explicit tests for these orderings.
- Multiple eligible boundaries on one activation use definition declaration order
  (preserved by parsing). Stop after an interrupting winner; already committed
  non-interrupting claims remain valid independent branches.
- Keep handler-generation fencing. A spawned branch inherits the owning handler
  activation when present. Process termination or scope interruption cancels it;
  ordinary activity completion does not cancel an already claimed branch.
- Suspension defers delivery and task execution, retaining committed claims.
  Resume evaluates current variables, then delivers saved claims without checking
  their condition again. No synthetic false/true transitions during suspension.

## Implementation outline

1. Add BoundaryConditionalEvent with attachment, condition, and interrupting flag.
   Extend selected-process XML validation and both ordinary/flattened parsing;
   reject empty expressions, filters, language/resource expressions, invalid
   targets, incoming boundary flows, and unsupported loop/scope attachments.
   Validate programmatic definitions too. Compile through the existing event
   condition cache, keeping gateway expression semantics unchanged.
2. Persist activation IDs, subscription IDs, owning token/node/generation,
   condition latch, claim ID, spawned token ID, and registration/claim revision.
   Give new fields backward-compatible serde defaults. Existing snapshots without
   boundary state remain readable; reconstruct subscriptions only for provably
   live parked task activations, never for completed or ambiguous work.
3. Integrate registration/closure with task activation and successful completion,
   plus the shared lock-held conditional evaluator. Audit native and Camunda
   completion, scripts/listeners, direct variables, mappings, and handler commits.
   Do not run async cleanup or callbacks while holding the instance write lock.
4. Persist claim and cancellation intent before exposing branches. Reuse durable
   ready-work recovery, adding boundary-specific occurrence validation. Retry
   cancellation cleanup using original task/job IDs; never cancel a later visit.
   Prevent replay from repeating committed callbacks or overwriting newer tokens.
5. Remove subscriptions on completion, cancellation, termination, and relevant
   handler interruption. Prevent administrative token move/migration from leaving
   orphaned boundary state; retain existing conditional-definition restrictions.
6. Extend modeler Condition editing to conditional boundaries, preserving
   cancelActivity and undo/export/reimport behavior. Expose pending activation and
   claim metadata through the existing conditional-catches API with an explicit
   catch kind and optional attached activity fields; preserve intermediate fields
   and synchronize TypeSpec/OpenAPI. Include history/SSE records for registration,
   claim, branch creation, and cancellation without exposing variable values.
7. Update runtime contract, parser support matrix, README, and backlog after
   verification. Keep X2 partial and document the excluded attachment types.

## Acceptance gates

- Parser: default/explicit interrupting flags, non-interrupting form, supported
  task kinds, handler bodies, flattened task IDs, invalid expressions/filters,
  unsupported attachments/loops/scopes, and unrelated undeployed-pool isolation.
- Engine: initially true/false; false→true→true→false→true; separate simultaneous
  activations and task revisits; multiple boundary ordering; parallel joins;
  handler-generation isolation; listener/output ordering and failed writes.
- Race/cancellation: boundary versus native/Camunda completion, timer/message/
  signal boundary delivery, termination, suspension/resume, late worker result,
  stale queued claims after revisit, and cancellation of all original work only.
- Recovery: waiting, claimed before branch publication, branch consumed before
  next task publication, cleanup retry, empty ready queue, version-pinned
  definition after redeploy, and legacy snapshots with/without unambiguous waits.
  Run real JetStream recovery tests, not only in-memory snapshot tests.
- REST: pending metadata and status/history visibility, native and Camunda
  completions, variable updates, and no duplicate branches after retries.
- Browser: edit condition/interrupting flag, undo, XML export/reimport, and existing
  X9/X2a regression coverage. Run frontend event tests, build, and lint.
- Run core/parser/server and affected NATS/REST suites, strict workspace Clippy,
  formatting, TypeSpec compile, diff check, and graphify update. Run Cargo commands
  sequentially. Record actual passing counts and any limitations before closure.

This plan defines the supported engine contract. It does not claim complete BPMN
conditional-event coverage. Call/loop boundaries and container scopes remain open.

## Implementation checkpoint (2026-10-16)

Implemented the boundary element/parser and same-scope task validation, strict
compiled conditions, per-visit identities and per-boundary subscription IDs,
persisted transition latches, atomic claim/completion ordering, cancellation
intent, durable branch callbacks/continuations, and startup reconciliation.
Receive tasks retain their existing MessageCatchEvent runtime mapping. Sibling
boundary jobs retain their existing persisted schemas; activation-owned job IDs
fence delayed timer/message/signal deliveries from later task visits.

Modeler Condition/Interrupting editing and TypeSpec/OpenAPI pending metadata are
implemented. The runtime contract is in [boundary conditional
catches](boundary-conditional-catches.md). Acceptance passed: 396 core, 86 parser,
19 real NATS, 29 server, 7 REST, 9 focused browser, and 9 frontend event tests.
Frontend build/lint, TypeSpec compilation, synchronized OpenAPI, Rust formatting,
strict workspace Clippy, and diff checks pass. Graphify was refreshed.
Nine focused X2/X9 browser tests and frontend build/lint/nine event tests pass.
A broader exploratory browser run encountered an unrelated Monaco initialization
failure; the focused editor suite excludes that monitoring test.

Final recovery review added exact task-ID cleanup for completion/self-loop crash
points, per-visit start-listener reset, sibling descriptor/deadline restoration
before task publication (including suspended restart/resume), delayed-job
membership checks, and recurring-timer regression coverage. The resulting core
suite passes 396 tests and strict workspace Clippy passes. The final real NATS
and server/REST reruns pass (19, 29, and 7 tests respectively).

The next proposed increment is [X2c explicit top-level conditional starts](x2c-top-level-conditional-starts-plan.md); its separate GO is pending.
