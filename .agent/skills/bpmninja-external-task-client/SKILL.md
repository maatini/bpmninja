---
name: bpmninja-external-task-client
description: Skill for the @bpmninja/external-task-client TypeScript package covering poll-based task consumption, retry with exponential backoff, automatic lock extension, and graceful shutdown.
version: 1.0.0
tags: [typescript, esm, external-tasks, worker, fetch, pino, vitest]
requires: [node]
---

# BPMNinja External Task Client Skill

## When to Activate
Activate whenever you work on the `bpmn-ninja-external-task-client` package:
- `ExternalTaskClient` – Poll loop, subscriptions, handler dispatch, graceful shutdown
- `TaskService` – Per-task REST helpers (complete, failure, extendLock, bpmnError)
- `withRetry` / `sleep` / `calculateBackoff` – Exponential-backoff retry utility
- `types.ts` – All TypeScript interfaces and API contracts
- Tests and examples

## Scope & File Map

### Source
```
bpmn-ninja-external-task-client/src/
├── index.ts                 # Barrel export (public API surface)
├── ExternalTaskClient.ts    # Main client: poll loop, subscriptions, retry dispatch
├── TaskService.ts           # Per-task API helpers (scoped to one ExternalTask)
├── types.ts                 # All TS interfaces (ClientConfig, ExternalTask, Logger, …)
└── utils/
    └── retry.ts             # sleep(), calculateBackoff(), withRetry()
```

### Tests
```
bpmn-ninja-external-task-client/src/__tests__/
├── ExternalTaskClient.test.ts   # 35 tests – constructor, lifecycle, poll-loop, retry, lock-ext, shutdown
├── TaskService.test.ts          # 20 tests – complete, failure, extendLock, bpmnError
├── retry.test.ts                # 13 tests – sleep, calculateBackoff, withRetry
└── helpers/
    ├── fixtures.ts              # createMockTask() factory
    ├── mockFetch.ts             # createMockFetch(), mockFetchResponse(), setupGlobalFetchMock()
    └── mockLogger.ts            # createMockLogger() – pino-compatible vi.fn() logger
```

### Configuration & Example
```
bpmn-ninja-external-task-client/
├── package.json             # ESM, type: "module", vitest scripts
├── tsconfig.json            # strict: true, ES2022, NodeNext
├── vitest.config.ts         # Test runner config
└── example/
    └── simple-worker.ts     # Runnable demo with 3 topic subscriptions
```

## Technology Stack
- **TypeScript 5+** with `strict: true`, ESM (`"type": "module"`)
- **Native `fetch()`** (Node 18+) — no axios, no node-fetch
- **pino** for structured logging
- **vitest** for testing — all mocks via `vi.fn()` and `vi.stubGlobal('fetch', …)`

## Domain Rules & Patterns

### Architecture
1. **ExternalTaskClient** is the only public entry point. It owns the poll loop, subscription registry, and handler dispatch.
2. **TaskService** is scoped per task. It is instantiated inside `executeHandler()` and passed to the user's handler alongside the `ExternalTask`.
3. **withRetry** wraps each handler invocation with configurable exponential backoff. When retries are exhausted, it returns `{ success: false }` so the client can report an incident via `taskService.failure(…, retries: 0)`.

### API Contract (BPMNinja Engine, Rust/Axum)
| Endpoint | Method | Body |
|---|---|---|
| `/api/service-task/fetchAndLock` | POST | `{ workerId, maxTasks, topics: [{ topicName, lockDuration }], asyncResponseTimeout }` |
| `/api/service-task/:id/complete` | POST | `{ workerId, variables? }` |
| `/api/service-task/:id/failure` | POST | `{ workerId, retries?, errorMessage?, errorDetails? }` |
| `/api/service-task/:id/extendLock` | POST | `{ workerId, newDuration }` (seconds!) |
| `/api/service-task/:id/bpmnError` | POST | `{ workerId, errorCode }` |

### Critical Implementation Details
1. **lockDuration conversion**: Always `Math.ceil(ms / 1000)` — the engine expects seconds.
2. **ExternalTask fields use `snake_case`**: Directly from Rust serde (`instance_id`, `node_id`, `variables_snapshot`, `lock_expiration`).
3. **Logger can be `false`**: Creates a noop logger. Or a pino-compatible object.
4. **start()** throws if no subscriptions, is idempotent on double-call.
5. **subscribe()** throws on duplicate topic.
6. **Graceful shutdown**: `stop()` → `AbortController.abort()` → await `pollLoopPromise` → `Promise.allSettled(activeHandlers)`.

### Testing Conventions
1. **No real HTTP**: Mock `global.fetch` via `vi.stubGlobal('fetch', createMockFetch())`.
2. **No real timers**: Use `vi.useFakeTimers()` + `vi.advanceTimersByTimeAsync()`.
3. **Deterministic shutdown in tests**: Always use `stopClientAndFlush(client)` helper (stops + advances timers) to prevent hanging tests.
4. **Assertion strings**: Match exact log messages from the source (the logger receives a single string, not structured args).
5. **Test isolation**: Each test gets a fresh `fetchMock` in `beforeEach`.

## Verifikation

```bash
cd bpmn-ninja-external-task-client && npm run lint && npm run test
```

## Common Pitfalls
- Forgetting `Math.ceil(ms / 1000)` when sending `lockDuration` or `newDuration` to the engine
- Using `expect.any(Object)` in logger assertions — the logger receives a single string, not structured args
- Tests hanging because `client.stop()` waits on a timer that was never advanced (`stopClientAndFlush` fixes this)
- Importing concrete types from `engine-core` or `engine-server` — this package is engine-agnostic, it speaks REST only
- Adding HTTP dependencies (axios, node-fetch) — the package uses native `fetch()` exclusively
- Breaking the barrel export in `index.ts` — always re-export new public types/classes

