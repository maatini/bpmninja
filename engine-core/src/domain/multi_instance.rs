use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiInstanceDef {
    pub is_sequential: bool,
    /// Number of instances: integer literal, variable name or `${var}`.
    pub loop_cardinality: Option<String>,
    /// Variable holding an array; one instance per element.
    pub collection: Option<String>,
    /// Local variable receiving the current collection element.
    pub element_variable: Option<String>,
    /// Ends the activity early once true (evaluated after each instance).
    #[serde(default)]
    pub completion_condition: Option<String>,
    /// Variable receiving the per-instance `output_element` values, in index order.
    #[serde(default)]
    pub output_collection: Option<String>,
    #[serde(default)]
    pub output_element: Option<String>,
}

/// `standardLoopCharacteristics`: repeat while `loop_condition` holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandardLoopDef {
    pub loop_condition: Option<String>,
    /// Evaluate the condition before the first iteration (`while` instead of `do-while`).
    pub test_before: bool,
    pub loop_maximum: Option<u32>,
}

/// Loop behaviour of an activity (X4 / X5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoopCharacteristics {
    MultiInstance(MultiInstanceDef),
    Standard(StandardLoopDef),
}

/// Strips a Camunda-style `${...}` wrapper from an expression.
pub fn strip_expression(expr: &str) -> &str {
    let trimmed = expr.trim();
    trimmed
        .strip_prefix("${")
        .and_then(|rest| rest.strip_suffix('}'))
        .map(str::trim)
        .unwrap_or(trimmed)
}
