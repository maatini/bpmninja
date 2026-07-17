# external-task-client — Interfaces

## Public API

```typescript
// Main client
class ExternalTaskClient {
  constructor(config: ClientConfig);
  subscribe(topic: string, handler: TaskHandler, options?: SubscribeOptions): Subscription;
  start(): Promise<void>;
  stop(): Promise<void>;
}

// Configuration
interface ClientConfig {
  baseUrl: string;                    // e.g. "http://localhost:8081"
  workerId: string;
  maxTasks?: number;                  // default: 10
  asyncResponseTimeout?: number;      // default: 10000 (ms, long polling)
  interval?: number;                  // default: 100 (ms, poll interval)
  maxRetries?: number;                // default: 3
  autoExtendLock?: boolean;           // default: false
  lockDuration?: number;              // default: 20000 (ms)
  logger?: Logger | false;            // false = disable, default: pino
}

// Task handler
type TaskHandler = (task: Task, service: TaskService) => Promise<void>;

// Task object (from fetchAndLock response)
interface Task {
  id: string;
  instanceId: string;
  definitionKey: string;
  nodeId: string;
  topic: string;
  tokenId: string;
  businessKey?: string;
  variables_snapshot: Record<string, any>;
  created_at: string;
  workerId?: string;
  lockExpiration?: string;
  retries: number;
  errorMessage?: string;
  errorDetails?: string;
}

// Per-handler service
interface TaskService {
  complete(variables?: Record<string, any>): Promise<void>;
  failure(message: string, details?: string, retries?: number): Promise<void>;
  extendLock(newDurationMs: number): Promise<void>;
  bpmnError(errorCode: string): Promise<void>;
}

// Subscription
interface Subscription {
  unsubscribe(): void;
}

// Subscribe options
interface SubscribeOptions {
  maxTasks?: number;                  // Override per-topic
  asyncResponseTimeout?: number;      // Override per-topic
  lockDuration?: number;              // Override per-topic
}
```

## HTTP Endpoints Called

| Method | Endpoint | Called By | When |
|--------|----------|-----------|------|
| `POST` | `/api/service-task/fetchAndLock` | Client poll loop | Every `interval` ms |
| `POST` | `/api/service-task/{id}/complete` | `service.complete()` | Task succeeded |
| `POST` | `/api/service-task/{id}/failure` | `service.failure()` | Task failed (retry or incident) |
| `POST` | `/api/service-task/{id}/extendLock` | `service.extendLock()` or auto-extend | Lock about to expire |
| `POST` | `/api/service-task/{id}/bpmnError` | `service.bpmnError()` | BPMN error boundary trigger |

## Usage Pattern

```typescript
import { ExternalTaskClient } from "@bpmninja/external-task-client";

const client = new ExternalTaskClient({
  baseUrl: "http://localhost:8081",
  workerId: "my-worker-01",
  maxRetries: 3,
  autoExtendLock: true,
});

client.subscribe("send-email", async (task, service) => {
  await sendEmail(task.variables_snapshot);
  await service.complete({ emailSent: true });
});

client.subscribe("validate-order", async (task, service) => {
  if (task.variables_snapshot.amount > 10000) {
    await service.bpmnError("ORDER_LIMIT_EXCEEDED");
    return;
  }
  await service.complete({ orderValid: true });
});

client.start();

// Graceful shutdown
process.on("SIGINT", async () => {
  await client.stop();
  process.exit(0);
});
```
