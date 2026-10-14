use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    #[allow(dead_code)]
    pub id: Uuid,
    pub current_node: String,
    pub variables: HashMap<String, Value>,
    #[serde(default)]
    pub is_merged: bool,
    /// Set while the token executes one iteration of a loop / multi-instance activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_instance: Option<LoopInstance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_scope: Option<Box<super::DataScope>>,
}

/// Identifies the loop iteration a token is executing (X4/X5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopInstance {
    pub node_id: String,
    pub index: usize,
}

impl Token {
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
            variables: HashMap::new(),
            is_merged: false,
            loop_instance: None,
            data_scope: None,
        }
    }

    /// Creates a new token with pre-populated variables.
    #[allow(dead_code)]
    pub fn with_variables(start_node: &str, variables: HashMap<String, Value>) -> Self {
        Self {
            id: Uuid::new_v4(),
            current_node: start_node.to_string(),
            variables,
            is_merged: false,
            loop_instance: None,
            data_scope: None,
        }
    }
}

// ---------------------------------------------------------------------------
// File Reference
// ---------------------------------------------------------------------------
