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
flags do not opt a pool into deployment. The legacy deployment endpoint selects
one process; the collection endpoint deploys all explicitly executable pools.

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

## Collection deployment (2026-10-27)

`POST /api/deploy/collection` accepts the existing `{xml, name}` request and
returns `collection_id`, ordered `processes` (`process_id`, `definition_key`,
`version`, optional `model_check`) and validated `collaborations`. The legacy
`POST /api/deploy` response and selection behavior remain unchanged.

All explicitly executable pools are parsed before deployment. Each model check
uses an isolated process with shared declarations and namespace bindings;
non-executable pools and collaboration diagrams do not affect its findings.
Enforce mode rejects the whole batch if any pool has violations. Existing
skipped/inconclusive model-check policy is retained. Empty collections fail.

The engine serializes collection and single deployments/version allocation and
publishes every top-level/embedded definition under one registry write lock.
With persistence configured, a single `definition_collections` JetStream KV
record containing definitions, original XML and collaboration metadata is the
commit boundary. No per-pool persistence jobs or background retries are queued.
Validation or definitive commit failure publishes nothing. Original metadata
resolves process IDs against immutable definition keys/versions in that record.
The backend must support atomic collection commits; other backends fail closed.

Startup loads collections before restoring instances, preserving exact keys and
versions, including gaps and batches mixed with legacy deployments. Collection
read/corruption errors abort startup. It never reparses a batch as the first
process. Deleted members remain tombstoned inside the record, preserving audit
metadata and preventing resurrection. XML retrieval supports collection keys.

Limits: the serialized record must fit the broker's maximum message payload
(often 1 MiB), even when XML fits the HTTP 10 MiB limit. There is one engine
writer per store; version allocation is not coordinated across server replicas.
A lost request/commit acknowledgment can leave a committed batch absent from
the running engine until restart; inspect definitions after recovery before
retrying. Collection deployment has no client idempotency key. Automatic internal
message delivery is still disabled; routing and durable delivery remain below.

The recovery tests require `nats-server` on PATH and use isolated temporary
JetStream stores; CI installs the executable from its running NATS image.

Example (the XML must explicitly mark each deployed process executable):

```sh
curl -X POST http://localhost:8081/api/deploy/collection \
  -H 'Content-Type: application/json' \
  --data @docs/examples/x12-collection.json
```

Collection verification (2026-10-27): **549 tests passed** across the engine,
parser, server and in-memory persistence library suites plus `e2e_collection`,
`e2e_deploy`, `e2e_deploy_persist` and `e2e_model_check`. The six collection tests
cover order, XML/metadata, concurrent versions, later-pool validation and model
check rejection, oversized commit failure without publication, real JetStream
restart with interleaved legacy versions, repeated restoration, deletion and
corrupt-record startup failure. Strict Clippy passed for `engine-core`,
`bpmn-parser`, `persistence-nats` and `engine-server` with `--all-targets -- -D
warnings`. TypeSpec compilation, formatting, diff checks and AST-only
`graphify update .` also passed.

## Version-pinned routing preflight (2026-10-27)

Collaboration types now live in `engine-core` and remain re-exported from
`bpmn-parser`, preserving the parser API and serialized metadata. Collection
commit and restoration compile that metadata into `PinnedMessageRoute` values.
Both endpoints retain the collection, collaboration, flow and message identities
plus exact definition key/version, participant, process and element IDs. Invalid
stored routes fail restoration before the affected collection is published;
invalid routes supplied directly to the engine fail before persistence.

`WorkflowEngine::plan_internal_messages(source_definition, source_element,
business_key, variables)` performs read-only receiver preflight. This engine API
is deliberately separate from public `correlate_message` broadcasts. Its contract:

- The source must be a deployed send element. Only its own collection's explicit
  flows participate. Routes stay pinned when newer pool versions are deployed;
  an unrouted/legacy send produces an empty plan, never a name broadcast.
- A nonempty business key is mandatory and matched exactly against instance
  metadata. Variables are an explicit sender output snapshot, copied independently
  per flow. They are payload, not a substitute for correlation metadata. Target
  input mappings belong to the later delivery step.
- Each explicit outgoing flow yields one plan in document order. Multiple flows
  are explicit fan-out; multiple matching catches for one flow are ambiguous.
  The planner never selects an arbitrary receiver or broadcasts to all matches.
- Message starts select the exact pinned start element without creating an
  instance. Existing catches require the exact target definition, element,
  message name, business key and live token occurrence. Event-gateway alternatives
  and message boundaries are recognized; signal catches, suspended/completed,
  missing/archived and stale receivers are excluded.
- Each plan contains `Start`, one exact `Catch` identity (instance/catch/token),
  or an explicit unavailable reason: deleted definition, no matching catch, or
  ambiguous catches with their count. A deleted pool never falls back to a newer
  definition or another pool sharing the message name.

Plans are observations, not reservations or durable deliveries. Receiver state
can change after preflight. No subscriptions are consumed, instances started,
sender variables mutated, or messages automatically sent. The next increment
must persist the sender outbox and claim/revalidate exact receiver occurrences
with durable deduplication. Every sender execution needs a distinct durable
occurrence identity, including sequence-flow revisits of the same send node.
It must retain unavailable deliveries and avoid partially dispatching a fan-out
solely on a preflight result.

Routing verification (2026-10-27): **545 tests passed**: 410 engine-core,
99 parser, 29 server library and seven collection integration tests. The eight
new routing unit tests cover version pinning, exact business key/pool/element
selection, payload snapshots and serialization, explicit fan-out, unavailable
and ambiguous receivers, deleted definitions, stale recovered jobs, event-gateway
and boundary occurrences, and invalid-metadata failure before commit/restore.
A new isolated real JetStream test redeploys both pools, restarts the broker from
disk and confirms that the original sender still selects its original receiver
version. The final focused routing rerun also passed all eight tests. Strict
Clippy (`--all-targets -- -D warnings`) passed for engine-core, bpmn-parser,
persistence-nats and engine-server. TypeSpec compilation, formatting, diff checks
and AST-only graphify update passed. No REST schema change was needed for this
engine-only preflight API.

## Remaining implementation

1. Participant/message-flow parsing and collection deployment are implemented
   within the contracts above. Expand batch size/idempotency and multi-writer
   support separately if required.
2. Route compilation and receiver preflight are implemented above. Connect these
   version-pinned plans to durable delivery; actual targeted catch/start execution
   remains gated on the delivery records and receiver claims below.
3. Use durable delivery records and receiver deduplication before enabling
   automatic sends. Recovery must cover sender commit before delivery, receiver
   commit before acknowledgment, repeated recovery, and archived receivers.
4. Update deployment clients, modeler workflows, examples,
   history/SSE, and user-facing limitations for automatic routing together.
   Collection REST types, OpenAPI and a request example are already updated.

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
