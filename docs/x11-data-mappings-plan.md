# X11 — Data defaults and activity mappings

Status: proposed for approval, 2026-10-14. Next item after X4/X5 in
`improvements.md`. This plan does not mark X11 implemented.

## Scope and behavior

Implement process data declarations and activity input/output mappings without
requiring the subprocess runtime-scope refactor (X6).

- Process properties and data objects declare variables. Declaration alone must
  not invent a value; supported explicit defaults initialize new instances only.
  Caller-supplied start variables override defaults. Defaults are not reapplied
  during restore.
- Parse `ioSpecification` and input/output associations into validated mappings.
  Resolve references within their process/activity scope; reject dangling,
  ambiguous, or unsupported executable associations at deploy.
- Support Camunda `inputOutput` literal, variable-reference, list, and map
  parameters. Publish the supported expression syntax explicitly. Reject
  unsupported script languages and expressions rather than treating them as
  successful mappings. Reuse the existing sandbox for any supported scripts.
- Support Call Activity `camunda:in` / `camunda:out` mappings alongside activity
  parameters. Explicit mappings determine transferred variables; preserve current
  behavior for definitions that contain no mappings.
- Apply inputs once per activity activation, before start listeners and task
  execution; apply outputs on successful completion before end listeners and
  outgoing conditions. Verify this ordering against existing listener behavior
  before introducing the hooks and document any necessary adjustment.
- Keep activity-local inputs separate from process variables. External workers
  receive the effective activity variables. Persist enough activation state to
  resume without applying inputs twice. Failure/retry preserves that state;
  cancellation discards it without applying success outputs.
- Evaluate all mappings in a phase against one source snapshot, then publish the
  resulting writes together. Mapping order must not affect results. Mapping errors
  must use the existing error/incident path and prevent successful advancement.
- Each multi-instance activation owns its mapping state; outputs integrate with
  the existing loop output collection. Parallel activations must not overwrite
  each other's local inputs.

Example: a process variable `customerId` maps to task-local `id`; a worker returns
`result`; the output mapping writes `receipt` to the process. `id` stays local,
and `receipt` becomes visible before the next gateway evaluates its condition.

## Implementation sequence

1. Add serializable declarations and mapping types in
   `engine-core/src/domain/definition.rs`, with serde defaults for old persisted
   definitions. Define supported value/expression forms and validation.
2. Extend `bpmn-parser/src/models.rs` and `parser.rs`, including flattened
   subprocess reference handling. Add parser fixtures for standard and Camunda
   forms and explicit rejection of unsupported constructs.
3. Add a shared mapping evaluator and activity activation state. Integrate it
   with executor entry, task completion, `engine/process_start.rs` child-process
   creation/completion, and loop activation/cancellation. Audit all task kinds,
   boundary paths, retries, and recovery paths for exactly-once input application.
4. Persist activation state with backward-compatible defaults. Cover definition
   and instance roundtrips and restart while a mapped external task is pending.
5. Add REST end-to-end fixtures for defaults, task mappings, Call Activity
   transfer, failures, and restart. Keep worker DTOs consistent with effective
   activity variables and existing Camunda facade behavior.
6. Expose supported mappings in the desktop properties panel, retaining original
   XML on export. Add frontend coverage for editing and roundtrip preservation.
7. Update the element matrix, mapping contract, and X11 progress in
   `improvements.md`; run `graphify update .` after code changes.

## Acceptance and verification

- Parser tests distinguish supported mappings from unsupported deploy input.
- Engine tests cover defaults versus supplied variables; input/output ordering;
  local-variable isolation; mapping errors; worker retry; interrupting boundary
  cancellation; parallel and sequential loops; child-process failure/completion.
- Persistence tests load old definitions/instances and roundtrip new activation
  state. Restart tests prove inputs are not repeated and outputs are not lost.
- REST tests execute models rather than checking deployment alone.
- Modeler tests preserve supported mapping attributes through edit/export.
- Run formatting, affected Rust crate tests, required repository checks, and
  affected frontend tests. Use NATS integration tests for recovery coverage where
  the environment supports them; report any checks that cannot run.

## Boundaries

Do not claim full FEEL/JUEL, arbitrary XML Schema typing, durable external data
stores, or subprocess runtime scopes. Unsupported executable data constructs
must produce clear deployment errors. X11 stays partial until all acceptance
criteria, including modeler support, pass.

The first implementation step must settle the concrete expression/default
encoding from existing parser conventions and fixtures, then document it in the
mapping contract; BPMN data declarations do not inherently specify JSON defaults.
