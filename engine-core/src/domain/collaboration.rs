//! Validated BPMN collaboration metadata shared by parser and runtime.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collaboration {
    pub id: String,
    pub participants: Vec<Participant>,
    pub message_flows: Vec<MessageFlow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    /// None identifies a black-box participant. Such pools cannot route internally.
    pub process_id: Option<String>,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageEndpoint {
    pub participant_id: String,
    pub process_id: String,
    pub element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFlow {
    pub id: String,
    pub message_id: String,
    pub message_name: String,
    pub source: MessageEndpoint,
    pub target: MessageEndpoint,
}
