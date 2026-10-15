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
repeated activation. Modeler property editing, REST/NATS end-to-end acceptance,
and additional scope edge cases remain on the [X9 plan](x9-event-subprocess-plan.md).
