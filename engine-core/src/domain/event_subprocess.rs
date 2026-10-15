use super::TimerDefinition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventSubProcessTrigger {
    Timer(TimerDefinition),
    Message(String),
    Signal(String),
    Error(Option<String>),
    Escalation(Option<String>),
}

/// Handler nodes belong to the original process definition, not a separately deployed child.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventSubProcess {
    pub id: String,
    pub start_node: String,
    pub node_ids: Vec<String>,
    pub trigger: EventSubProcessTrigger,
    pub is_interrupting: bool,
}
