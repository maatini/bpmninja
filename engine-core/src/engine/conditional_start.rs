//! Explicit, durable conditional starts. Request identity uses structural JSON equality.
use super::WorkflowEngine;
use crate::domain::{BpmnElement, EngineError, EngineResult, ProcessDefinition, Token};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalStartRequest {
    pub request_id: Uuid,
    pub definition_key: Option<Uuid>,
    pub process_id: Option<String>,
    pub business_key: Option<String>,
    pub variables: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalStartResult {
    pub request_id: Uuid,
    pub definition_key: Uuid,
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<Uuid>,
}

/// Retained indefinitely. No TTL: a false result and a completed instance remain retryable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalStartReceipt {
    pub version: u32,
    pub request: ConditionalStartRequest,
    pub result: ConditionalStartResult,
    pub initial_variables: HashMap<String, Value>,
    pub business_key: String,
    pub dispatched: bool,
}

pub(crate) fn validate(definition: &ProcessDefinition) -> EngineResult<()> {
    for (id, node) in &definition.nodes {
        if let BpmnElement::ConditionalStartEvent { condition } = node {
            crate::condition::validate_event_condition(condition)?;
            if definition
                .event_subprocesses
                .iter()
                .any(|h| h.node_ids.contains(id))
                || definition.start_event().map(|(node, _)| node) != Some(id.as_str())
                || definition
                    .flows
                    .values()
                    .flatten()
                    .any(|flow| flow.target == *id)
            {
                return Err(EngineError::InvalidDefinition(
                    "Conditional start must be the sole top-level start, without incoming flows"
                        .into(),
                ));
            }
        }
    }
    Ok(())
}

impl WorkflowEngine {
    pub async fn evaluate_conditional_start(
        &self,
        request: ConditionalStartRequest,
    ) -> EngineResult<ConditionalStartResult> {
        if request.request_id.is_nil()
            || request.definition_key.is_some() == request.process_id.is_some()
            || request
                .process_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
            || request.variables.contains_key("business_key")
        {
            return Err(EngineError::InvalidDefinition(
                "Provide a non-nil request_id, exactly one definition selector, and business_key outside variables".into(),
            ));
        }
        let _guard = self.conditional_start_locks.lock(request.request_id).await;
        let saved = if let Some(persistence) = &self.persistence {
            persistence
                .get_conditional_start(request.request_id)
                .await?
        } else {
            self.conditional_start_receipts
                .get(&request.request_id)
                .map(|r| r.clone())
        };
        let receipt = if let Some(saved) = saved {
            if saved.request != request {
                return Err(EngineError::ConditionalStartConflict(request.request_id));
            }
            saved
        } else {
            let key = match request.definition_key {
                Some(key) => key,
                None => self
                    .definitions
                    .find_latest_by_bpmn_id(request.process_id.as_deref().unwrap_or_default())
                    .map(|(key, _)| key)
                    .ok_or_else(|| {
                        EngineError::InvalidDefinition("No definition for process_id".into())
                    })?,
            };
            let definition = self
                .definitions
                .get(&key)
                .ok_or(EngineError::NoSuchDefinition(key))?;
            super::event_subprocess::validate_definition(&definition)?;
            let Some((_, BpmnElement::ConditionalStartEvent { condition })) =
                definition.start_event()
            else {
                return Err(EngineError::InvalidDefinition(
                    "Selected definition has no conditional start".into(),
                ));
            };
            let mut variables = definition.variable_defaults.clone();
            variables.extend(request.variables.clone());
            // Match ordinary start semantics: business_key is instance metadata.
            let default_business_key = variables
                .remove("business_key")
                .and_then(|v| v.as_str().map(str::to_owned));
            let matched = definition.evaluate_event_condition(condition, &variables);
            let receipt = ConditionalStartReceipt {
                version: 1,
                result: ConditionalStartResult {
                    request_id: request.request_id,
                    definition_key: key,
                    matched,
                    instance_id: matched.then(Uuid::new_v4),
                },
                business_key: request
                    .business_key
                    .clone()
                    .or(default_business_key)
                    .unwrap_or_else(|| Uuid::new_v4().to_string()),
                request,
                initial_variables: variables,
                dispatched: !matched,
            };
            if let Some(persistence) = &self.persistence {
                let stored = persistence.reserve_conditional_start(&receipt).await?;
                if stored.request != receipt.request {
                    return Err(EngineError::ConditionalStartConflict(
                        receipt.request.request_id,
                    ));
                }
                stored
            } else {
                self.conditional_start_receipts
                    .insert(receipt.request.request_id, receipt.clone());
                receipt
            }
        };
        self.dispatch_conditional_start(&receipt).await?;
        Ok(receipt.result)
    }

    async fn dispatch_conditional_start(
        &self,
        receipt: &ConditionalStartReceipt,
    ) -> EngineResult<()> {
        if receipt.version != 1 {
            return Err(EngineError::PersistenceError(
                "Unsupported conditional-start receipt version".into(),
            ));
        }
        let Some(id) = receipt.result.instance_id else {
            return Ok(());
        };
        if receipt.dispatched {
            return Ok(());
        }
        // Archive lookup must propagate storage failures: an outage cannot authorize a new start.
        if let Some(p) = &self.persistence
            && p.get_completed_instance(&id.to_string()).await?.is_some()
        {
            return self.finish_conditional_start(receipt).await;
        }
        if self.instances.get(&id).await.is_none() {
            let definition = self
                .definitions
                .get(&receipt.result.definition_key)
                .ok_or(EngineError::NoSuchDefinition(receipt.result.definition_key))?;
            let (start, _) = definition
                .start_event()
                .ok_or_else(|| EngineError::InvalidDefinition("Missing pinned start".into()))?;
            let mut instance = Self::build_initial_instance(
                definition.key,
                id,
                start,
                receipt.initial_variables.clone(),
                None,
            );
            instance.business_key = receipt.business_key.clone();
            self.instances.insert(id, instance).await;
            let mut token = Token::with_variables(start, receipt.initial_variables.clone());
            self.initialize_event_subprocesses(id, &mut token).await?;
            let instance = self
                .instances
                .get(&id)
                .await
                .ok_or(EngineError::NoSuchInstance(id))?;
            let mut instance = instance.write().await;
            let state = instance.event_state.as_mut().ok_or_else(|| {
                EngineError::PersistenceError("Missing conditional runtime state".into())
            })?;
            state.lifecycle_outbox.push(
                crate::history::HistoryEntry::new(
                    id,
                    crate::history::HistoryEventType::InstanceStarted,
                    "Conditional evaluation accepted",
                    crate::history::ActorType::Engine,
                    None,
                )
                .with_node(start)
                .with_context("request_id", serde_json::json!(receipt.request.request_id))
                .with_context("definition_key", serde_json::json!(definition.key)),
            );
            drop(instance);
        }
        // Fail closed before exposing work. A retry reuses the in-memory candidate.
        self.persist_instance_required(id).await?;
        self.reconcile_event_instance(id).await?;
        self.finish_conditional_start(receipt).await
    }

    async fn finish_conditional_start(
        &self,
        receipt: &ConditionalStartReceipt,
    ) -> EngineResult<()> {
        let mut receipt = receipt.clone();
        receipt.dispatched = true;
        if let Some(p) = &self.persistence {
            p.complete_conditional_start(&receipt).await?;
        } else {
            self.conditional_start_receipts
                .insert(receipt.request.request_id, receipt);
        }
        Ok(())
    }

    /// Call after definitions, snapshots and pending work load, before accepting evaluation traffic.
    pub async fn reconcile_conditional_starts(&self) -> EngineResult<()> {
        let receipts = if let Some(p) = &self.persistence {
            p.list_conditional_starts().await?
        } else {
            self.conditional_start_receipts
                .iter()
                .map(|r| r.clone())
                .collect()
        };
        for receipt in receipts {
            let _guard = self
                .conditional_start_locks
                .lock(receipt.request.request_id)
                .await;
            self.dispatch_conditional_start(&receipt).await?;
        }
        self.reconcile_event_subprocesses().await
    }
}
