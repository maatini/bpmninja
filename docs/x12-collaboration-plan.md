# X12 — Collaboration

## Parser foundation (2026-10-17)

`parse_bpmn_processes` returns all explicitly executable top-level processes in
document order. Shared message/error/signal/escalation declarations remain
available to each process. Each selected process uses existing semantic
validation, including conditional events, data mappings, and event subprocesses.
Any selected-process error fails the entire parse before deployment can begin.
Empty or duplicate process IDs are rejected, including non-executable pools,
because process-based lookup would otherwise be ambiguous.

`parse_bpmn_xml` retains its executable-first, first-process fallback behavior.
The collection API requires `isExecutable="true"`; absent or false execution
flags do not opt a pool into deployment. Current deployment endpoints still
select one process.

## Participant and route validation (2026-10-17)

`parse_bpmn_collaboration` returns executable process definitions together with
serializable collaboration, participant, and directed message-flow metadata.
Routes retain participant ID, process ID, element ID, message declaration ID,
and compiled message name. This API prepares metadata; it does not publish
definitions or perform message delivery. Both existing parser APIs are unchanged.

The initial routing contract is deliberately explicit:

- Each participant process reference must resolve to a top-level process. A
  process may have only one participant within a collaboration. Black-box pools
  (no process reference) and non-executable pools are retained as metadata.
- Internal flow endpoints must be top-level elements in distinct executable
  participant processes belonging to the same collaboration. Sources support
  send tasks and intermediate message throws. Targets support message starts,
  receive tasks (including instantiating receives), intermediate message catches,
  and message boundaries.
- Both endpoints must explicitly reference the same declared, named message.
  Equal message names do not equate different message declarations. An optional
  flow `messageRef` must match; when absent, the endpoints determine it. Conflicting
  nested/direct references and legacy fallback names are rejected for routing.
- All document IDs must be unique and nonempty. References must be unqualified
  local IDs because existing process compilation does not resolve QName prefixes.
- Participant-level flow endpoints, black-box/non-executable routes, nested scopes,
  and loop endpoints (including boundaries attached to loops) are rejected. Those
  require external transport, scope, or occurrence-aware delivery contracts.

Validation: all **99 parser tests** pass, including eight new collaboration
tests and a serialization roundtrip. Strict parser Clippy (`--all-targets --
-D warnings`), formatting, and diff checks pass.

## Remaining implementation

1. Participant and message-flow parsing is complete within the contract above.
   Wire this metadata into the collection deployment path below.
2. Add collection deployment with a result for every process. Validate all pools
   and perform per-process model checking before publishing definitions. Define
   persistence failure/rollback and startup restoration semantics for a batch;
   preserve the existing single-process response contract.
3. Persist routing metadata with its deployed definition version. Internal sends
   must target the intended pool and catch/start element; public message-name
   broadcast is insufficient. Specify correlation, payload mapping, multiple
   receivers, unavailable receivers, and version selection explicitly.
4. Use durable delivery records and receiver deduplication before enabling
   automatic sends. Recovery must cover sender commit before delivery, receiver
   commit before acknowledgment, repeated recovery, and archived receivers.
5. Update deployment clients, TypeSpec/OpenAPI, modeler workflows, examples,
   history/SSE, and user-facing limitations together.

## Acceptance

Parser coverage includes pool order/isolation, shared message references, an
invalid later executable pool, ignored non-executable pool semantics, missing
execution flags, duplicate/empty process IDs, and the legacy API fallback.

Subsequent gates: no partial batch on validation failure; deployment failure and
restart tests against real JetStream; version-pinned targeted routing without
unrelated-pool delivery; concurrent/retried delivery and crash-point recovery;
REST/client contract and modeler roundtrip tests. Run affected Rust/frontend
suites, formatting, strict Clippy, TypeSpec compilation and graphify update.

X12 remains open until deployment, routing and durable recovery pass acceptance.

Parser verification: `cargo test -p bpmn-parser` passed all 91 tests, including
four new collaboration tests. `cargo fmt --all --check` and `git diff --check`
passed. `cargo clippy -p bpmn-parser --all-targets -- -D warnings` passed.
`graphify update .` refreshed the code graph without LLM extraction;
the command used a workspace-local `XDG_CONFIG_HOME` because the sandbox cannot
read the global Git ignore file.
