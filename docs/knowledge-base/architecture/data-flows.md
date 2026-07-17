# Data Flows

## 1. Deployment Flow

```mermaid
sequenceDiagram
    participant Client as Client (UI/API)
    participant Server as engine-server
    participant Parser as bpmn-parser
    participant Engine as engine-core
    participant NATS as persistence-nats
    participant Events as EngineEvent broadcast

    Client->>Server: POST /api/deploy (BPMN XML)
    Server->>Parser: parse_bpmn_xml(xml)
    Parser-->>Server: ProcessDefinition
    Server->>Engine: deploy_definition(def)
    Engine-->>Engine: Register in DefinitionRegistry
    Engine->>NATS: save_definition + save_bpmn_xml
    NATS-->>Engine: OK
    Engine->>Events: DefinitionChanged
    Server-->>Client: { definition_key, version }
```

## 2. Instance Execution Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Server as engine-server
    participant Engine as WorkflowEngine
    participant Exec as Executor
    participant Script as Script Runner
    participant NATS as persistence-nats
    participant Events as Event broadcast

    Client->>Server: POST /api/start
    Server->>Engine: start_instance(definition_key, variables)
    Engine-->>Engine: Create ProcessInstance + Token
    Engine->>NATS: save_instance
    Engine->>Exec: run_instance_batch(instance_id, token)
    loop Token execution
        Exec->>Exec: execute_step(instance, token)
        alt UserTask/ServiceTask/Timer/Message
            Exec-->>Engine: WaitFor* — store token, pause
            Engine->>NATS: save_task/timer/message
            Engine->>Events: InstanceChanged / TaskChanged
        else Gateway (XOR/AND/OR)
            Exec->>Exec: Evaluate conditions, fork/join tokens
            Engine->>Events: InstanceChanged
        else ScriptTask
            Exec->>Script: execute_listener_script()
            Script-->>Exec: Continue(token)
        else EndEvent
            Exec->>Engine: Complete — mark instance completed
            Engine->>NATS: archive instance
            Engine->>Events: InstanceChanged
        end
    end
    Server-->>Client: { instance_id, state }
```

## 3. Service Task (Fetch-and-Lock)

```mermaid
sequenceDiagram
    participant Worker as External Worker
    participant Server as engine-server
    participant Engine as WorkflowEngine

    Worker->>Server: POST /api/service-task/fetchAndLock { topics, workerId, maxTasks }
    Server->>Engine: Fetch unlocked service tasks matching topics
    Engine-->>Server: List of matching PendingServiceTasks
    Server->>Engine: Lock each task (workerId, lockExpiration)
    Server-->>Worker: [{ id, topic, variables_snapshot, ... }]
    
    Note over Worker: Execute business logic
    
    alt Success
        Worker->>Server: POST /api/service-task/{id}/complete { variables }
        Server->>Engine: complete_service_task(id, variables)
        Engine-->>Engine: Resume instance execution
    else Failure
        Worker->>Server: POST /api/service-task/{id}/failure { message, details, retries }
        Server->>Engine: fail_service_task(id, ...)
        alt retries > 0
            Engine-->>Engine: Decrement retries, unlock task
        else retries = 0
            Engine-->>Engine: Create incident
        end
    else BPMN Error
        Worker->>Server: POST /api/service-task/{id}/bpmnError { errorCode }
        Server->>Engine: bpmn_error(id, errorCode)
        Engine-->>Engine: Route to BoundaryErrorEvent handler
    end
```

## 4. Timer Processing

```mermaid
sequenceDiagram
    participant Scheduler as Timer Scheduler (bg task)
    participant Engine as WorkflowEngine
    participant NATS as persistence-nats

    loop Every TIMER_INTERVAL_MS (default 1000ms)
        Scheduler->>Engine: process_timers()
        Engine-->>Engine: Scan pending_timers DashMap for expired
        alt Found expired timers
            Engine-->>Engine: Resume instance with stored token
            Engine->>NATS: Delete timer from KV
            alt Repeating timer
                Engine->>Engine: Compute next expiry, create new PendingTimer
                Engine->>NATS: Save new timer
            end
        end
    end
```

## 5. SSE Event Push

```mermaid
sequenceDiagram
    participant Engine as WorkflowEngine
    participant Channel as tokio::broadcast
    participant SSE as SSE Handler
    participant UI as Desktop UI

    Engine->>Channel: emit_event(EngineEvent::InstanceChanged)
    Channel->>SSE: receive (subscribed)
    SSE->>UI: SSE: data: {"type":"instance_changed"}
    UI->>UI: Re-fetch instance list / detail from REST
```

## 6. Startup Restore Flow

```mermaid
sequenceDiagram
    participant Main as main()
    participant NATS as persistence-nats
    participant SC as StartupCoordinator
    participant Engine as WorkflowEngine
    participant Parser as bpmn-parser

    Main->>NATS: NatsPersistence::connect(nats_url)
    NATS-->>Main: NatsPersistence instance
    Main->>SC: StartupCoordinator::new(nats)
    SC->>SC: restore_definitions(engine, xml_cache)
    loop Each definition key
        SC->>NATS: load_bpmn_xml(key)
        SC->>Parser: parse_bpmn_xml(xml)
        Parser-->>SC: ProcessDefinition
        SC->>Engine: deploy_definition(def)
    end
    SC->>SC: restore_instances(engine)
    SC->>SC: restore_user_tasks(engine)
    SC->>SC: restore_service_tasks(engine)
    SC->>SC: restore_timers(engine)
    SC->>SC: restore_message_catches(engine)
    SC-->>Main: RestoreStats (counts)
```

## 7. Persistence Retry (Fault-Tolerant)

```mermaid
sequenceDiagram
    participant Engine as WorkflowEngine
    participant Inline as Inline Retry
    participant Queue as RetryQueue (mpsc channel)
    participant Worker as RetryWorker (bg task)
    participant NATS as NATS JetStream

    Engine->>Inline: save_instance(instance)
    Inline->>NATS: save_instance (attempt 1)
    alt Success
        NATS-->>Inline: OK
    else Failure
        Inline->>NATS: save_instance (attempt 2, 50ms backoff)
        alt Success
            NATS-->>Inline: OK
        else Still failing
            Inline->>Queue: PersistJob::SaveInstance(instance_id)
            Queue->>Worker: Dequeue job
            Worker->>Worker: Wait (exponential backoff: 1s → 2s → ... → 60s)
            Worker->>Engine: Re-read instance from InstanceStore
            Worker->>NATS: save_instance (retry up to 50 times)
        end
    end
```
