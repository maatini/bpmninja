# bpmn-parser — Interfaces

## Public API

```rust
/// Parses a BPMN 2.0 XML string into a ProcessDefinition.
///
/// Handles all 29 BpmnElement variants, sub-process flattening,
/// ISO 8601 timer parsing, and Camunda extension elements.
pub fn parse_bpmn_xml(xml: &str) -> EngineResult<ProcessDefinition>
```

## Input/Output

| Direction | Type | Notes |
|-----------|------|-------|
| **Input** | `&str` | Raw BPMN 2.0 XML (UTF-8) |
| **Output (Ok)** | `ProcessDefinition` | Fully parsed definition with nodes, edges, listeners |
| **Output (Err)** | `EngineError::InvalidDefinition(String)` | Descriptive error message for invalid XML |

## Internal Models (private)

```rust
// Intermediate deserialization structs (models.rs)
struct BpmnDefinitions {
    process: Vec<BpmnProcess>,
}

struct BpmnProcess {
    id: String,
    name: Option<String>,
    // ... all BPMN XML attributes
}

struct BpmnExtensionElements {
    execution_listeners: Vec<BpmnExecutionListener>,
}

struct BpmnExecutionListener {
    event: String,       // "start" or "end"
    script: Option<BpmnScript>,
}
```

## Supported BPMN XML Elements

The parser maps these XML tags to `BpmnElement` variants:

| XML Tag | BpmnElement Variant |
|---------|-------------------|
| `startEvent` (no trigger) | `StartEvent` |
| `startEvent > timerEventDefinition` | `TimerStartEvent(timer_def)` |
| `startEvent > messageEventDefinition` | `MessageStartEvent { message_name }` |
| `endEvent` | `EndEvent` |
| `endEvent > terminateEventDefinition` | `TerminateEndEvent` |
| `endEvent > errorEventDefinition` | `ErrorEndEvent { error_code }` |
| `endEvent > escalationEventDefinition` | `EscalationEndEvent { escalation_code }` |
| `endEvent > compensationEventDefinition` | `CompensationEndEvent { activity_ref }` |
| `userTask` | `UserTask(assignee)` |
| `serviceTask` | `ServiceTask { topic, multi_instance }` |
| `scriptTask` | `ScriptTask { script, multi_instance }` |
| `sendTask` | `SendTask { message_name, multi_instance }` |
| `exclusiveGateway` | `ExclusiveGateway { default }` |
| `parallelGateway` | `ParallelGateway` |
| `inclusiveGateway` | `InclusiveGateway` |
| `eventBasedGateway` | `EventBasedGateway` |
| `complexGateway` | `ComplexGateway { join_condition, default }` |
| `intermediateCatchEvent > timerEventDefinition` | `TimerCatchEvent(timer_def)` |
| `intermediateCatchEvent > messageEventDefinition` | `MessageCatchEvent { message_name }` |
| `boundaryEvent > timerEventDefinition` | `BoundaryTimerEvent { attached_to, timer, cancel_activity }` |
| `boundaryEvent > messageEventDefinition` | `BoundaryMessageEvent { attached_to, message_name, cancel_activity }` |
| `boundaryEvent > errorEventDefinition` | `BoundaryErrorEvent { attached_to, error_code }` |
| `boundaryEvent > escalationEventDefinition` | `BoundaryEscalationEvent { ... }` |
| `boundaryEvent > compensationEventDefinition` | `BoundaryCompensationEvent { attached_to }` |
| `intermediateThrowEvent > escalationEventDefinition` | `EscalationThrowEvent { escalation_code }` |
| `intermediateThrowEvent > compensateEventDefinition` | `CompensationThrowEvent { activity_ref }` |
| `callActivity` | `CallActivity { called_element }` |
| `subProcess` (embedded) | `EmbeddedSubProcess { start_node_id }` (flattened) |
