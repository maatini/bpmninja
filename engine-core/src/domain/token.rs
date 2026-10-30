use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub id: Uuid,
    pub current_node: String,
    pub variables: HashMap<String, Value>,
    #[serde(default)]
    pub is_merged: bool,
    /// Lokale Aktivierung. Fehlt bei Token, die vor X11 persistiert wurden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_scope: Option<super::DataScope>,
}

impl Token {
    /// Variablen, die außerhalb der aktuellen Aktivität sichtbar sind.
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

    /// Abbruch verwirft die Aktivierung, ohne Output-Mappings anzuwenden.
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
            data_scope: None,
        }
    }

    /// Creates a new token with pre-populated variables.
    pub fn with_variables(start_node: &str, variables: HashMap<String, Value>) -> Self {
        Self {
            id: Uuid::new_v4(),
            current_node: start_node.to_string(),
            variables,
            is_merged: false,
            data_scope: None,
        }
    }
}
