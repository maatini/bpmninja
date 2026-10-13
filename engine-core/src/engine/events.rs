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
    pub state: InstanceState,
    pub current_node: String,
    pub business_key: String,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&ProcessInstance> for InstanceDelta {
    fn from(instance: &ProcessInstance) -> Self {
        Self {
            state: instance.state.clone(),
            current_node: instance.current_node.clone(),
            business_key: instance.business_key.clone(),
            started_at: instance.started_at,
            completed_at: instance.completed_at,
        }
    }
}

/// Grobgranulare Ereignisse, die die Engine bei Zustandsänderungen aussendet.
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
