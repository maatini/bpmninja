use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{EngineError, EngineResult};

// ---------------------------------------------------------------------------
// BPMN element types
// ---------------------------------------------------------------------------

/// A BPMN flow-node element.
///
/// Closed enum — the compiler enforces exhaustive matching, so adding a new
/// variant later will break every unhandled `match`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BpmnElement {
    /// A plain (none) start event — the process starts immediately.
    StartEvent,
    /// A timer-triggered start event that fires after the given duration.
    TimerStartEvent(Duration),
    /// An end event — the process terminates here.
    EndEvent,
    /// A service task that invokes a registered async handler by name.
    ServiceTask(String),
    /// A user task assigned to a specific role or user.
    UserTask(String),
    /// An external task that can be fetched and completed by remote workers.
    ExternalTask { topic: String },
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// A token traveling through the process graph.
///
/// Carries a unique ID, its current position, and a bag of process variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    #[allow(dead_code)]
    pub id: Uuid,
    pub current_node: String,
    pub variables: HashMap<String, Value>,
}

impl Token {
    /// Creates a new token positioned at the given node with empty variables.
    pub fn new(start_node: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            current_node: start_node.to_string(),
            variables: HashMap::new(),
        }
    }

    /// Creates a new token with pre-populated variables.
    #[allow(dead_code)]
    pub fn with_variables(start_node: &str, variables: HashMap<String, Value>) -> Self {
        Self {
            id: Uuid::new_v4(),
            current_node: start_node.to_string(),
            variables,
        }
    }
}

// ---------------------------------------------------------------------------
// Process definition (validated at construction time)
// ---------------------------------------------------------------------------

/// An immutable, structurally validated BPMN process definition.
///
/// - `nodes`: maps each node ID → its `BpmnElement` type.
/// - `flows`: maps each source node ID → target node ID (single outgoing
///   flow per node for this minimal engine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDefinition {
    pub id: String,
    pub nodes: HashMap<String, BpmnElement>,
    pub flows: HashMap<String, String>,
}

impl ProcessDefinition {
    /// Creates a new process definition after validating structural integrity.
    ///
    /// # Validation rules
    /// - Exactly one start event (StartEvent or TimerStartEvent) must exist.
    /// - At least one end event must exist.
    /// - All flow targets must reference existing node IDs.
    /// - Every non-end node must have an outgoing flow.
    pub fn new(
        id: impl Into<String>,
        nodes: HashMap<String, BpmnElement>,
        flows: HashMap<String, String>,
    ) -> EngineResult<Self> {
        let id = id.into();

        // --- exactly one start event ---
        let start_count = nodes
            .values()
            .filter(|e| matches!(e, BpmnElement::StartEvent | BpmnElement::TimerStartEvent(_)))
            .count();

        if start_count == 0 {
            return Err(EngineError::InvalidDefinition(
                "No start event defined".into(),
            ));
        }
        if start_count > 1 {
            return Err(EngineError::InvalidDefinition(
                "Multiple start events are not supported".into(),
            ));
        }

        // --- at least one end event ---
        let end_count = nodes
            .values()
            .filter(|e| matches!(e, BpmnElement::EndEvent))
            .count();
        if end_count == 0 {
            return Err(EngineError::InvalidDefinition(
                "No end event defined".into(),
            ));
        }

        // --- all flow targets reference existing nodes ---
        for (from, to) in &flows {
            if !nodes.contains_key(from) {
                return Err(EngineError::NoSuchNode(from.clone()));
            }
            if !nodes.contains_key(to) {
                return Err(EngineError::NoSuchNode(to.clone()));
            }
        }

        // --- every non-end node must have an outgoing flow ---
        for (node_id, element) in &nodes {
            if matches!(element, BpmnElement::EndEvent) {
                continue;
            }
            if !flows.contains_key(node_id) {
                return Err(EngineError::InvalidDefinition(format!(
                    "Node '{node_id}' has no outgoing sequence flow"
                )));
            }
        }

        Ok(Self { id, nodes, flows })
    }

    /// Returns the (id, element) of the start event.
    pub fn start_event(&self) -> Option<(&str, &BpmnElement)> {
        self.nodes.iter().find_map(|(id, e)| {
            if matches!(e, BpmnElement::StartEvent | BpmnElement::TimerStartEvent(_)) {
                Some((id.as_str(), e))
            } else {
                None
            }
        })
    }

    /// Returns the element at the given node ID.
    pub fn get_node(&self, id: &str) -> Option<&BpmnElement> {
        self.nodes.get(id)
    }

    /// Returns the target node ID for the outgoing flow from `from_id`.
    pub fn next_node(&self, from_id: &str) -> Option<&str> {
        self.flows.get(from_id).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// Builder helper (ergonomic construction)
// ---------------------------------------------------------------------------

/// Fluent builder for creating a `ProcessDefinition`.
pub struct ProcessDefinitionBuilder {
    id: String,
    nodes: HashMap<String, BpmnElement>,
    flows: HashMap<String, String>,
}

impl ProcessDefinitionBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            nodes: HashMap::new(),
            flows: HashMap::new(),
        }
    }

    /// Adds a node to the definition.
    pub fn node(mut self, id: impl Into<String>, element: BpmnElement) -> Self {
        self.nodes.insert(id.into(), element);
        self
    }

    /// Adds a sequence flow (edge) between two nodes.
    pub fn flow(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.flows.insert(from.into(), to.into());
        self
    }

    /// Builds and validates the definition.
    pub fn build(self) -> EngineResult<ProcessDefinition> {
        ProcessDefinition::new(self.id, self.nodes, self.flows)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_definition_with_builder() {
        let def = ProcessDefinitionBuilder::new("p1")
            .node("start", BpmnElement::StartEvent)
            .node("svc", BpmnElement::ServiceTask("do_it".into()))
            .node("end", BpmnElement::EndEvent)
            .flow("start", "svc")
            .flow("svc", "end")
            .build();
        assert!(def.is_ok());
    }

    #[test]
    fn rejects_missing_start() {
        let def = ProcessDefinitionBuilder::new("p1")
            .node("end", BpmnElement::EndEvent)
            .build();
        assert!(matches!(
            def,
            Err(EngineError::InvalidDefinition(msg)) if msg.contains("No start event")
        ));
    }

    #[test]
    fn rejects_missing_end() {
        let def = ProcessDefinitionBuilder::new("p1")
            .node("start", BpmnElement::StartEvent)
            .flow("start", "nowhere")
            .build();
        assert!(def.is_err());
    }

    #[test]
    fn rejects_dangling_flow() {
        let def = ProcessDefinitionBuilder::new("p1")
            .node("start", BpmnElement::StartEvent)
            .node("end", BpmnElement::EndEvent)
            .flow("start", "end")
            .flow("end", "ghost")
            .build();
        assert!(matches!(def, Err(EngineError::NoSuchNode(id)) if id == "ghost"));
    }

    #[test]
    fn rejects_node_without_outgoing_flow() {
        let def = ProcessDefinitionBuilder::new("p1")
            .node("start", BpmnElement::StartEvent)
            .node("orphan", BpmnElement::ServiceTask("noop".into()))
            .node("end", BpmnElement::EndEvent)
            .flow("start", "end")
            .build();
        assert!(matches!(
            def,
            Err(EngineError::InvalidDefinition(msg)) if msg.contains("orphan")
        ));
    }

    #[test]
    fn find_node_and_next_work() {
        let def = ProcessDefinitionBuilder::new("p1")
            .node("start", BpmnElement::StartEvent)
            .node("svc", BpmnElement::ServiceTask("action".into()))
            .node("end", BpmnElement::EndEvent)
            .flow("start", "svc")
            .flow("svc", "end")
            .build()
            .unwrap();

        assert_eq!(def.get_node("svc"), Some(&BpmnElement::ServiceTask("action".into())));
        assert_eq!(def.next_node("start"), Some("svc"));
        assert_eq!(def.next_node("end"), None);
    }

    #[test]
    fn token_creation() {
        let token = Token::new("start");
        assert_eq!(token.current_node, "start");
        assert!(token.variables.is_empty());
    }

    #[test]
    fn timer_start_event_definition() {
        let def = ProcessDefinitionBuilder::new("timer")
            .node("ts", BpmnElement::TimerStartEvent(Duration::from_secs(5)))
            .node("end", BpmnElement::EndEvent)
            .flow("ts", "end")
            .build();
        assert!(def.is_ok());
    }
}
