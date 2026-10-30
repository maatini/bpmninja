//! Versionsgepinntes Routing und Empfänger-Vorprüfung.
//! Hier wird keine Nachricht zugestellt. Ein Plan ist eine Beobachtung;
//! eine spätere Zustellung muss den Empfänger erneut prüfen und beanspruchen.
use super::WorkflowEngine;
use crate::{
    BpmnElement, Collaboration, DefinitionCollection, EngineError, EngineResult, InstanceState,
    MessageEndpoint, ProcessDefinition,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployedMessageEndpoint {
    pub definition_key: Uuid,
    pub version: i32,
    pub participant_id: String,
    pub process_id: String,
    pub element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedMessageRoute {
    pub collection_id: Uuid,
    pub collaboration_id: String,
    pub flow_id: String,
    pub message_id: String,
    pub message_name: String,
    pub source: DeployedMessageEndpoint,
    pub target: DeployedMessageEndpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiverUnavailable {
    DefinitionDeleted,
    NoMatchingCatch,
    AmbiguousCatches { count: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlannedMessageReceiver {
    Start,
    Catch {
        instance_id: Uuid,
        catch_id: Uuid,
        token_id: Uuid,
    },
    Unavailable(ReceiverUnavailable),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalMessagePlan {
    pub route: PinnedMessageRoute,
    pub business_key: String,
    /// Expliziter Snapshot des Senders. Input-Mappings des Ziels laufen erst bei der Zustellung.
    pub variables: HashMap<String, Value>,
    pub receiver: PlannedMessageReceiver,
}

fn invalid(message: impl Into<String>) -> EngineError {
    EngineError::InvalidDefinition(format!("Internal message routing: {}", message.into()))
}

fn pin_endpoint(
    collection: &DefinitionCollection,
    collaboration: &Collaboration,
    endpoint: &MessageEndpoint,
    message: &str,
    source: bool,
) -> EngineResult<DeployedMessageEndpoint> {
    if !collaboration.participants.iter().any(|participant| {
        participant.id == endpoint.participant_id
            && participant.executable
            && participant.process_id.as_deref() == Some(&endpoint.process_id)
    }) {
        return Err(invalid("endpoint is not an executable participant"));
    }
    let definition = collection
        .definitions
        .iter()
        .find(|definition| definition.id == endpoint.process_id)
        .ok_or_else(|| invalid(format!("missing process '{}'", endpoint.process_id)))?;
    let node = definition
        .nodes
        .get(&endpoint.element_id)
        .ok_or_else(|| invalid("missing endpoint element"))?;
    let valid = match node {
        BpmnElement::SendTask {
            message_name,
            multi_instance,
        } => source && multi_instance.is_none() && message_name == message,
        BpmnElement::MessageStartEvent { message_name }
        | BpmnElement::MessageCatchEvent { message_name }
        | BpmnElement::BoundaryMessageEvent { message_name, .. } => {
            !source && message_name == message
        }
        _ => false,
    };
    let multi_instance_host = match node {
        BpmnElement::BoundaryMessageEvent { attached_to, .. } => matches!(
            definition.nodes.get(attached_to),
            Some(
                BpmnElement::ServiceTask {
                    multi_instance: Some(_),
                    ..
                } | BpmnElement::ScriptTask {
                    multi_instance: Some(_),
                    ..
                }
            )
        ),
        _ => false,
    };
    if !valid || multi_instance_host {
        return Err(invalid("unsupported or mismatched endpoint"));
    }
    Ok(DeployedMessageEndpoint {
        definition_key: definition.key,
        version: definition.version,
        participant_id: endpoint.participant_id.clone(),
        process_id: endpoint.process_id.clone(),
        element_id: endpoint.element_id.clone(),
    })
}

/// Routen aus dem committeten Collection-Datensatz. Nie die neueste Version.
pub(super) fn compile_routes(
    collection: &DefinitionCollection,
) -> EngineResult<Vec<PinnedMessageRoute>> {
    let collaborations: Vec<Collaboration> =
        serde_json::from_value(collection.collaborations.clone())
            .map_err(|err| invalid(format!("invalid stored collaboration metadata: {err}")))?;
    let mut definitions = HashSet::new();
    let mut keys = HashSet::new();
    for definition in &collection.definitions {
        if !definitions.insert(&definition.id) || !keys.insert(definition.key) {
            return Err(invalid("duplicate collection definition"));
        }
    }
    let mut routes = Vec::new();
    let mut ids = HashSet::new();
    for collaboration in collaborations {
        if collaboration.id.trim().is_empty() || !ids.insert(collaboration.id.clone()) {
            return Err(invalid("invalid collaboration ID"));
        }
        let mut participants = HashSet::new();
        let mut processes = HashSet::new();
        for participant in &collaboration.participants {
            if participant.id.trim().is_empty()
                || !ids.insert(participant.id.clone())
                || !participants.insert(&participant.id)
                || participant
                    .process_id
                    .as_ref()
                    .is_some_and(|id| !processes.insert(id))
            {
                return Err(invalid("duplicate or empty participant"));
            }
        }
        for flow in &collaboration.message_flows {
            if flow.id.trim().is_empty()
                || !ids.insert(flow.id.clone())
                || flow.message_id.trim().is_empty()
                || flow.message_name.trim().is_empty()
            {
                return Err(invalid("invalid flow identity"));
            }
            let source = pin_endpoint(
                collection,
                &collaboration,
                &flow.source,
                &flow.message_name,
                true,
            )?;
            let target = pin_endpoint(
                collection,
                &collaboration,
                &flow.target,
                &flow.message_name,
                false,
            )?;
            if source.definition_key == target.definition_key {
                return Err(invalid("flow endpoints must be in distinct pools"));
            }
            routes.push(PinnedMessageRoute {
                collection_id: collection.id,
                collaboration_id: collaboration.id.clone(),
                flow_id: flow.id.clone(),
                message_id: flow.message_id.clone(),
                message_name: flow.message_name.clone(),
                source,
                target,
            });
        }
    }
    Ok(routes)
}

impl WorkflowEngine {
    pub(super) fn install_routes(&self, routes: Vec<PinnedMessageRoute>) {
        let mut index = self
            .internal_routes
            .write()
            .unwrap_or_else(|err| err.into_inner());
        let sources: HashSet<_> = routes
            .iter()
            .map(|route| route.source.definition_key)
            .collect();
        for source in sources {
            index.insert(
                source,
                routes
                    .iter()
                    .filter(|route| route.source.definition_key == source)
                    .cloned()
                    .collect(),
            );
        }
    }

    /// Nebenwirkungsfreie Vorprüfung der expliziten ausgehenden Flows.
    /// Ein nicht leerer Business-Key ist Pflicht. Jeder explizite Flow liefert
    /// genau ein Ergebnis, in Dokumentreihenfolge. Kein Namens-Broadcast,
    /// kein Fallback auf die neueste Version, keine Empfänger-Reservierung
    /// und kein automatisches Senden.
    pub async fn plan_internal_messages(
        &self,
        source_definition: Uuid,
        source_element: &str,
        business_key: &str,
        variables: HashMap<String, Value>,
    ) -> EngineResult<Vec<InternalMessagePlan>> {
        let _guard = self.deployment_lock.lock().await;
        if business_key.trim().is_empty() {
            return Err(invalid("a nonempty business key is required"));
        }
        let definition = self
            .definitions
            .get(&source_definition)
            .ok_or(EngineError::NoSuchDefinition(source_definition))?;
        if !matches!(
            definition.nodes.get(source_element),
            Some(BpmnElement::SendTask { .. })
        ) {
            return Err(invalid("source element is not a message send"));
        }
        let routes: Vec<_> = self
            .internal_routes
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .get(&source_definition)
            .into_iter()
            .flatten()
            .filter(|route| route.source.element_id == source_element)
            .cloned()
            .collect();
        let mut plans = Vec::new();
        for route in routes {
            let receiver = match self.definitions.get(&route.target.definition_key) {
                None => PlannedMessageReceiver::Unavailable(ReceiverUnavailable::DefinitionDeleted),
                Some(target)
                    if matches!(
                        target.nodes.get(&route.target.element_id),
                        Some(BpmnElement::MessageStartEvent { .. })
                    ) =>
                {
                    PlannedMessageReceiver::Start
                }
                Some(target) => {
                    self.select_message_receiver(&route, &target, business_key)
                        .await
                }
            };
            plans.push(InternalMessagePlan {
                route,
                business_key: business_key.into(),
                variables: variables.clone(),
                receiver,
            });
        }
        Ok(plans)
    }

    async fn select_message_receiver(
        &self,
        route: &PinnedMessageRoute,
        target: &ProcessDefinition,
        business_key: &str,
    ) -> PlannedMessageReceiver {
        // Snapshot vor dem Await: keine DashMap-Guard über den Await halten.
        let candidates: Vec<_> = self
            .pending_message_catches
            .iter()
            .filter(|catch| {
                catch.node_id == route.target.element_id && catch.message_name == route.message_name
            })
            .map(|catch| catch.value().clone())
            .collect();
        let mut selected = Vec::new();
        for catch in candidates {
            let Some(instance) = self.instances.get(&catch.instance_id).await else {
                continue;
            };
            let instance = instance.read().await;
            if instance.definition_key != route.target.definition_key
                || instance.business_key != business_key
                || matches!(
                    instance.state,
                    InstanceState::Suspended { .. }
                        | InstanceState::Completed
                        | InstanceState::CompletedWithError { .. }
                )
            {
                continue;
            }
            let Some(token) = instance.tokens.get(&catch.token_id) else {
                continue;
            };
            let expected = match target.nodes.get(&catch.node_id) {
                Some(BpmnElement::BoundaryMessageEvent { attached_to, .. }) => attached_to.as_str(),
                _ => catch.node_id.as_str(),
            };
            let waiting_at_gateway = matches!(
                target.nodes.get(&token.current_node),
                Some(BpmnElement::EventBasedGateway)
            ) && target
                .flows
                .get(&token.current_node)
                .is_some_and(|flows| flows.iter().any(|flow| flow.target == catch.node_id));
            if token.current_node != expected && !waiting_at_gateway {
                continue;
            }
            selected.push(PlannedMessageReceiver::Catch {
                instance_id: catch.instance_id,
                catch_id: catch.id,
                token_id: catch.token_id,
            });
        }
        match selected.len() {
            0 => PlannedMessageReceiver::Unavailable(ReceiverUnavailable::NoMatchingCatch),
            1 => selected
                .pop()
                .unwrap_or(PlannedMessageReceiver::Unavailable(
                    ReceiverUnavailable::NoMatchingCatch,
                )),
            count => {
                PlannedMessageReceiver::Unavailable(ReceiverUnavailable::AmbiguousCatches { count })
            }
        }
    }
}
