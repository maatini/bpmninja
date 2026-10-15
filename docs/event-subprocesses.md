# Event subprocess runtime (X9a)

An event subprocess directly inside a process executes in the same process
instance. Its start is a subscription, rather than an additional ordinary start
or a Call Activity. Deploy the XML through the existing deployment API and use
the existing message, signal, timer, or worker-error interfaces to trigger it.

Supported starts are timer, message, signal, error, and escalation. Error starts
must interrupt; the other supported starts accept `isInterrupting="false"`.
Omitting the flag means interrupting. Message/signal references resolve declared
names; error/escalation references resolve codes. An omitted error/escalation
reference catches any code, with an exact match taking precedence.

Non-interrupting subscriptions remain available while the process scope is live.
Repeated triggers create separate activations, including separate parallel joins
and loop state. The main flow can finish while handlers remain active; process
completion waits for them. Subscriptions alone do not keep the process alive.
An interrupting activation cancels main execution and other handler activations,
including outstanding child calls. Cancelled task completions cannot resume that
execution. Successful completion of the remaining handler completes the scope.

Handlers share process variables. A completing token applies changes relative to
its activation baseline, preserving unrelated concurrent writes. Writes to the
same variable follow completion order. X11 task-local mappings still apply inside
handlers; this does not introduce a handler-local variable scope.

Instances persist their definition key, subscriptions, activation identities, and
cancellation generation. They continue using the original deployed definition.
After restoring definitions, instances, and pending jobs, call
`reconcile_event_subprocesses`; server startup does this automatically. Recovery
removes stale cancelled jobs, recreates missing subscription jobs, and resumes
claims saved before the handler exposed its first waiting task. This is not an
exactly-once guarantee for external side effects; workers still need idempotency.

Nested event subprocesses, conditional/compensation starts, and handler-level
data declarations/mappings are rejected. Container execution listeners accept
inline Rhai scripts for start/end; external delegates/resources and other listener
events/languages are rejected. Legacy
programmatic `ScopeEventListener` definitions and persisted nil-token listener
subscriptions require migration to explicit `EventSubProcess` metadata.

The external model analyzer reports event subprocess semantics as `skipped`;
parser structural validation still runs. It does not certify interruption or
repeated activation. Supported-scope acceptance and follow-up increments are tracked in the [X9 plan](x9-event-subprocess-plan.md).

## Editing a start in the modeler

Select the start event inside an expanded event subprocess and open **Event
subprocess start**. Message and signal starts expose a reference ID and event
name; error and escalation starts expose a reference ID and code. An existing
declaration ID reuses that declaration. A new ID creates and binds a declaration
in one undo step. Editing its name or code affects all events using it. IDs
belonging to another BPMN element type are rejected.

Leave the error/escalation reference empty to catch any code. Error starts are
always interrupting; other supported starts expose the **Interrupting** checkbox.
Timer starts expose a duration/date/cycle selector and expression. Changing the
timer type removes the old expression, then accepts a value for the selected
type. Deployment validates the timer syntax and supported subprocess structure.

## Lifecycle history and notifications

History uses the owning instance ID and the subprocess container ID as `node_id`.
`EventSubprocessActivated` and `EventSubprocessCompleted` identify normal handler
execution. `EventSubprocessScopeInterrupted` records an interrupting handler's
effect on the containing scope; `EventSubprocessInterrupted` records cancellation
of an already active handler, including scope termination or failure. Cancelled
handlers do not receive a normal completion entry.

Each entry includes `activation_id`, `definition_key`, `generation`, and `reason`
in its context, plus the definition version. Pending entries are saved in the
instance's `lifecycle_outbox` before history append. Successful append acknowledges
only those entry IDs; failed writes retain them for the next drain or startup
reconciliation. Replays reuse stable IDs, and persistence adapters must deduplicate
history by entry ID. This also covers a crash after append but before acknowledgment.

The SSE event `event_subprocess_changed` carries `event_id`, `instance_id`,
`definition_key`, `subprocess_id`, `activation_id`, `phase`, and `reason`. Phases
are `activated`, `scope_interrupted`, `interrupted`, and `completed`. Delivery is
best effort, as for existing SSE events; reconnect/resync reloads persisted
history. The desktop history timeline refreshes only for its owning instance.

## Container execution listeners

Attach `camunda:executionListener` entries with `event="start"` or `event="end"`
and an inline `camunda:script scriptFormat="rhai"` to the event subprocess itself.
The existing modeler **Execution Scripts** group edits both callbacks when the
container is selected; body-node listeners remain separate.

A container start listener runs once per activation, before the start event's own
listeners and body tasks. It sees current process variables plus the activation
payload. Successful writes and the activation's started marker are saved in the
same instance snapshot before the body exposes work. Restoring that snapshot
skips the committed callback.

The normal container end listener runs once after every branch, loop iteration,
and child call belonging to that activation has finished. It sees the latest
process-variable state, including body output mappings. Cancelled, terminated,
and failed activations skip the normal end listener. A successful end commits its
writes, removes the activation/started marker, and queues completion history in
one instance-state update. Recovery also settles an activation whose body has
finished without a runnable token remaining.

Each callback chain commits process-variable writes only if all its scripts
succeed. A callback error fails the owning process scope and cancels its pending
work; partial writes from that callback chain are discarded. Listener history
uses the container ID, listener actor type, activation identity, and variable
diff. This is a snapshot-commit guarantee, not an exactly-once guarantee for
external effects. The scripting sandbox and its existing limits still apply.
