# Data Flows

## 1. Deployment and Routing Flows

### Single-process deployment

```mermaid
sequenceDiagram
    participant Client
    participant Server as engine-server
    participant Parser as bpmn-parser
    participant Engine as WorkflowEngine
    participant Store as WorkflowPersistence
    Client->>Server: POST /api/deploy
    Server->>Parser: parse_bpmn_xml(xml)
    Parser-->>Server: one ProcessDefinition
    Server->>Server: model check
    Server->>Engine: deploy_definition(def)
    Engine->>Engine: register immutable definition/version
    Engine->>Store: save_definition (legacy retry policy)
    Engine-->>Server: definition key + version
    Server->>Store: save_bpmn_xml(key, xml)
    alt XML save fails
        Server->>Engine: delete_definition(key, cascade=true)
        Server-->>Client: persistence error
    else XML saved or no persistence
        Server->>Server: cache XML
        Server-->>Client: definition_key + version + optional model_check
    end
```

### Collection deployment

```mermaid
sequenceDiagram
    participant Client
    participant Server as engine-server
    participant Parser as bpmn-parser
    participant Engine as WorkflowEngine
    participant Store as WorkflowPersistence
    Client->>Server: POST /api/deploy/collection
    Server->>Parser: parse_bpmn_collaboration(xml)
    Parser-->>Server: executable processes + validated collaborations
    loop Each selected pool
        Server->>Server: isolated model check
    end
    Server->>Engine: deploy_collection(processes, xml, metadata)
    Engine->>Engine: assign keys/versions + compile routes
    opt Persistence configured
        Engine->>Store: save_definition_collection(one atomic record)
        Store-->>Engine: commit acknowledgment
    end
    Engine->>Engine: install routes + publish all definitions in one registry write
    Engine-->>Server: complete collection
    Server->>Server: cache collection XML for every definition
    Server-->>Client: collection_id + ordered processes + collaborations
```

Validation and definitive commit failures publish no pools. Collection persistence
uses `definition_collections`, does not enqueue background retries, and must fit
the broker payload limit. A lost acknowledgment may require restart recovery
before retrying. Version allocation assumes one engine writer per store.

### Read-only receiver preflight

`plan_internal_messages` reads routes indexed by the source definition key,
then selects only the pinned target definition, element and business key.
Message starts yield a start plan; catches yield exact instance/catch/token IDs.
Missing, deleted or ambiguous receivers produce explicit unavailable results.
Other pools/versions, signals and stale/inactive occurrences are excluded.

The supplied payload is a snapshot per flow; planning changes no sender or
receiver state. Plans are not reservations. Public message-name correlation
remains separate; durable delivery and automatic internal sends are pending.
See [the X12 contract](../../x12-collaboration-plan.md).

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

    Worker->>Server: POST /api/service-task/fetchAndLock { topics, workerId, maxTasks, asyncResponseTimeout }
    Server->>Engine: fetch_and_lock_service_tasks_long_poll(...)
    Note over Engine: Empty → wait on Notify (new / unlocked task)<br/>or 500 ms fallback tick (lock expiry)
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

    loop Until next due time, new timer (Notify) or TIMER_INTERVAL_MS max idle
        Scheduler->>Engine: wait_for_timer_work(max_idle)
        Scheduler->>Engine: process_timers()
        Engine-->>Engine: Walk due IDs from TimerDueIndex
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
    participant Store as NatsPersistence
    participant SC as StartupCoordinator
    participant Engine as WorkflowEngine
    Main->>Store: connect(nats_url)
    alt Persistence available
        Main->>SC: restore(engine, xml_cache)
        SC->>Store: list_definition_collections()
        Store-->>SC: complete committed records (or fatal error)
        SC->>Store: list_definitions()
        SC->>Engine: restore individual JSON definitions without renumbering
        Note over SC,Engine: XML parsing is a legacy fallback only
        loop Each committed collection
            SC->>Engine: restore_definition_collection(record)
            Engine->>Engine: validate/compile pinned routes
            Engine->>Engine: publish active definitions, skipping tombstones
            SC->>SC: populate XML cache from the collection record
        end
        SC->>SC: restore instances, tasks, timers and message catches
        SC->>Engine: reconcile_conditional_starts()
        SC-->>Main: RestoreStats
    else Connection failure and REQUIRE_NATS=true
        Main->>Main: exit
    else Connection failure and REQUIRE_NATS=false
        Main->>Main: ephemeral in-memory engine
    end
```

Collection read/validation errors abort restoration. A collection is never
restored by reparsing only its first process. Keys and version gaps survive
restarts, including batches interleaved with individual deployments. Deleted
members remain tombstoned; pinned routes never fall back to newer versions.

## 7. Health vs Readiness

```mermaid
sequenceDiagram
    participant LB as Load balancer / K8s
    participant API as engine-server

    LB->>API: GET /api/health
    API-->>LB: 200 (process up)

    LB->>API: GET /api/ready
    alt require_nats and no persistence
        API-->>LB: 503 Persistence required
    else persistence present but storage check fails
        API-->>LB: 503 NATS disconnected
    else
        API-->>LB: 200 Ready
    end
```

## 8. Persistence Retry (Fault-Tolerant)

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
