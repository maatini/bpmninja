# X2c — Explicit top-level conditional starts

Status: completed and verified after user authorization, 2026-10-16.
Runtime contract: [conditional starts](conditional-starts.md).

## Scope and observable behavior

Support one top-level conditional start per executable process definition, using
the existing strict inline condition grammar. Reject mixed/multiple top-level
starts, filters, expression languages/resources, and conditional end/throw events.
Keep process-level conditional event-subprocess starts and intermediate/boundary
contracts unchanged. Call/loop/container boundaries and X6 scopes remain separate.

Evaluation is an explicit command, not a process-global variable subscription:

`POST /api/conditional-starts/evaluate`

The request contains exactly one definition selector (`definition_key` or
`process_id` for latest), a required UUID `request_id`, an optional business key,
and a variables map. Defaults and supplied process variables form the initial
condition context using the same precedence as normal starts. Task-local values
are unavailable. False creates no instance; true starts exactly one instance at
the selected conditional start and follows its ordinary outgoing flow.

Return `request_id`, the pinned `definition_key`, `matched`, and `instance_id`
only when matched. Repeating the same request returns the saved result, including
a saved false result. Reusing its ID with another selector, variables, or business
key returns 409. To evaluate changed input or deliberately trigger another
instance, the caller supplies a new request ID. No cross-request false/true latch
is implied. Request IDs are global to the configured persistence store.

Resolve latest only on the first accepted request. Retries remain bound to that
version after redeploy, even if the condition or latest start type changes.
Resolve the selected latest definition itself; do not fall back to an older
conditional version. Ordinary manual start APIs reject definitions whose only
start is conditional, directing callers to evaluation. Existing instance variable
updates, startup, timers, signals, and deployment never implicitly launch these
processes. No Camunda evaluation API is included in this increment.

## Durable evaluation and recovery

Reserve a receipt before execution, containing the request ID, normalized input
identity, selector, pinned definition key, condition outcome, and a preallocated
instance ID for true results. Persist false outcomes as terminal receipts too.
Compare input structurally/canonically, independent of JSON map ordering; do not
use an unstable standard-library hash. Validate the complete request before
creating a receipt. Document receipt retention; automatic eviction is excluded.

Add an internal idempotent start path taking that reserved instance ID. Persist
its initial instance/token snapshot before running work. If that instance already
exists in active or archived storage, return/reconcile it instead of starting
again. Startup finishes reserved true receipts lacking an instance snapshot and
reconciles saved instances lacking published first work. It must never reevaluate
a saved outcome against a newer definition or new variable values.

Receipt reservation, instance creation, and completion markers need an explicit
ordering and crash tests. Implement the necessary persistence adapter methods for
memory and NATS, with atomic receipt creation and conflict detection; do not hide
new records inside an unrelated bucket/document type. Use existing execution
fences for callbacks and saved continuations. Exactly-once claims concern committed
engine state, not arbitrary external side effects. Preserve the current single
engine writer limitation (C6); this feature does not introduce HA ownership.

## Implementation outline

1. Add ConditionalStartEvent and selected-process XML validation in both model
   and structural checks. Include it in start-node validation, strict condition
   compilation, definition capability detection, and conditional runtime context
   initialization. Ordinary start dispatch consumes an already accepted receipt;
   it does not reevaluate after creating the instance.
2. Add persisted receipt types and adapter operations with serde/version handling,
   input validation/normalization, global request-ID conflict behavior, and stable
   version binding. Audit codec/journal formats and storage initialization.
3. Implement the evaluation command and idempotent reserved-ID initializer; enforce
   manual-start rejection across key/latest APIs and internal launch paths that
   could otherwise bypass evaluation. Retain normal starts for other start kinds.
4. Reconcile unfinished receipts during startup after definitions and instance
   stores load, before accepting new evaluation traffic. Preserve archived receipt
   results and prevent startup recovery from resurrecting completed instances.
5. Add the native REST endpoint and synchronized TypeSpec/OpenAPI contracts.
   History/SSE identify the request and definition without copying request values
   into lifecycle metadata. False evaluations have receipts but no instance events.
6. Extend modeler Condition editing to a top-level conditional start, with no
   interrupting checkbox. Retain undo and XML roundtrip behavior. Extend analyzer
   skipping to the new conditional-start semantics while structural checks run.
7. Document evaluation, retries, version selection, receipt retention, and remaining
   X2 limits; update the backlog only after acceptance and refresh graphify.

## Acceptance gates

- Parser/programmatic definitions: valid simple conditions and defaults, selected
  pool isolation, missing/empty/unsupported expressions, mixed/multiple starts,
  incoming start flows, and unsupported event kinds.
- Engine: false creates nothing; initial true creates one instance with effective
  input/defaults and business key; normal variable updates and redeploy/startup
  create nothing implicitly. Direct key/latest/manual launch cannot bypass the
  condition. Separate request IDs may deliberately create separate instances.
- Idempotency: concurrent identical requests create one instance; equivalent map
  order is accepted; modified input/selector/business key returns conflict; saved
  false remains false on retry; latest remains pinned after a newer deployment.
- Recovery: reserved receipt before instance, initial snapshot before first task,
  completed/archived instance before receipt finalization, original definition after
  redeploy, and restart repetition without duplicate committed listeners or tasks.
  Run real JetStream reconnect tests for these cut points.
- REST: request validation, 409 conflicts, true/false responses, manual-start
  rejection, pending first work, and original result after completion/redeploy.
- Browser: top-level Condition editing, no interruption control, undo/export/import,
  and existing X9/X2a/X2b editor regression coverage.
- Verify affected core/parser/NATS/server/REST suites; frontend build/lint/event
  tests and focused browser tests; TypeSpec compile, formatting, strict workspace
  Clippy, diff check, and graphify update. Keep Cargo commands sequential. Record
  actual passing totals and any limitations in this plan before closure.

X2c completes the planned top-level-start subset, not full BPMN conditional
coverage: call/loop/container boundary attachments and nested runtime scopes
remain explicitly unsupported.

## Umsetzung und Abnahme (2026-10-16)

Implementiert sind `ConditionalStartEvent`, die ausgewählte Prozess-/XML-
Validierung, strikte Condition-Kompilierung, explizite Evaluation und die
Ablehnung manueller Umgehungen. Memory und NATS reservieren Request-Quittungen
atomar; True und False bleiben an Ergebnis und ursprüngliche Definition
gebunden. Initialsnapshot und Start-Continuation besitzen explizite
Persistenzbarrieren. Die Größenprüfung des vollständig vorbereiteten Snapshots
läuft vor Reservierung; gespeicherte aktive/archivierte Instanzen werden per ID
abgeglichen. Startup-Recovery schlägt bei kritischen Lesefehlern fehl.

REST, TypeSpec und beide OpenAPI-Dateien sind synchronisiert. Modeler-Condition,
Undo und XML-Roundtrip funktionieren ohne Interrupting-Feld am Top-Level-Start.
Lifecycle-History und SSE enthalten Request-/Definitions-IDs ohne Kopie der
Request-Variablen. Vertrag und Grenzen: [Conditional-Starts](conditional-starts.md).

Bestanden:

- `cargo test --workspace -j 2 -- --test-threads=1` mit verpflichtendem realem
  JetStream (`REQUIRE_NATS_TESTS=1`): **636 Tests**, keine Fehler oder ignorierten
  Tests. Enthalten: 402 Core, 5 Core-Integration, 87 Parser, 29 Server-Unit,
  91 Server-E2E (davon 8 X2/X9-REST), 2 Memory und 20 NATS.
- NATS prüft atomare Reservierung über getrennte Verbindungen, reservierte
  Quittung ohne Instanz, Initialsnapshot mit geleerter Ready-Queue, gespeicherte
  Continuation ohne erste Aufgabe, ursprüngliche Definition nach Redeploy,
  False-Retry und archivierte Instanz vor Quittierungsabschluss. Wiederholtes
  Recovery erzeugt keine zusätzlichen Tasks oder committed Start-/End-Listener.
- Zehn fokussierte X2/X9-Browsertests mit installiertem Chrome; die von dieser
  Playwright-Version erwartete gebündelte Chromium-Version fehlt lokal.
- Neun Frontend-Eventtests, Frontend-Build und Lint (0 Fehler, 277 Warnungen).
- TypeSpec-Kompilierung, synchronisierte OpenAPI-Dateien, `cargo fmt --all --check`
  und `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

Zusätzlich bestanden: `cargo test -p persistence-nats --all-features --lib x2c --
--test-threads=1` gegen denselben realen JetStream-Testserver. Der gezielte
Recovery-Test besteht mit kompakten Snapshots, separaten Audit-Daten und
Delta-Persistenz (1 Test, 19 ausgefiltert). Graphify wurde mit
`graphify update .` ohne LLM aktualisiert; die Diff-Prüfung ist erfolgreich. X2 bleibt wegen Call-/Loop-/Container-Boundaries
und verschachtelter Runtime-Scopes ausdrücklich teilweise offen (X6).
