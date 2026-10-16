use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    #[allow(dead_code)]
    pub id: Uuid,
    pub current_node: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditional_wait: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_activation: Option<Uuid>,
    pub variables: HashMap<String, Value>,
    #[serde(default)]
    pub is_merged: bool,
    /// Set while the token executes one iteration of a loop / multi-instance activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_instance: Option<LoopInstance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_scope: Option<Box<super::DataScope>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_context: Option<Box<EventContext>>,
}

/// Identifies the loop iteration a token is executing (X4/X5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopInstance {
    pub node_id: String,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventContext {
    pub generation: u64,
    pub activation: Option<Uuid>,
    #[serde(default)]
    pub started_node: Option<String>,
    pub baseline: HashMap<String, Value>,
}

impl Token {
    pub(crate) fn scope_key(&self, node: &str) -> String {
        match self
            .event_context
            .as_ref()
            .and_then(|context| context.activation)
        {
            Some(id) => format!("{id}/{node}"),
            None => node.to_string(),
        }
    }
    pub(crate) fn inherit_event_context(&mut self, other: &Token) {
        self.event_context = other.event_context.clone();
    }

    /// Variables visible outside the current activity.
    pub fn process_variables(&self) -> HashMap<String, Value> {
        let mut variables = self.variables.clone();
        if let Some(scope) = &self.data_scope {
            for name in &scope.local_names {
                if let Some(value) = scope.original_variables.get(name) {
                    variables.insert(name.clone(), value.clone());
                } else {
                    variables.remove(name);
                }
            }
        }
        variables
    }

    pub(crate) fn apply_process_updates(&mut self, updates: &HashMap<String, Value>) {
        for (key, value) in updates {
            if let Some(context) = &mut self.event_context {
                if value.is_null() {
                    context.baseline.remove(key);
                } else {
                    context.baseline.insert(key.clone(), value.clone());
                }
            }
            if let Some(scope) = &mut self.data_scope {
                if value.is_null() {
                    scope.original_variables.remove(key);
                } else {
                    scope.original_variables.insert(key.clone(), value.clone());
                }
                if scope.local_names.contains(key) {
                    continue;
                }
            }
            if value.is_null() {
                self.variables.remove(key);
            } else {
                self.variables.insert(key.clone(), value.clone());
            }
        }
    }

    /// Cancellation discards activation writes without success mappings.
    pub fn discard_data_scope(&mut self) {
        if let Some(scope) = self.data_scope.take() {
            self.variables = scope.original_variables;
        }
    }

    /// Creates a new token positioned at the given node with empty variables.
    pub fn new(start_node: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            current_node: start_node.to_string(),
            conditional_wait: None,
            boundary_activation: None,
            variables: HashMap::new(),
            is_merged: false,
            loop_instance: None,
            data_scope: None,
            event_context: None,
        }
    }

    /// Creates a new token with pre-populated variables.
    #[allow(dead_code)]
    pub fn with_variables(start_node: &str, variables: HashMap<String, Value>) -> Self {
        Self {
            id: Uuid::new_v4(),
            current_node: start_node.to_string(),
            conditional_wait: None,
            boundary_activation: None,
            variables,
            is_merged: false,
            loop_instance: None,
            data_scope: None,
            event_context: None,
        }
    }
}

// ---------------------------------------------------------------------------
// File Reference
// ---------------------------------------------------------------------------
