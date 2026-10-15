# Event subprocess runtime (X9a, in progress)

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

Nested event subprocesses, conditional/compensation starts, handler-level data
declarations/mappings, and handler-container execution listeners are rejected.
Attach execution listeners to body nodes for the implemented lifecycle. Legacy
programmatic `ScopeEventListener` definitions and persisted nil-token listener
subscriptions require migration to explicit `EventSubProcess` metadata.

The external model analyzer reports event subprocess semantics as `skipped`;
parser structural validation still runs. It does not certify interruption or
repeated activation. Remaining acceptance work is tracked in the [X9 plan](x9-event-subprocess-plan.md).

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
