use crate::domain::TimerDefinition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListenerEvent {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionListener {
    pub event: ListenerEvent,
    pub script: String,
}

// ---------------------------------------------------------------------------
// BPMN element types
// ---------------------------------------------------------------------------

/// A BPMN flow-node element.
///
/// Closed enum — the compiler enforces exhaustive matching, so adding a new
/// variant later will break every unhandled `match`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScopeEventListener {
    Timer {
        timer: TimerDefinition,
        is_interrupting: bool,
        target_definition: String,
    },
    Message {
        message_name: String,
        is_interrupting: bool,
        target_definition: String,
    },
    Error {
        error_code: Option<String>,
        target_definition: String,
    },
}

// ---------------------------------------------------------------------------
// Sequence flow (edge with optional condition)
// ---------------------------------------------------------------------------
