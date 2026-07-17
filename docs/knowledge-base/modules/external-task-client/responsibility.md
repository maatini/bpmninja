# external-task-client — Responsibilities

## What It Owns

1. **@tag:worker-client** — TypeScript client for connecting to BPMNinja engine-server service task API.
2. **@tag:long-polling** — Fetch-and-lock with configurable `asyncResponseTimeout` for efficient task acquisition.
3. **@tag:multi-topic-subscription** — Parallel topic subscriptions with individual handlers via `client.subscribe(topic, handler)`.
4. **@tag:retry-backoff** — Global retry with exponential backoff on handler errors (1s → 2s → 4s → ..., capped at 30s).
5. **@tag:lock-extension** — Automatic lock extension (`autoExtendLock`) for long-running tasks.
6. **@tag:incident-creation** — After retries exhausted, creates incident on the process (`failure` with `retries: 0`).
7. **@tag:bpmn-error-throwing** — Triggering BPMN boundary error events from the worker via `service.bpmnError(code)`.
8. **@tag:graceful-shutdown** — Waits for in-flight handlers, cancels fetches via `AbortController`.

## Invariants

1. **No HTTP library dependency** — Uses native `fetch()` (Node ≥ 18).
2. **Strict TypeScript** — `strict: true`, ESM only, full type safety.
3. **No real timers in tests** — Tests use `vi.useFakeTimers()` and `vi.stubGlobal('fetch', ...)`.
4. **Pino for logging** — Structured logging, can be disabled via `logger: false`.
5. **Camunda API compatibility** — `TaskService` methods mirror Camunda's `camunda-external-task-client-js`.

## Key Files

| File | Lines | Purpose |
|------|-------|---------|
| `ExternalTaskClient.ts` | ~687 | Main client: constructor, subscribe, start, stop, internal poll loop |
| `TaskService.ts` | ~216 | Per-handler API: complete, failure, extendLock, bpmnError |
| `types.ts` | ~92 | Type definitions: ClientConfig, Task, SubscribeOptions, etc. |
| `index.ts` | ~14 | Public exports |
| `utils/retry.ts` | ~92 | `withRetry`, `calculateBackoff`, `sleep` |
| `utils/fetch.ts` | ~59 | HTTP fetch wrapper with JSON parsing and error handling |

## Key Entry Points

| Entry Point | Returns | Description |
|------------|---------|-------------|
| `new ExternalTaskClient(config)` | `ExternalTaskClient` | Create a new worker client |
| `client.subscribe(topic, handler, options?)` | `Subscription` | Register a topic handler |
| `client.start()` | `Promise<void>` | Start the poll loop |
| `client.stop()` | `Promise<void>` | Graceful shutdown |
| `service.complete(variables?)` | `Promise<void>` | Complete a task with optional output variables |
| `service.failure(msg, details?, retries?)` | `Promise<void>` | Mark a task as failed |
| `service.extendLock(ms)` | `Promise<void>` | Extend the lock duration |
| `service.bpmnError(errorCode)` | `Promise<void>` | Trigger a BPMN error event |
