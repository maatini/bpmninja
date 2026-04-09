# @bpmninja/external-task-client

Production-ready **External Task Worker Client** for the [BPMNinja](https://github.com/maatini/bpmninja) BPMN 2.0 Workflow Engine.

Analogous to the [Camunda External Task Client](https://github.com/camunda/camunda-external-task-client-js), but adapted for BPMNinja's REST API.

## Features

- 🔄 **Long-Polling** — Efficient fetch-and-lock with configurable timeout
- 📌 **Multi-Topic Subscriptions** — Subscribe to multiple service task topics simultaneously
- 🔁 **Global Retry with Exponential Backoff** — Automatic retry on handler failures (1s → 2s → 4s → …)
- 🔒 **Automatic Lock Extension** — Prevents lock expiration during long-running tasks
- 🚨 **Incident Creation** — Reports unrecoverable failures as incidents to the engine
- ⚡ **BPMN Error Throwing** — Trigger boundary error events from worker code
- 🛑 **Graceful Shutdown** — Waits for in-flight handlers before stopping
- 📝 **Structured Logging** — Built-in pino logger with pretty-printing
- 🦾 **Strongly Typed** — Full TypeScript with strict mode, JSDoc, and declaration maps

## Installation

```bash
npm install @bpmninja/external-task-client
```

Or install from the monorepo workspace:

```bash
cd bpmn-ninja-external-task-client
npm install
npm run build
```

## Quick Start

```typescript
import { ExternalTaskClient } from "@bpmninja/external-task-client";

const client = new ExternalTaskClient({
  baseUrl: "http://localhost:8080",
  workerId: "my-worker",
  maxRetries: 3,
});

client.subscribe("send-email", async (task, service) => {
  const { recipient, subject } = task.variables_snapshot as {
    recipient: string;
    subject: string;
  };

  await sendEmail(recipient, subject);

  await service.complete({
    emailSent: true,
    sentAt: new Date().toISOString(),
  });
});

client.start();
```

## Configuration

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `baseUrl` | `string` | `http://localhost:8080` | BPMNinja engine URL |
| `workerId` | `string` | Auto-generated | Unique worker identifier |
| `lockDuration` | `number` | `30000` | Lock duration in ms |
| `maxTasks` | `number` | `10` | Max tasks fetched per poll |
| `asyncResponseTimeout` | `number` | `10000` | Long-poll timeout in ms |
| `pollingInterval` | `number` | `300` | Interval between polls in ms |
| `maxRetries` | `number` | `3` | Global handler retry count |
| `baseRetryDelay` | `number` | `1000` | Base delay for exponential backoff in ms |
| `autoExtendLock` | `boolean` | `false` | Auto-extend locks for long tasks |
| `autoExtendLockInterval` | `number` | `10000` | Lock extension interval in ms |
| `logger` | `Logger \| false` | pino (pretty) | Custom logger or `false` to disable |

## API Reference

### `ExternalTaskClient`

#### `constructor(config?: ClientConfig)`

Creates a new client instance with the given configuration.

#### `subscribe(topic: string, handler: TaskHandler, options?: SubscriptionOptions): this`

Registers a handler for a specific topic. The handler receives the task and a `TaskService` instance.

```typescript
client.subscribe("my-topic", async (task, service) => {
  // Process task...
  await service.complete({ result: "done" });
}, {
  lockDuration: 60_000,  // Override lock duration for this topic
  maxRetries: 5,         // Override retries for this topic
});
```

#### `unsubscribe(topic: string): boolean`

Removes a topic subscription.

#### `start(): void`

Starts the polling loop.

#### `stop(): Promise<void>`

Gracefully stops the client, waiting for in-flight handlers.

### `TaskService` (passed to handler)

#### `complete(variables?: Record<string, unknown>): Promise<void>`

Marks the task as completed successfully with optional output variables.

#### `failure(errorMessage: string, errorDetails?: string, retries?: number): Promise<void>`

Reports a task failure. When `retries` is `0`, the engine creates an incident.

#### `extendLock(additionalDurationMs: number): Promise<void>`

Extends the lock on the current task.

#### `bpmnError(errorCode: string): Promise<void>`

Throws a BPMN error that can be caught by a boundary error event.

## Retry Mechanism

The client wraps every handler with a configurable retry mechanism:

```
Attempt 1 → Handler runs
  ❌ Fails → Wait 1s (baseRetryDelay * 2^0)
Attempt 2 → Handler runs
  ❌ Fails → Wait 2s (baseRetryDelay * 2^1)
Attempt 3 → Handler runs
  ❌ Fails → Wait 4s (baseRetryDelay * 2^2)
Attempt 4 → Handler runs
  ❌ Fails → All retries exhausted!
             → POST /api/service-task/:id/failure { retries: 0 }
             → Engine creates INCIDENT 🚨
```

The retry mechanism is **transparent** to the handler — your handler code stays simple, and the client handles transient failures automatically.

### Per-topic override:

```typescript
client.subscribe("critical-task", handler, {
  maxRetries: 10,   // More retries for this critical topic
});
```

## REST API Endpoints

The client communicates with these BPMNinja endpoints:

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/service-task/fetchAndLock` | Fetch and lock available tasks |
| `POST` | `/api/service-task/:id/complete` | Complete a task with variables |
| `POST` | `/api/service-task/:id/failure` | Report a task failure |
| `POST` | `/api/service-task/:id/extendLock` | Extend the task lock |
| `POST` | `/api/service-task/:id/bpmnError` | Throw a BPMN error |

## Graceful Shutdown

```typescript
async function shutdown(signal: string) {
  console.log(`Received ${signal}, shutting down...`);
  await client.stop();  // Waits for in-flight handlers
  process.exit(0);
}

process.on("SIGINT", () => shutdown("SIGINT"));
process.on("SIGTERM", () => shutdown("SIGTERM"));
```

## Example

Run the included example worker:

```bash
cd bpmn-ninja-external-task-client
npx tsx example/simple-worker.ts
```

See [`example/simple-worker.ts`](./example/simple-worker.ts) for a complete working example with:
- Multi-topic subscriptions
- BPMN error handling
- Retry demonstration
- Graceful shutdown

## Project Structure

```
bpmn-ninja-external-task-client/
├── src/
│   ├── index.ts                 # Public barrel export
│   ├── ExternalTaskClient.ts    # Main client (poll loop, subscriptions)
│   ├── TaskService.ts           # Per-task API helper (complete, fail, etc.)
│   ├── types.ts                 # All TypeScript interfaces
│   └── utils/
│       └── retry.ts             # Exponential backoff retry utility
├── example/
│   └── simple-worker.ts         # Usage example
├── package.json
├── tsconfig.json
└── README.md
```

## License

MIT
