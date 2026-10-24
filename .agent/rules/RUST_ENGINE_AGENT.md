---
trigger: file_match
file_patterns: ["engine-core/**"]
---

# Engine-Core-Regeln und BPMN-Spezifikation

## Rolle
Reine State Machine: Token-Fortschritt, `BpmnElement`-Dispatch, Gateway-Routing, Condition-Auswertung,
Script-Ausführung sowie Timer-, Message-, Error-, Escalation- und Compensation-Events.
Kein Netzwerk, kein Dateisystem; alle I/O läuft über `WorkflowPersistence`.

## Essential Rules
- Kein NATS-, HTTP- oder Dateisystem-Code in `engine-core`.
- Timer über `tokio::time`; externe Effekte ausschließlich über Traits.
- Concurrency über DashMap im Instance-Store; kein globaler `RwLock`.
- `ProcessDefinition` ist nach dem Bau unveränderlich und wird als `Arc<ProcessDefinition>` geteilt.
- `EngineError` (`domain/error.rs`) ist der einzige Fehlertyp; neue Fehlerfälle dort ergänzen.
- Script-Fehler (Rhai, Listener, ScriptTask) dürfen Instanzen nie in einem unklaren Zustand lassen:
  deterministischer Übergang (`CompletedWithError` oder expliziter Incident) und persistieren.
- Unbehandelte Prozessfehler erzeugen vor dem Rückgabewert einen Audit- und History-Eintrag.

## Unterstützte BPMN-Elemente (29)
- **StartEvent**, **TimerStartEvent(TimerDefinition)**, **EndEvent**, **TerminateEndEvent**
- **ServiceTask { topic, multi_instance }** — Camunda-External Task mit `PendingServiceTask`
- **UserTask(String)** — `PendingUserTask`, wartet auf `complete_user_task()`
- **ScriptTask { script, multi_instance }** — Rhai inline, danach automatisch weiter
- **SendTask { message_name, multi_instance }** — publiziert Nachricht und läuft weiter
- **ExclusiveGateway { default }** — XOR: erste wahre Bedingung, dann unbedingter Nicht-Default-Flow, dann Default
- **InclusiveGateway** — OR-Split mit `ContinueMultiple`; Join über `expected_count` der genommenen Zweige
- **ParallelGateway** — AND-Split/Join über alle eingehenden Tokens
- **EventBasedGateway** — wartet auf genau ein Ziel (Message- oder Timer-Catch)
- **ComplexGateway { join_condition, default }** — früher Join bei erfüllter Bedingung
- **TimerCatchEvent**, **BoundaryTimerEvent { attached_to, timer, cancel_activity }**
- **MessageStartEvent**, **MessageCatchEvent { message_name }**, **BoundaryMessageEvent**
- **BoundaryErrorEvent { attached_to, error_code }**, **ErrorEndEvent { error_code }**
- **EscalationEndEvent**, **EscalationThrowEvent**, **BoundaryEscalationEvent**
- **CompensationThrowEvent**, **CompensationEndEvent**, **BoundaryCompensationEvent**
- **CallActivity { called_element }**, **EmbeddedSubProcess { start_node_id }**, **SubProcessEndEvent**

Zusätzlich: bedingte Sequence Flows (`condition.rs`), Execution Listener (Rhai an Start/End),
Scope Event Listener für Event-Subprozesse.

## Modell
- `SequenceFlow { target, condition }`; `Token { id, current_node, variables, is_merged }`.
- `ProcessDefinition { key, id, version, nodes, flows, listeners, event_listeners, sub_processes }`,
  validiert bei Konstruktion.
- `ProcessDefinitionBuilder` mit `.node()`, `.flow()`, `.conditional_flow()`, `.listener()`,
  `.scope_event()`, `.sub_process()`.
- `TimerDefinition`: Duration, AbsoluteDate, CronCycle, RepeatingInterval.

## Laufzeit-Typen
`InstanceState` (11 Varianten): `Running`, `WaitingOnUserTask`, `WaitingOnServiceTask`,
`WaitingOnTimer`, `WaitingOnMessage`, `WaitingOnEventBasedGateway`, `ParallelExecution`,
`WaitingOnCallActivity`, `Completed`, `CompletedWithError`, `Suspended { previous_state }`.

`NextAction` (16 Varianten): `Continue`, `ContinueMultiple`, `WaitForUser`, `WaitForServiceTask`,
`WaitForJoin`, `WaitForTimer`, `WaitForMessage`, `WaitForEventGroup`, `Complete`, `ErrorEnd`,
`EscalationEnd`, `SpawnAndContinue`, `Terminate`, `WaitForCallActivity`, `MultiInstanceFork`,
`MultiInstanceNext`.

## External Tasks
`PendingServiceTask` mit `worker_id`, `lock_expiration`, `retries`, `error_message`.
Operationen: `fetch_and_lock_service_tasks`, `complete_service_task`, `fail_service_task`,
`extend_lock`, `handle_bpmn_error`.
Ein unhandled BPMN-Error wird als Incident (`retries = 0`, entsperrt) wieder eingefügt; der
Boundary-Error-Pfad entfernt den Task und läuft weiter.

## Conditions, Scripts, Persistence
- `evaluate_condition(expr, variables)`: `==`, `!=`, `>`, `>=`, `<`, `<=`, Truthy-Checks.
- Rhai-Engine embedded; `run_node_scripts()` verändert Token-Variablen.
- `WorkflowPersistence` deckt Instances, Definitions, User-/Service-Tasks, XML, Storage-Info und
  History ab (`append_history_entry`, `query_history`, `save_bpmn_xml`, `load_bpmn_xml`).
- Persistenz optional über `engine.with_persistence(Arc<dyn WorkflowPersistence>)`.

## Fehler und History
`EngineError`: `InvalidDefinition`, `NoSuchNode`, `NoSuchDefinition`, `NoSuchInstance`,
`TaskNotPending`, `InstanceCompleted`, `TimerMismatch`, `NoMatchingCondition`,
`ServiceTaskNotFound`, `ServiceTaskLocked`, `ServiceTaskNotLocked`, `DefinitionHasInstances`,
`PersistenceError`, `ScriptError`.
`HistoryEntry` mit `HistoryEventType`, `HistoryDiff`/`VariableDiff`, `ActorType` und `calculate_diff()`.

## Gotchas
- Timer laufen über `pending_timers` als SSOT plus `timer_due_index: BTreeMap<(expires_at, Uuid)>`.
  Mutationen nur über `insert_pending_timer` / `remove_pending_timer` / `retain_pending_timers`;
  direktes `get_mut` auf `expires_at` lässt den Index veralten.
- Inclusive-Joins registrieren nur für Splits (`is_split_gateway`, outgoing ≥ 2) eine `JoinBarrier`;
  Join-only-Gateways nicht.
- `process_timers` läuft über `due_ids`, nicht über die ganze Map.
- Externe Task-Queues: `service_task_topic_index` ist sekundär zu `pending_service_tasks`.

Details zu Modulen und Datenflüssen: `docs/knowledge-base/modules/engine-core/`.
