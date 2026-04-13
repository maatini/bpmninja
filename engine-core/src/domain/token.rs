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
}

impl Token {
    /// Creates a new token positioned at the given node with empty variables.
    pub fn new(start_node: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            current_node: start_node.to_string(),
            variables: HashMap::new(),
            is_merged: false,
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
        }
    }
}

// ---------------------------------------------------------------------------
// File Reference
// ---------------------------------------------------------------------------
