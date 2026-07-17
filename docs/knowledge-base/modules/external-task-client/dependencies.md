# external-task-client — Dependencies

## Outbound

| Dependency | Type | Purpose |
|-----------|------|---------|
| engine-server | HTTP REST | Service task API endpoints |
| pino | npm | Structured logging (optional, can disable) |
| pino-pretty | npm | Development log formatting |

**No other runtime dependencies.** The package uses native `fetch()` (Node ≥ 18).

## Inbound

| Caller | How | For |
|--------|-----|-----|
| External worker applications | npm import | Processing service tasks from BPMNinja engine |
| `example/simple-worker.ts` | Local import | Demo worker with 3 topic subscriptions |

## Architecture

```mermaid
graph TD
    subgraph "Worker Application"
        App["Worker code"]
        Client["ExternalTaskClient"]
        Service["TaskService<br>(per handler)"]
    end
    
    subgraph "engine-server :8081"
        Fetch["POST /api/service-task/fetchAndLock"]
        Complete["POST /api/service-task/{id}/complete"]
        Failure["POST /api/service-task/{id}/failure"]
        BpmnError["POST /api/service-task/{id}/bpmnError"]
        Extend["POST /api/service-task/{id}/extendLock"]
    end
    
    App --> Client
    Client --> Fetch
    Client -->|autoExtendLock| Extend
    App --> Service
    Service --> Complete
    Service --> Failure
    Service --> BpmnError
    Service --> Extend
```

## Dev Dependencies

| Dependency | Purpose |
|-----------|---------|
| typescript | Compiler |
| vitest | Test runner |
| tsx | Development script runner |
| @types/node | Node.js type definitions |
