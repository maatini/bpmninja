use crate::domain::{FileReference, Incident, IncidentKind, MAX_INCIDENTS_PER_INSTANCE, Token};
use crate::runtime::{
    PendingConditionalCatch, PendingMessageCatch, PendingServiceTask, PendingSignalCatch,
    PendingTimer, PendingUserTask,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum NextAction {
    /// The token should continue to the next node.
    Continue(Token),
    /// Multiple tokens should continue (inclusive gateway fork).
    ContinueMultiple(Vec<Token>),
    /// The engine must pause — a user task is pending.
    WaitForUser(PendingUserTask),
    /// The engine must pause — an external task is pending.
    WaitForServiceTask(PendingServiceTask),
    /// Token arrived at a join gateway but must wait for sibling tokens.
    WaitForJoin { gateway_id: String, token: Token },
    /// The engine must pause — a timer is pending.
    WaitForTimer(PendingTimer),
    /// The engine must pause — a message catch is pending.
    WaitForMessage(PendingMessageCatch),
    /// The engine must pause — a signal catch is pending (broadcast, no correlation).
    WaitForSignal(PendingSignalCatch),
    /// The engine must pause — a conditional catch is pending.
    WaitForCondition(PendingConditionalCatch),
    /// The engine must pause at an event-based gateway, registering multiple event listeners.
    WaitForEventGroup(Vec<NextAction>),
    /// The process reached an end event.
    Complete,
    /// Ends the current process instance with an error code (for error propagation).
    ErrorEnd { error_code: String },
    /// Ends the current path with an escalation code (non-fatal, propagates to parent).
    EscalationEnd { escalation_code: String },
    /// Spawns extra tokens (e.g. non-interrupting escalation handler) while the main token continues.
    SpawnAndContinue { main: Token, spawned: Vec<Token> },
    /// Terminate End Event: kill all active tokens and complete the instance immediately.
    Terminate,
    /// The engine must pause — a call activity (sub-process) is pending.
    WaitForCallActivity {
        called_element: String,
        token: Token,
    },
    /// Multi-instance parallel: spawn N tokens that each execute the same task.
    MultiInstanceFork { node_id: String, tokens: Vec<Token> },
    /// Multi-instance sequential: re-execute the same task node.
    MultiInstanceNext { node_id: String, token: Token },
}

// ---------------------------------------------------------------------------
// Instance state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiInstanceProgress {
    pub node_id: String,
    pub total: usize,
    pub completed: usize,
    /// For sequential MI: the token that will re-execute the task
    pub sequential_token: Option<Token>,
    /// Items of a sequential MI activity (parallel MI leaves this empty).
    /// Older snapshots without this field deserialize to an empty vector.
    #[serde(default)]
    pub items: Vec<Option<Value>>,
}

/// The state of a process instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstanceState {
    Running,
    WaitingOnUserTask {
        task_id: Uuid,
    },
    WaitingOnServiceTask {
        task_id: Uuid,
    },
    WaitingOnTimer {
        timer_id: Uuid,
    },
    WaitingOnMessage {
        message_id: Uuid,
    },
    WaitingOnSignal {
        catch_id: Uuid,
    },
    WaitingOnCondition {
        catch_id: Uuid,
    },
    WaitingOnEventBasedGateway,
    /// Multiple tokens are active; some may be waiting, some running.
    ParallelExecution {
        active_token_count: usize,
    },
    WaitingOnCallActivity {
        sub_instance_id: Uuid,
        token: Token,
    },
    Completed,
    /// Process ended in an ErrorEndEvent.
    CompletedWithError {
        error_code: String,
    },
    /// Instance is suspended – no timers fire, no tasks can be completed.
    /// Stores the state the instance was in before suspension so it can be
    /// restored on resume.
    Suspended {
        previous_state: Box<InstanceState>,
    },
}

// ---------------------------------------------------------------------------
// Process instance
// ---------------------------------------------------------------------------

/// A token actively traveling through the process graph.
/// Part of the Token-Registry on ProcessInstance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveToken {
    pub token: Token,
    /// ID of the fork gateway that spawned this token (None for the root token).
    pub fork_id: Option<String>,
    /// Index within the fork (0, 1, 2, ...) for deterministic ordering.
    pub branch_index: usize,
    /// Whether this token has completed (reached EndEvent or joined).
    pub completed: bool,
}

/// Synchronization barrier at a converging gateway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinBarrier {
    pub gateway_node_id: String,
    /// Number of tokens that must arrive before the join fires.
    pub expected_count: usize,
    /// Tokens that have arrived so far.
    pub arrived_tokens: Vec<Token>,
}

/// Tracks a successfully completed compensatable activity and its handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationRecord {
    /// The BPMN node ID of the activity that completed.
    pub activity_id: String,
    /// The BPMN node ID of the compensation handler (connected from BoundaryCompensationEvent).
    pub handler_node_id: String,
}

/// A spawned Call Activity that has not yet completed, keyed by child instance id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutstandingCall {
    /// BPMN node id of the Call Activity on the parent.
    pub node_id: String,
    /// Parent token parked at the Call Activity, resumed when the child completes.
    pub token: Token,
}

/// A live process instance tracked by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInstance {
    pub id: Uuid,
    pub definition_key: Uuid,
    pub business_key: String,
    #[serde(default)]
    pub parent_instance_id: Option<Uuid>,
    pub state: InstanceState,
    pub current_node: String,
    pub audit_log: Vec<String>,
    /// Current process variables (synced from the executing token).
    pub variables: HashMap<String, Value>,
    /// Central token store — the single source of truth for all active tokens.
    /// PendingTasks reference tokens by UUID instead of owning copies.
    #[serde(default)]
    pub tokens: HashMap<Uuid, Token>,
    /// All currently active tokens (Token-Registry).
    #[serde(default)]
    pub active_tokens: Vec<ActiveToken>,
    /// Join barriers waiting for tokens at converging gateways.
    #[serde(default)]
    pub join_barriers: HashMap<String, JoinBarrier>,
    /// Progress tracking for Multi-Instance tasks.
    #[serde(default)]
    pub multi_instance_state: HashMap<String, MultiInstanceProgress>,
    /// LIFO log of completed compensatable activities and their handlers.
    #[serde(default)]
    pub compensation_log: Vec<CompensationRecord>,
    /// Child Call-Activities still running, keyed by child instance id.
    /// Independent of `InstanceState` so parallel calls can resume individually.
    #[serde(default)]
    pub outstanding_calls: HashMap<Uuid, OutstandingCall>,
    /// Timestamp when this instance was started.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when this instance reached a terminal state.
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    /// Offene und aufgelöste Incidents dieser Instanz (begrenzt).
    #[serde(default)]
    pub incidents: Vec<Incident>,
}

impl ProcessInstance {
    /// Returns a typed FileReference if the variable exists and has type "file".
    pub fn get_file_reference(&self, var_name: &str) -> Option<FileReference> {
        self.variables
            .get(var_name)
            .and_then(FileReference::from_variable_value)
    }

    /// Returns all variable names that contain file references.
    pub fn file_variable_names(&self) -> Vec<String> {
        self.variables
            .iter()
            .filter(|(_, v)| FileReference::from_variable_value(v).is_some())
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Pushes an entry to the audit log, enforcing MAX_AUDIT_LOG_ENTRIES limit.
    pub fn push_audit_log(&mut self, entry: String) {
        self.audit_log.push(entry);
        if self.audit_log.len() > crate::runtime::MAX_AUDIT_LOG_ENTRIES {
            let overflow = self.audit_log.len() - crate::runtime::MAX_AUDIT_LOG_ENTRIES;
            self.audit_log.drain(0..overflow);
        }
    }

    /// Appends multiple entries to the audit log, enforcing MAX_AUDIT_LOG_ENTRIES limit.
    pub fn append_audit_log(&mut self, entries: &mut Vec<String>) {
        self.audit_log.append(entries);
        if self.audit_log.len() > crate::runtime::MAX_AUDIT_LOG_ENTRIES {
            let overflow = self.audit_log.len() - crate::runtime::MAX_AUDIT_LOG_ENTRIES;
            self.audit_log.drain(0..overflow);
        }
    }

    /// Legt einen Incident an oder führt ihn mit einem offenen Incident
    /// gleicher Art und gleicher Zuordnung (`node_id`, `task_id`) zusammen.
    ///
    /// Gibt die ID des angelegten bzw. aktualisierten Incidents zurück.
    pub fn push_or_touch_incident(
        &mut self,
        kind: IncidentKind,
        node_id: Option<String>,
        task_id: Option<Uuid>,
        message: impl Into<String>,
        details: Option<String>,
    ) -> Uuid {
        let now = Utc::now();
        let message = message.into();

        if let Some(existing) = self.incidents.iter_mut().find(|incident| {
            incident.resolved_at.is_none()
                && incident.kind == kind
                && incident.node_id == node_id
                && incident.task_id == task_id
        }) {
            existing.occurrences = existing.occurrences.saturating_add(1);
            existing.last_seen_at = now;
            existing.message = message;
            if details.is_some() {
                existing.details = details;
            }
            return existing.id;
        }

        let incident = Incident {
            id: Uuid::new_v4(),
            kind,
            node_id,
            task_id,
            message,
            details,
            created_at: now,
            last_seen_at: now,
            occurrences: 1,
            resolved_at: None,
            resolved_by: None,
        };
        let id = incident.id;
        self.incidents.push(incident);
        self.trim_incidents();
        id
    }

    /// Begrenzt die Incident-Liste; aufgelöste Incidents werden zuerst verworfen.
    fn trim_incidents(&mut self) {
        while self.incidents.len() > MAX_INCIDENTS_PER_INSTANCE {
            let index = self
                .incidents
                .iter()
                .enumerate()
                .filter(|(_, incident)| incident.resolved_at.is_some())
                .min_by_key(|(_, incident)| incident.created_at)
                .map(|(index, _)| index)
                .or_else(|| {
                    self.incidents
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, incident)| incident.created_at)
                        .map(|(index, _)| index)
                });
            match index {
                Some(index) => {
                    self.incidents.remove(index);
                }
                None => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn make_instance() -> ProcessInstance {
        ProcessInstance {
            id: Uuid::new_v4(),
            definition_key: Uuid::new_v4(),
            business_key: "test".into(),
            parent_instance_id: None,
            state: InstanceState::Running,
            current_node: "start".into(),
            audit_log: vec![],
            variables: HashMap::new(),
            tokens: HashMap::new(),
            active_tokens: vec![],
            join_barriers: HashMap::new(),
            multi_instance_state: HashMap::new(),
            compensation_log: Vec::new(),
            outstanding_calls: HashMap::new(),
            started_at: None,
            completed_at: None,
            incidents: Vec::new(),
        }
    }

    #[test]
    fn push_audit_log_enforces_limit() {
        // Catches: replace > with ==, >=; replace - with +, /
        let mut inst = make_instance();
        for i in 0..crate::runtime::MAX_AUDIT_LOG_ENTRIES + 50 {
            inst.push_audit_log(format!("entry {i}"));
        }
        assert_eq!(inst.audit_log.len(), crate::runtime::MAX_AUDIT_LOG_ENTRIES);
        // The latest entry should be the last one added
        assert!(inst.audit_log.last().unwrap().contains("249"));
        // First entry should be "entry 50" (first 50 were trimmed)
        assert!(
            inst.audit_log.first().unwrap().contains("entry 50"),
            "Oldest entry should be 'entry 50', got: {}",
            inst.audit_log.first().unwrap()
        );
    }

    #[test]
    fn push_audit_log_exactly_at_limit_does_not_trim() {
        // Catches: replace > with >= in push_audit_log
        // At exactly MAX entries, no trimming should occur
        let mut inst = make_instance();
        for i in 0..crate::runtime::MAX_AUDIT_LOG_ENTRIES {
            inst.push_audit_log(format!("entry {i}"));
        }
        assert_eq!(inst.audit_log.len(), crate::runtime::MAX_AUDIT_LOG_ENTRIES);
        // First entry should still be "entry 0" (no trimming at boundary)
        assert!(
            inst.audit_log.first().unwrap().contains("entry 0"),
            "At exact limit, 'entry 0' should still be present"
        );
    }

    #[test]
    fn push_audit_log_one_over_limit_trims_one() {
        // One entry over limit → should trim exactly 1 entry from front
        let mut inst = make_instance();
        for i in 0..crate::runtime::MAX_AUDIT_LOG_ENTRIES + 1 {
            inst.push_audit_log(format!("entry {i}"));
        }
        assert_eq!(inst.audit_log.len(), crate::runtime::MAX_AUDIT_LOG_ENTRIES);
        // "entry 0" should be gone, "entry 1" should be first
        assert!(
            inst.audit_log.first().unwrap().contains("entry 1"),
            "First entry should be 'entry 1' after trimming, got: {}",
            inst.audit_log.first().unwrap()
        );
    }

    #[test]
    fn push_audit_log_under_limit() {
        let mut inst = make_instance();
        inst.push_audit_log("one".into());
        inst.push_audit_log("two".into());
        assert_eq!(inst.audit_log.len(), 2);
    }

    #[test]
    fn append_audit_log_enforces_limit() {
        // Catches: replace append_audit_log with (); replace > with ==, <, >=; replace - with +, /
        let mut inst = make_instance();
        let mut entries: Vec<String> = (0..crate::runtime::MAX_AUDIT_LOG_ENTRIES + 10)
            .map(|i| format!("batch {i}"))
            .collect();
        inst.append_audit_log(&mut entries);
        assert_eq!(inst.audit_log.len(), crate::runtime::MAX_AUDIT_LOG_ENTRIES);
    }

    #[test]
    fn append_audit_log_exactly_at_limit_does_not_trim() {
        // Catches: replace > with >= in append_audit_log
        let mut inst = make_instance();
        let mut entries: Vec<String> = (0..crate::runtime::MAX_AUDIT_LOG_ENTRIES)
            .map(|i| format!("batch {i}"))
            .collect();
        inst.append_audit_log(&mut entries);
        assert_eq!(inst.audit_log.len(), crate::runtime::MAX_AUDIT_LOG_ENTRIES);
        assert!(inst.audit_log.first().unwrap().contains("batch 0"));
    }

    #[test]
    fn append_audit_log_under_limit() {
        let mut inst = make_instance();
        let mut entries = vec!["a".into(), "b".into()];
        inst.append_audit_log(&mut entries);
        assert_eq!(inst.audit_log.len(), 2);
    }

    #[test]
    fn get_file_reference_returns_some_for_file() {
        // Catches: replace get_file_reference -> Option<FileReference> with None
        let mut inst = make_instance();
        inst.variables.insert(
            "doc".into(),
            json!({
                "type": "file",
                "object_key": "file:abc",
                "filename": "doc.pdf",
                "mime_type": "application/pdf",
                "size_bytes": 1024,
                "uploaded_at": "2026-01-01T00:00:00Z"
            }),
        );
        let fr = inst.get_file_reference("doc");
        assert!(fr.is_some());
        assert_eq!(fr.unwrap().filename, "doc.pdf");
    }

    #[test]
    fn get_file_reference_returns_none_for_non_file() {
        let mut inst = make_instance();
        inst.variables.insert("x".into(), json!(42));
        assert!(inst.get_file_reference("x").is_none());
        assert!(inst.get_file_reference("missing").is_none());
    }

    #[test]
    fn file_variable_names_returns_correct_list() {
        // Catches: replace file_variable_names -> Vec<String> with vec![], vec![String::new()], vec!["xyzzy".into()]
        let mut inst = make_instance();
        inst.variables.insert("x".into(), json!(42));
        inst.variables.insert(
            "file1".into(),
            json!({"type": "file", "object_key": "k1", "filename": "a.txt", "mime_type": "text/plain", "size_bytes": 10, "uploaded_at": "2026-01-01T00:00:00Z"}),
        );
        let names = inst.file_variable_names();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"file1".to_string()));
    }

    #[test]
    fn file_variable_names_returns_empty_when_no_files() {
        let mut inst = make_instance();
        inst.variables.insert("x".into(), json!(42));
        assert!(inst.file_variable_names().is_empty());
    }

    #[test]
    fn push_or_touch_incident_merges_open_entries() {
        let mut inst = make_instance();
        let first = inst.push_or_touch_incident(
            IncidentKind::Timer,
            Some("timer_1".into()),
            None,
            "boom".to_string(),
            None,
        );
        let second = inst.push_or_touch_incident(
            IncidentKind::Timer,
            Some("timer_1".into()),
            None,
            "boom again".to_string(),
            Some("details".into()),
        );

        assert_eq!(first, second);
        assert_eq!(inst.incidents.len(), 1);
        assert_eq!(inst.incidents[0].occurrences, 2);
        assert_eq!(inst.incidents[0].message, "boom again");
        assert_eq!(inst.incidents[0].details.as_deref(), Some("details"));

        // Andere Node oder andere Art bleibt ein eigener Incident.
        inst.push_or_touch_incident(
            IncidentKind::Timer,
            Some("timer_2".into()),
            None,
            "other".to_string(),
            None,
        );
        inst.push_or_touch_incident(
            IncidentKind::Persistence,
            Some("timer_1".into()),
            None,
            "other kind".to_string(),
            None,
        );
        assert_eq!(inst.incidents.len(), 3);
    }

    #[test]
    fn resolved_incident_is_not_reopened() {
        let mut inst = make_instance();
        let id = inst.push_or_touch_incident(
            IncidentKind::Timer,
            Some("timer_1".into()),
            None,
            "boom".to_string(),
            None,
        );
        inst.incidents[0].resolved_at = Some(Utc::now());

        let new_id = inst.push_or_touch_incident(
            IncidentKind::Timer,
            Some("timer_1".into()),
            None,
            "boom again".to_string(),
            None,
        );

        assert_ne!(id, new_id);
        assert_eq!(inst.incidents.len(), 2);
        assert_eq!(
            inst.incidents
                .iter()
                .filter(|incident| incident.resolved_at.is_none())
                .count(),
            1
        );
    }

    #[test]
    fn incidents_are_capped_and_prefer_open_entries() {
        let mut inst = make_instance();
        for i in 0..MAX_INCIDENTS_PER_INSTANCE + 10 {
            inst.push_or_touch_incident(
                IncidentKind::Timer,
                Some(format!("timer_{i}")),
                None,
                "boom".to_string(),
                None,
            );
        }
        assert_eq!(inst.incidents.len(), MAX_INCIDENTS_PER_INSTANCE);

        // Alle bestehenden auflösen, dann neue offene Incidents anlegen:
        // beim Trimmen müssen aufgelöste zuerst weichen.
        for incident in &mut inst.incidents {
            incident.resolved_at = Some(Utc::now());
        }
        for i in 0..5 {
            inst.push_or_touch_incident(
                IncidentKind::Timer,
                Some(format!("new_{i}")),
                None,
                "boom".to_string(),
                None,
            );
        }

        assert_eq!(inst.incidents.len(), MAX_INCIDENTS_PER_INSTANCE);
        assert_eq!(
            inst.incidents
                .iter()
                .filter(|incident| incident.resolved_at.is_none())
                .count(),
            5
        );
    }
}
