# Intermediate conditional catches (X2a)

An intermediate conditional catch waits for a process-variable condition and
continues once for that arrival. It works in ordinary flow, event-subprocess
handler bodies, flattened ordinary subprocesses within their existing limits,
and exclusive, non-instantiating event-based gateway alternatives.

```xml
<intermediateCatchEvent id="threshold">
  <conditionalEventDefinition>
    <condition xsi:type="tFormalExpression">amount &gt; 100</condition>
  </conditionalEventDefinition>
</intermediateCatchEvent>
```

Declare `xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"` in the document.
Conditions use the [X9b expression grammar](event-subprocesses.md#conditional-starts-x9b):
an ASCII variable name or one scalar comparison; explicit languages, resources,
filters, compound expressions, and calls are rejected. The selected executable
process is validated, including supported handler/subprocess bodies.

If the condition is already true on arrival, the token continues immediately.
Otherwise, successful global variable commits reevaluate the waiting occurrence.
X11 task-local inputs remain invisible; committed outputs, scripts/listeners,
messages/signals, child-call results, loop collections, and direct/file API writes
participate through the same evaluator as X9b. Further changes cannot release a
consumed wait again. A loop revisit gets a new occurrence and can pass while the
condition remains true. This is distinct from X9b's persistent rearm latch.

Start listeners execute once per standalone arrival, before evaluation. A
successful start marker and its writes share the saved token state. End listeners
run on consumption. Their writes, occurrence consumption, continuation position,
and queued history commit atomically before subsequent waiting work is exposed.
An end-listener failure discards that chain's partial writes and fails the scope.
Replayed or cancelled claims cannot repeat callbacks or replace newer work.

## Event-based gateways

All timer/message/signal/conditional alternatives share a persisted winner claim
per gateway arrival. The first successful claim wins. Conditional alternatives
already true at registration are considered in definition sequence-flow order.
Only the winner executes target listeners; losing alternatives are cancelled.
Gateway targets cannot have another incoming sequence flow.

The group stores its occurrence identity, owning token, alternative job IDs,
original pending descriptors, and winner. Pending timer/message wire formats
remain compatible: job-ID membership identifies their group. Cleanup targets
those saved IDs, so an old delivery cannot cancel a newer loop occurrence.
Legacy gateway waits are upgraded only when the saved token, definition, and
complete restored alternative set identify one unambiguous wait. Ambiguous
restores fail explicitly instead of guessing or dropping work.

## Recovery and visibility

Waits, claims, and groups persist in the serde-compatible instance execution
metadata, including processes that have no event subprocesses. A saved claim
continues even if the condition has subsequently become false. Original
definition binding, stable history IDs, and ready continuations survive restart.
Missing gateway jobs are recreated with their saved IDs and timer deadlines;
loser cleanup is replayable. External effects retain the existing worker and
persistence idempotency limits; this does not guarantee distributed exactly-once
delivery.

Suspension defers activation and coalesces changes. Resume evaluates current
globals and drains saved claims. Interruption, termination, and deletion discard
their waits; queued stale occurrences cannot revive execution. Parked and claimed
waits keep their owning branch alive. Existing conditional-chain and execution
limits apply.

`GET /api/conditional-catches` lists occurrence, instance, node, token, generation,
and claimed status without copying process variables. Instances expose
`WaitingOnConditional { node_id }`; compact SSE updates carry that status and
the desktop displays **Waiting for condition**. Parallel execution retains its
existing aggregate state. The modeler exposes a **Condition** field with undo
and XML export/reimport support.

History uses the catch node and stable occurrence/token identity, with definition,
generation, group, and variable-revision context on consumption. The external
analyzer explicitly skips conditional semantics; structural validation still runs.

Top-level conditional starts and conditional boundary events remain rejected.
Nested runtime scopes remain an X6 prerequisite. Administrative token movement
and migration involving conditional definitions remain rejected before mutation.
See the [X2a plan and acceptance](x2a-intermediate-conditional-plan.md).
