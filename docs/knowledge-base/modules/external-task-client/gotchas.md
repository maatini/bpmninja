# external-task-client — Gotchas

### ⚠️ Uses native fetch(), not an HTTP library

The client uses `globalThis.fetch()`. This requires Node ≥ 18. No axios, got, or node-fetch dependency. For test mocking, use `vi.stubGlobal('fetch', mockFetch)`.

### ⚠️ Tests use fake timers, not real timeouts

All 68 tests use `vi.useFakeTimers()` and mock `fetch`. No real HTTP requests or timers run during tests. This means timing-sensitive bugs may only appear in integration, not unit tests.

### ⚠️ Lock extension is automatic but optional

`autoExtendLock: true` makes the client call `extendLock` at 80% of `lockDuration`. If the handler takes longer than `lockDuration`, the lock expires and another worker may claim the task.

### ⚠️ Retry backoff is global per client, not per task

All failed handlers share the same retry count. If task A fails and task B fails immediately after, B's retry backoff builds on A's previous failures. This is intentional — prevents hammering the server.

### ⚠️ `bpmnError()` is not retried

BPMN errors are thrown immediately without retry. This is by design — they represent business exceptions (order too large, customer not found), not technical failures.

### ⚠️ `failure()` with `retries: 0` creates an incident

When `maxRetries` is exhausted (or explicitly set to 0), the task becomes an incident. It must be manually retried or resolved via the UI.

### ⚠️ Graceful shutdown waits for in-flight handlers

`client.stop()` waits for all currently executing handlers to finish (up to a timeout). New tasks from the poll loop are not dispatched during shutdown. The poll loop is cancelled via `AbortController`.

### ⚠️ Logger default is Pino

The client uses `pino` for structured logging. Set `logger: false` to suppress all logging. Custom loggers must implement the `{ info, warn, error, debug }` interface.

### ⚠️ ESM only

The package is `"type": "module"`. Cannot be used with `require()`. Import with `import { ExternalTaskClient } from "@bpmninja/external-task-client"`.

### ⚠️ Example worker path

The example is at `example/simple-worker.ts` and demonstrates three handlers: `send-email`, `validate-order`, and `flaky-task`. Run with `npm run example` (requires `tsx` and a running engine-server).
