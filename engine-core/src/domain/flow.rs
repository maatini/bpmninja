use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceFlow {
    /// Target node ID this flow points to.
    pub target: String,
    /// Optional condition expression, e.g. `"amount > 100"`.
    pub condition: Option<String>,
}

impl SequenceFlow {
    /// Creates a simple (unconditional) sequence flow.
    pub fn simple(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            condition: None,
        }
    }

    /// Creates a conditional sequence flow.
    pub fn conditional(target: impl Into<String>, condition: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            condition: Some(condition.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------
