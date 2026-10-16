# X2 / X9b — Conditional events

Status: X9b implemented and verified after user GO, 2026-10-15. X2a intermediate catches are complete. X2 remains partial; X2b task boundaries are complete and verified; call/loop/container boundaries and top-level conditional starts remain later increments.

## First increment: shared evaluation and X9b

Implement conditional starts on event subprocesses directly under a process,
using the X9a activation, interruption, cancellation, history, and recovery
machinery. Build the variable-change hook so intermediate and boundary catches
can reuse it later. This increment completes X9b, not X2 or full X9.

Example: `amount > 100` is initially false. A successful task completion commits
`amount = 150`, activating the handler before normal flow continues. A later
update to 160 does not start another activation while the condition remains true.
Committing 90 and subsequently 150 rearms and triggers another non-interrupting
activation. An interrupting handler invalidates the committing token before it
can expose its next task or start a child call.

Top-level conditional starts, intermediate catches, boundary catches, nested
scopes, variable-event filters, and arbitrary expression languages remain explicit
deployment errors in this increment. Do not silently ignore Camunda
`variableName` / `variableEvents`, FEEL, JUEL, script resources, or delegates.

## Expression contract

- Accept one nonempty inline `bpmn:condition` formal expression under exactly one
  `conditionalEventDefinition` on the handler's sole start event. Preserve the
  original XML for export. Both interruption modes are supported.
- Reuse the engine's simple condition semantics: a variable truthiness check or
  a comparison against a literal using `==`, `!=`, `>`, `<`, `>=`, or `<=`.
  Publish examples, variable-name rules, supported literal types, missing/null
  behavior, truthiness, and numeric comparison behavior in the runtime contract.
- Add strict validation for this new use of the evaluator. The existing
  `CompiledCondition::parse` accepts arbitrary text as a variable name and falls
  back to strings for malformed literals; that is insufficient deployment
  validation. Reject unsupported operators, compound expressions, empty operands,
  and unsupported language declarations. Keep existing gateway behavior stable.
- Compile validated conditions on deployment/restore, outside the instance lock.
  Evaluation must be synchronous, bounded, and free of writes or external calls.
  Missing variables evaluate false under the existing comparison rules.

## Trigger and ordering contract

1. Evaluate on subscription installation after defaults and caller variables are
   available, before main-flow execution. Initially true conditions activate once.
2. Thereafter evaluate against each successfully committed process-variable
   snapshot. Only changes to JSON values or key presence create a revision;
   identical assignments, type-hint changes, reads, and failed/rolled-back
   completions do not. Deletion participates in evaluation.
3. Persist the last boolean value per subscription. A false-to-true transition
   triggers; a true-to-true evaluation does not. Returning false rearms a
   non-interrupting subscription even if an earlier activation is still running.
   This is the proposed engine retrigger contract, not a claim of Camunda parity.
4. Evaluate all eligible subscriptions against the same committed snapshot in
   stable subprocess-ID order. Claim non-interrupting handlers in that order
   until an interrupting claim invalidates the containing generation; stop then.
   Never depend on HashMap iteration order or script execution timing.
5. Commit variable changes, evaluation state, activation tokens, cancellation
   generation, and lifecycle outbox together under the owning instance write
   lock. Refactor the claim logic into a shared lock-held operation; do not
   evaluate, release the lock, then call an API that claims against newer values.
6. Persist the resulting snapshot before publishing pending handler work.
   Release instance locks before backend I/O, child cancellation, and execution.
   Use the existing work queue to run claimed handlers; avoid recursive batches
   and lock re-entry from listeners.
7. Drain conditional claims before the committing token resolves/exposes normal
   continuation. Recheck its generation after interruption. Completion must
   recheck live tokens, active handlers, and child calls under the final state
   lock so a concurrent claim cannot be overwritten by `Completed`.
8. Container/body listener writes can trigger other handlers and later genuine
   transitions. A latch prevents repeated activation solely because a condition
   stays true. Also bound chained conditional activations per execution batch:
   alternating handler writes can otherwise oscillate indefinitely. Exhaustion
   must produce a documented execution error, cancel pending scope work, and
   preserve diagnostic history; never drop claims silently.

## Variable commit inventory

Use a shared process-variable commit helper with explicit before/after values,
revision, and origin. Audit every assignment rather than hooking persistence or
SSE: both run for changes unrelated to variables and can miss intermediate
committed transitions.

| Origin | Integration points and required behavior |
| --- | --- |
| Instance creation | `process_start.rs`: defaults/caller inputs, ordinary/API/child start paths; install once, do not reinitialize on restore. |
| Direct API/file updates | `instance_ops.rs::update_instance_variables`, used by server instance/file routes; process variables and central tokens must become consistent in one write transaction before evaluation. |
| User/service completion | `user_task.rs`, all native/typed/Camunda completion paths in `service_task.rs`; evaluate only after mappings/listeners succeed and rollback is no longer possible. |
| Scripts and body listeners | `handlers/tasks.rs`, `executor/completion.rs`, `scripting/runner.rs`, and executor variable merge; evaluate committed globals, not intermediate script statements. |
| X11 mappings | `engine/data_mappings.rs` and completion paths; task-local inputs are invisible, successful process outputs are visible before outgoing flow decisions. |
| Messages and signals | `message_processor.rs` and handler activation payload merge; include successful process writes, preserve the existing payload visibility contract. |
| Child calls | `process_start.rs` parent-resumption/output transfer; evaluate the parent's committed mapped outputs before parent continuation. |
| Handler containers | `event_subprocess.rs` start/end callback commits; preserve atomic callback-chain failure behavior and original-definition binding. |
| Loops and parallel merges | `loops.rs`, `executor/mod.rs`; cover each actual process commit without double-counting token synchronization. |
| Administrative operations | `instance_ops.rs` token movement and migration involving conditional subscriptions are rejected before mutation: existing relocation resets execution registries without preserving subscription/activation ownership. Direct variable updates remain supported. |

Keep unrelated concurrent writes intact. Distinguish global deltas from stale
token snapshots and local X11 variables. A direct variable update must not expose
a new global map alongside old central-token copies while another completion runs.

## Persistence, suspension, and failure

- Extend persisted subscription state with serde-defaulted conditional state and
  evaluation revision. Persist a monotonic variable revision with the instance.
  Keep enum/definition decoding compatible with existing X9a snapshots.
- Bind expression text and compiled form to the original definition key/version.
  A redeployed clone must not change a running instance's expression.
- Reconcile saved conditional subscriptions and claimed tokens without resetting
  their latches, recreating timer/message jobs for them, or generating duplicate
  lifecycle IDs. Saved claims resume through existing activation recovery.
- For old snapshots without conditional state, only initialize subscriptions
  actually present in the saved definition; never retrofit handlers from latest.
- While suspended, allow the existing variable-update contract but defer
  activation. On resume, evaluate the current snapshot once against the last
  evaluated state before resuming runnable work. Intermediate changes made while
  suspended are deliberately coalesced. Persist resume claims before execution.
- Completion/deletion/termination remove subscriptions and deferred evaluation;
  terminal instances cannot be revived by variable updates. Recovery must settle
  pending evaluation before declaring the containing scope complete.
- Preserve existing completion retry behavior: failed mappings/listeners must
  neither advance the latch nor leak a handler. Stale-generation errors must not
  fail a newly interrupting handler's generation.
- Reuse durable X9a lifecycle history/SSE. Add trigger kind and variable revision
  to activation context; avoid copying arbitrary variable payloads into SSE.
  External effects remain subject to the existing persistence/idempotency limits.

## Acceptance and delivery

1. Parser/domain: valid forms and both flags; strict malformed/language/filter
   rejection; XML retention; old definition/instance decoding.
2. Core: initially true/false, false→true→true→false→true, no-op/delete updates,
   concurrent updates, deterministic competing handlers, handler self-writes,
   bounded oscillation, shared-variable isolation, and every commit origin above.
3. Completion races: interrupting activation prevents the old next task/child;
   concurrent non-interrupting claim survives process-completion checks; a stale
   callback error cannot fail the new generation.
4. X11/task compatibility: local inputs do not trigger; committed outputs do;
   failed outputs/end scripts remain retryable without activation or latch drift.
5. Recovery: real NATS crash points before claim publication and after history
   append, repeated restart, saved true latch, missing claimed task, cancelled
   stale jobs, original-definition expression, suspension/resume, completed body.
6. REST/native/Camunda: direct update, user/service completion, file update path,
   child outputs, and restart all reach the same evaluator.
7. Modeler: add condition editing to `EventSubprocessPropertiesProvider.ts` with
   interruption controls, validation, undo, XML export/reimport browser coverage.
   Model checking must explicitly skip unsupported conditional semantics.
8. Update README, event-subprocess contract, parser element matrix, and backlog.
   Run affected suites, frontend build/lint/browser checks, formatting, workspace
   Clippy, and `graphify update .`. Run Cargo checks serially.

## Later X2 increments

- **Intermediate catches (X2a):** [Completed plan and acceptance](x2a-intermediate-conditional-plan.md); implementation and integration acceptance complete. Persisted per-token subscriptions, immediate
  evaluation on arrival, single consumption, event-based gateway competition,
  loop/parallel activation isolation, and restore/cancellation.
- **Boundary catches (X2b):** [Concrete implementation and acceptance plan](x2b-boundary-conditional-plan.md), completed and verified 2026-10-16. Per-activity activation identity, interrupting versus
  repeated non-interrupting branches, exact task/job cancellation, and precedence
  with activity completion are implemented for ordinary tasks. Call/loop/container
  attachments remain excluded; subprocess scope behavior still requires X6.
- **Top-level starts (X2c):** [Concrete next-increment plan](x2c-top-level-conditional-starts-plan.md), prepared 2026-10-16; awaiting separate GO. A definition-level evaluation API and explicit input
  context, definition-version selection, deduplication/retrigger ownership, and
  startup/redeploy behavior. An existing instance's variable update must not
  implicitly launch unrelated process instances.

Each later increment needs its own concrete acceptance plan. X2 remains open
after X9b; nested/compensation event subprocesses remain X9c/X6/G1.

## References and limits

The normative target is [OMG BPMN 2.0.2](https://www.omg.org/spec/BPMN/2.0.2/PDF),
Clause 10.5: conditional catches react when their condition becomes true, and
executable conditional definitions require a formal expression. The latch,
commit ordering, supported expression grammar, and suspension policy above are
explicit proposed engine choices within this increment's limited scope.

[Camunda conditional-event documentation](https://docs.camunda.io/docs/components/modeler/bpmn/conditional-events/)
provides a comparison for event-subprocess interruption and repeated activation.
Its FEEL/filter behavior does not define this engine's expression or retrigger
contract. This plan does not claim BPMN or Camunda conformance.

## Implementation and acceptance checkpoint (2026-10-15)

- Added validated, separately compiled conditional expressions without changing
  gateway parsing. Both event-subprocess interruption modes are supported;
  prefixed unsupported filters/resources/languages are rejected.
- Existing activity-specific write/rollback paths now invoke a shared lock-held
  evaluator. Its observed variable snapshot, revision, transition latch, claimed
  tokens, cancellation generation, and lifecycle outbox share instance state.
  Direct API updates synchronize globals and central tokens in one transaction.
- Durable ready claims use the existing activation/recovery machinery and a
  serialized publication drain. Recovery retains successful start-listener
  markers and true latches. Cancellation remains pending until cleanup finishes.
- Added generation checks before continuation, a final completion recheck, bounded
  conditional chains, multi-instance output-collection evaluation, and safeguards
  against redundant script writes or failed completions restoring stale globals.
- The modeler edits conditions with undo and XML roundtrip support. The runtime
  contract documents grammar, truthiness, transition/rearm rules, suspension,
  revision history, and explicit administrative relocation restrictions.
- Validation passed: **358 core tests, 83 parser tests, 17 real-NATS adapter tests,
  29 server unit tests, 4 event-subprocess REST tests, and 7 Edge browser tests**.
  REST includes native/Camunda completion and restart. Real NATS covers a missing
  claimed task, retained start marker/latch, repeated reconciliation, and original
  expression binding after redeployment. Frontend build/lint, formatting,
  whitespace checks, and workspace Clippy with warnings denied pass. Knowledge
  graph refreshed; temporary browser config and JetStream store removed.

X9b is complete within this process-level subset. X2 and full X9 remain partial.
