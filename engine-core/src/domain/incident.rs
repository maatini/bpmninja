//! Einheitliches Incident-Modell für Prozessinstanzen.
//!
//! Incidents sind persistente, aber bewusst begrenzte Diagnosedatensätze. Sie
//! ersetzen kein Audit-Log und keine History: dieselbe Störung wird bei
//! wiederholtem Auftreten über `occurrences` hochgezählt statt dupliziert.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Maximale Anzahl Incidents pro Prozessinstanz (älteste werden verworfen).
pub const MAX_INCIDENTS_PER_INSTANCE: usize = 100;

/// Ursache eines Incidents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentKind {
    /// Service-Task hat alle Retries verbraucht oder meldet einen unbehandelten BPMN-Error.
    ServiceTask,
    /// Timer konnte nicht ausgewertet werden.
    Timer,
    /// Persistenz-Job ist nach dem Retry-Limit endgültig fehlgeschlagen.
    Persistence,
    /// BPMN-Error ohne passenden Catch am Prozessscope.
    BpmnError,
}

impl IncidentKind {
    /// Stabiler Bezeichner für Metriken und Logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ServiceTask => "service_task",
            Self::Timer => "timer",
            Self::Persistence => "persistence",
            Self::BpmnError => "bpmn_error",
        }
    }
}

/// Ein einzelner Incident an einer Prozessinstanz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub kind: IncidentKind,
    /// BPMN-Node, an dem die Störung auftrat (falls zuordenbar).
    pub node_id: Option<String>,
    /// Betroffener Service-Task (falls zuordenbar).
    pub task_id: Option<Uuid>,
    pub message: String,
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Zeitpunkt des letzten Auftretens (bei Deduplizierung aktualisiert).
    pub last_seen_at: DateTime<Utc>,
    /// Anzahl der zusammengeführten Vorkommen.
    pub occurrences: u32,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}
