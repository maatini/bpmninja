//! Engine-interne Event-Typen für Push-Benachrichtigungen.
//!
//! Der `WorkflowEngine` hält einen `tokio::sync::broadcast::Sender<EngineEvent>`.
//! Subscriber (z.B. der SSE-Handler im engine-server) erhalten via
//! `engine.subscribe_events()` einen eigenen Receiver.

use super::WorkflowEngine;
use crate::{InstanceState, ProcessInstance};
use serde::Serialize;
use uuid::Uuid;

/// Kleine Listenprojektion; keine Variablen, Tokens oder Audit-Logs im Broadcast.
#[derive(Debug, Clone, Serialize)]
pub struct InstanceDelta {
    pub state: InstanceStatus,
    pub current_node: String,
    pub business_key: String,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&ProcessInstance> for InstanceDelta {
    fn from(instance: &ProcessInstance) -> Self {
        Self {
            state: InstanceStatus::from(&instance.state),
            current_node: instance.current_node.clone(),
            business_key: instance.business_key.clone(),
            started_at: instance.started_at,
            completed_at: instance.completed_at,
        }
    }
}

/// Wie der Instanzzustand, aber ohne den geparkten Token einer Call-Activity.
#[derive(Debug, Clone, Serialize)]
pub enum InstanceStatus {
    Running,
    WaitingOnUserTask { task_id: Uuid },
    WaitingOnServiceTask { task_id: Uuid },
    WaitingOnTimer { timer_id: Uuid },
    WaitingOnMessage { message_id: Uuid },
    WaitingOnEventBasedGateway,
    ParallelExecution { active_token_count: usize },
    WaitingOnCallActivity { sub_instance_id: Uuid },
    Completed,
    CompletedWithError { error_code: String },
    Suspended { previous_state: Box<InstanceStatus> },
}

impl From<&InstanceState> for InstanceStatus {
    fn from(state: &InstanceState) -> Self {
        match state {
            InstanceState::Running => Self::Running,
            InstanceState::WaitingOnUserTask { task_id } => {
                Self::WaitingOnUserTask { task_id: *task_id }
            }
            InstanceState::WaitingOnServiceTask { task_id } => {
                Self::WaitingOnServiceTask { task_id: *task_id }
            }
            InstanceState::WaitingOnTimer { timer_id } => Self::WaitingOnTimer {
                timer_id: *timer_id,
            },
            InstanceState::WaitingOnMessage { message_id } => Self::WaitingOnMessage {
                message_id: *message_id,
            },
            InstanceState::WaitingOnEventBasedGateway => Self::WaitingOnEventBasedGateway,
            InstanceState::ParallelExecution { active_token_count } => Self::ParallelExecution {
                active_token_count: *active_token_count,
            },
            InstanceState::WaitingOnCallActivity {
                sub_instance_id, ..
            } => Self::WaitingOnCallActivity {
                sub_instance_id: *sub_instance_id,
            },
            InstanceState::Completed => Self::Completed,
            InstanceState::CompletedWithError { error_code } => Self::CompletedWithError {
                error_code: error_code.clone(),
            },
            InstanceState::Suspended { previous_state } => Self::Suspended {
                previous_state: Box::new(Self::from(&**previous_state)),
            },
        }
    }
}

/// Ereignisse mit Entity-IDs und kompakten Zustandsprojektionen.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    /// Eine Prozessinstanz wurde gestartet, hat einen Schritt ausgeführt oder ist abgeschlossen.
    InstanceChanged {
        instance_id: Uuid,
        definition_key: Uuid,
        removed: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<InstanceDelta>,
    },
    /// Ein User Task oder Service Task wurde erstellt oder abgeschlossen.
    TaskChanged {
        instance_id: Uuid,
        task_id: Uuid,
        definition_key: Option<Uuid>,
    },
    /// Eine Prozessdefinition wurde deployed oder gelöscht.
    DefinitionChanged { definition_key: Uuid },
    /// Lifecycle of one handler activation within the owning process instance.
    EventSubprocessChanged {
        event_id: Uuid,
        instance_id: Uuid,
        definition_key: Uuid,
        subprocess_id: String,
        activation_id: Uuid,
        phase: String,
        reason: String,
    },
}

impl WorkflowEngine {
    pub(crate) async fn emit_task_changed(&self, instance_id: Uuid, task_id: Uuid) {
        if self.event_tx.receiver_count() == 0 {
            return;
        }
        let definition_key = match self.instances.get(&instance_id).await {
            Some(instance) => Some(instance.read().await.definition_key),
            None => None,
        };
        self.emit_event(EngineEvent::TaskChanged {
            instance_id,
            task_id,
            definition_key,
        });
    }

    pub(crate) async fn emit_instance_changed(&self, instance_id: Uuid) {
        if self.event_tx.receiver_count() == 0 {
            return;
        }
        if let Some(instance) = self.instances.get(&instance_id).await {
            let instance = instance.read().await;
            self.emit_event(EngineEvent::InstanceChanged {
                instance_id,
                definition_key: instance.definition_key,
                removed: false,
                delta: Some(InstanceDelta::from(&*instance)),
            });
        }
    }
}
