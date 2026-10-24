//! Incident-Lebenszyklus auf Engine-Ebene.
//!
//! Incidents werden direkt an der Prozessinstanz gehalten und mit ihr
//! persistiert. Es gibt bewusst keine zusätzliche Persistence-Trait-Methode:
//! die Incident-Liste ist Teil des `ProcessInstance`-Dokuments.

use chrono::Utc;
use uuid::Uuid;

use super::WorkflowEngine;
use super::instance_store::InstanceStore;
use crate::domain::{EngineError, EngineResult, Incident, IncidentKind};

impl WorkflowEngine {
    /// Liefert alle Incidents einer Instanz (aktiv oder archiviert).
    pub async fn get_instance_incidents(&self, instance_id: Uuid) -> EngineResult<Vec<Incident>> {
        Ok(self.get_instance_or_archived(instance_id).await?.incidents)
    }

    /// Löst einen Incident manuell auf und persistiert die Instanz.
    ///
    /// `resolved_by` ist optional (z. B. ein Admin-Benutzer). Ein bereits
    /// aufgelöster Incident wird nicht erneut verändert.
    pub async fn resolve_instance_incident(
        &self,
        instance_id: Uuid,
        incident_id: Uuid,
        resolved_by: Option<String>,
    ) -> EngineResult<Incident> {
        let Some(inst_arc) = self.instances.get(&instance_id).await else {
            return Err(EngineError::NoSuchInstance(instance_id));
        };

        let resolved = {
            let mut inst = inst_arc.write().await;
            let Some(incident) = inst
                .incidents
                .iter_mut()
                .find(|incident| incident.id == incident_id)
            else {
                return Err(EngineError::IncidentNotFound(incident_id));
            };
            if incident.resolved_at.is_some() {
                return Err(EngineError::IncidentAlreadyResolved(incident_id));
            }
            incident.resolved_at = Some(Utc::now());
            incident.resolved_by = resolved_by;
            incident.clone()
        };

        self.persist_instance(instance_id).await;
        Ok(resolved)
    }

    /// Legt einen Incident an der Instanz an oder zählt ein offenes Duplikat hoch.
    ///
    /// Liefert `None`, wenn die Instanz nicht (mehr) existiert. `bool` im Tupel
    /// ist `true`, wenn ein neuer Incident entstanden ist (für bedarfsgesteuerte
    /// Persistenz).
    pub(crate) async fn record_incident(
        &self,
        instance_id: Uuid,
        kind: IncidentKind,
        node_id: Option<String>,
        task_id: Option<Uuid>,
        message: String,
        details: Option<String>,
    ) -> Option<(Uuid, bool)> {
        record_incident_on(
            &self.instances,
            instance_id,
            kind,
            node_id,
            task_id,
            message,
            details,
        )
        .await
    }
}

/// Wie [`WorkflowEngine::record_incident`], aber für Aufrufer ohne Engine-Referenz
/// (z. B. den Persistenz-Retry-Worker).
pub(crate) async fn record_incident_on(
    instances: &InstanceStore,
    instance_id: Uuid,
    kind: IncidentKind,
    node_id: Option<String>,
    task_id: Option<Uuid>,
    message: String,
    details: Option<String>,
) -> Option<(Uuid, bool)> {
    let inst_arc = instances.get(&instance_id).await?;
    let mut inst = inst_arc.write().await;

    let already_open = inst.incidents.iter().any(|incident| {
        incident.resolved_at.is_none()
            && incident.kind == kind
            && incident.node_id == node_id
            && incident.task_id == task_id
    });

    let id = inst.push_or_touch_incident(kind.clone(), node_id, task_id, message, details);
    metrics::counter!("bpmn_incidents_total", "kind" => kind.as_str()).increment(1);

    Some((id, !already_open))
}
