use super::TimerDefinition;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Auslöser eines Event-Subprozesses. Der Handler liegt im selben Prozessgraphen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventSubProcessTrigger {
    Timer(TimerDefinition),
    Message(String),
    Signal(String),
    /// `None` fängt jeden Fehlercode.
    Error(Option<String>),
    /// `None` fängt jeden Eskalationscode.
    Escalation(Option<String>),
}

/// Event-Subprozess auf Prozessebene. Die Handler-Knoten stehen in `ProcessDefinition::nodes`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventSubProcess {
    pub id: String,
    pub start_node: String,
    pub node_ids: Vec<String>,
    pub trigger: EventSubProcessTrigger,
    pub is_interrupting: bool,
}

/// Laufende Subscription eines Event-Subprozesses. Liegt auf der Instanz, damit ein
/// Neustart Timer und Catches aus diesem Datensatz wiederherstellen kann.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventSubscription {
    pub id: Uuid,
    pub subprocess_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_due: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u32>,
}
