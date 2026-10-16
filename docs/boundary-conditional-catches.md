# Conditional boundary catches (X2b)

A conditional boundary catches a committed process-variable condition while its
attached task activation is live. XML uses `conditionalEventDefinition` with one
inline `condition`, for example `amount > 100`. The strict simple-expression
grammar is shared with [intermediate catches](conditional-catches.md).

Supported attachments are ordinary non-loop user, manual, service, business-rule,
script, send, and non-instantiating receive tasks, including tasks in supported
process-level event-subprocess bodies and flattened ordinary subprocesses.
Call activities, loop/multi-instance activities, subprocess containers, incoming
flows to the boundary, condition filters/languages/resources, and top-level
conditional starts are rejected. Runtime MessageCatchEvent represents receive
work; XML validation additionally requires a receiveTask attachment in that scope.

## Activation and delivery

Successful activity start listeners run before subscription registration. Their
process writes are visible to initial evaluation. Task-local mapping inputs do
not trigger conditions. Initial true fires immediately, before task body execution
or external task publication. Start-listener failure registers no subscription.

The default `cancelActivity=true` closes the task activation and claims one
boundary branch. It cancels the original user/service/message/timer work. A late
completion cannot commit output or end-listener writes for that activation.
With `cancelActivity=false`, the activity remains live and a separate branch runs
on each observed false-to-true transition. Remaining true does not repeat delivery.
An independent branch survives ordinary completion of its attached task.

Completion and condition claims share the instance write lock. Completion closes
its subscriptions in the successful output/end-listener transaction, before
reevaluating other subscriptions. Its own completion outputs cannot interrupt the
activity that just completed. Failed end listeners retain the subscription and
pending task, discarding unsuccessful writes. Every task revisit has a fresh
activation identity; a stale callback cannot affect a later visit.

Boundaries on one activation use XML declaration order; programmatic definitions
use the `order` field, with node ID as a deterministic tie-break. An interrupting
winner stops later boundary evaluation for that activation. Already claimed
non-interrupting branches remain independent. Interrupting event-subprocess
handlers take precedence through the shared evaluator and generation fences.

Suspension defers evaluation/delivery. Resume evaluates current variables and
runs saved claims regardless of their condition's current value. No intermediate
false/true transitions are synthesized while suspended. Termination and scope
interruption invalidate pending subscriptions and branches.

## Persistence and observability

Instance snapshots retain activation/subscription IDs, latch values, original
source token, original sibling job descriptors/IDs/deadlines, generation,
revisions, claim IDs, and continuation tokens. Claim
persistence precedes branch publication. Cleanup retains cancelled source token
IDs and the exact original task IDs. Completion cleanup uses those task IDs even
when the next visit is at the same node. Interrupting claims use new branch token
IDs so cleanup cannot cancel a
later task visit. Saved claims recover even with an empty ready queue. Successful
completion persists the next central token with its output transaction, allowing
recovery before the next external task is published. Missing sibling jobs are
restored from the saved descriptors; boundary jobs alone do not mark an
unpublished task as parked. Delayed sibling deliveries validate membership in
the current activation before cancelling any other work.

New fields have serde defaults. Startup reconstructs legacy subscriptions only
when a central token and exactly one matching pending task/message wait identify
a live activation. Completed or ambiguous work is not reconstructed.

`GET /api/conditional-catches` includes `kind: boundary`, `attached_to`,
`interrupting`, and `activity_activation_id`, as well as existing pending metadata.
Waiting subscriptions and claimed occurrences have separate stable IDs. History
and SSE use the durable lifecycle outbox for registration, claims, subscription
closure, and consumption. Metadata excludes variable values.

The modeler edits Condition and Interrupting with undo and XML roundtrip support.
The external analyzer skips conditional runtime semantics while parser structural
validation remains active. Administrative token moves and migrations involving
conditional definitions remain rejected. See the [implementation and acceptance
plan](x2b-boundary-conditional-plan.md).
