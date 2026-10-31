//! Validierte BPMN-Collaboration-Metadaten für Parser und Laufzeit.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::ProcessDefinition;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collaboration {
    pub id: String,
    pub participants: Vec<Participant>,
    pub message_flows: Vec<MessageFlow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    pub id: String,
    /// `None` ist ein Black-Box-Participant. Solche Pools routen nicht intern.
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

/// Ein atomarer Collection-Commit: alle ausführbaren Pools, das Original-XML
/// und die Collaboration-Metadaten. Gelöschte Mitglieder bleiben als Tombstone
/// im Datensatz, damit ein Neustart sie nicht wiederbelebt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionCollection {
    pub id: Uuid,
    pub definitions: Vec<ProcessDefinition>,
    pub xml: String,
    pub collaborations: serde_json::Value,
    #[serde(default)]
    pub deleted: Vec<Uuid>,
}
