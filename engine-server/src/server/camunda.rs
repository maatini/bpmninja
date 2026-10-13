//! Camunda-7-External-Task-Vertrag über der JSON-nativen Engine.
#[cfg(test)]
mod tests;
mod variables;

use super::state::AppState;
use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{TimeDelta, Utc};
use engine_core::{PendingServiceTask, error::EngineError};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;
use variables::{TypedVariable, decode_variables, encode_variable};

#[derive(Debug)]
pub(super) struct CamundaError(StatusCode, String);
impl CamundaError {
    fn bad(message: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, message.into())
    }
}
impl IntoResponse for CamundaError {
    fn into_response(self) -> Response {
        let kind = if self.0 == StatusCode::NOT_FOUND {
            "RestException"
        } else if self.0.is_server_error() {
            "ProcessEngineException"
        } else {
            "InvalidRequestException"
        };
        (
            self.0,
            Json(json!({"type": kind, "message": self.1, "code": 0})),
        )
            .into_response()
    }
}
impl From<EngineError> for CamundaError {
    fn from(error: EngineError) -> Self {
        let status = match &error {
            EngineError::ServiceTaskNotFound(_)
            | EngineError::NoSuchInstance(_)
            | EngineError::NoSuchDefinition(_) => StatusCode::NOT_FOUND,
            EngineError::InvalidDefinition(_)
            | EngineError::ServiceTaskNotLocked(_)
            | EngineError::ServiceTaskLocked { .. }
            | EngineError::InstanceSuspended(_)
            | EngineError::AlreadyCompleted => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self(status, error.to_string())
    }
}
fn body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, CamundaError> {
    payload.map(|Json(value)| value).map_err(|error| {
        CamundaError(
            if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
                error.status()
            } else {
                StatusCode::BAD_REQUEST
            },
            error.body_text(),
        )
    })
}
fn worker(id: &str) -> Result<(), CamundaError> {
    if id.is_empty() || id.chars().count() > 255 {
        return Err(CamundaError::bad("workerId must contain 1–255 characters"));
    }
    Ok(())
}
fn duration(ms: i64, allow_zero: bool) -> Result<(), CamundaError> {
    if ms < 0
        || (!allow_zero && ms == 0)
        || Utc::now()
            .checked_add_signed(TimeDelta::milliseconds(ms))
            .is_none()
    {
        return Err(CamundaError::bad(
            "duration must be nonnegative and within the date range (locks must be positive)",
        ));
    }
    Ok(())
}
fn unsupported(extra: &HashMap<String, Value>) -> Result<(), CamundaError> {
    if let Some(name) = extra
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(key, _)| key)
        .min()
    {
        return Err(CamundaError::bad(format!("Unsupported option: {name}")));
    }
    Ok(())
}
fn id(raw: &str) -> Result<Uuid, CamundaError> {
    Uuid::parse_str(raw).map_err(|_| CamundaError::bad("Invalid task UUID"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Topic {
    topic_name: String,
    lock_duration: i64,
    variables: Option<HashSet<String>>,
    #[serde(default, rename = "deserializeValues")]
    _deserialize_values: bool,
    #[serde(default)]
    include_extension_properties: bool,
    #[serde(default)]
    local_variables: bool,
    #[serde(default)]
    without_tenant_id: bool,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FetchRequest {
    worker_id: String,
    max_tasks: usize,
    #[serde(default)]
    topics: Vec<Topic>,
    async_response_timeout: Option<u64>,
    #[serde(default, rename = "usePriority")]
    _use_priority: bool,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

pub(super) async fn fetch(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<FetchRequest>, JsonRejection>,
) -> Result<Json<Vec<Value>>, CamundaError> {
    let payload = body(payload)?;
    worker(&payload.worker_id)?;
    unsupported(&payload.extra)?;
    if payload.max_tasks == 0 || payload.max_tasks > i32::MAX as usize {
        return Err(CamundaError::bad(
            "maxTasks must be a positive 32-bit integer",
        ));
    }
    let timeout = payload.async_response_timeout.unwrap_or(0);
    if timeout > 1_800_000 {
        return Err(CamundaError::bad(
            "asyncResponseTimeout must not exceed 1800000ms",
        ));
    }
    let mut names = HashSet::new();
    for topic in &payload.topics {
        duration(topic.lock_duration, false)?;
        unsupported(&topic.extra)?;
        if topic.topic_name.is_empty() || !names.insert(&topic.topic_name) {
            return Err(CamundaError::bad(
                "topics must have unique, nonempty topicName values",
            ));
        }
        if topic.local_variables || topic.include_extension_properties {
            return Err(CamundaError::bad(
                "Local scopes and extension properties are not supported",
            ));
        }
        // Every BPMNinja task is tenantless; withoutTenantId is therefore satisfied.
        let _ = topic.without_tenant_id;
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout);
    loop {
        let notified = state.engine.service_tasks_changed();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let mut result = Vec::new();
        for topic in &payload.topics {
            let tasks = state
                .engine
                .fetch_and_lock_service_tasks_ms(
                    &payload.worker_id,
                    payload.max_tasks - result.len(),
                    std::slice::from_ref(&topic.topic_name),
                    topic.lock_duration,
                )
                .await;
            for task in tasks {
                result.push(task_dto(&state, task, topic.variables.as_ref()).await?);
            }
            if result.len() == payload.max_tasks {
                break;
            }
        }
        let now = tokio::time::Instant::now();
        if !result.is_empty() || payload.topics.is_empty() || now >= deadline {
            return Ok(Json(result));
        }
        tokio::select! {
            _ = &mut notified => {}
            _ = tokio::time::sleep((deadline - now).min(std::time::Duration::from_millis(500))) => {}
        }
    }
}

async fn task_dto(
    state: &AppState,
    task: PendingServiceTask,
    selected: Option<&HashSet<String>>,
) -> Result<Value, CamundaError> {
    let instance = state.engine.get_instance_details(task.instance_id).await?;
    let definition = state.engine.get_definition(&task.definition_key).await;
    let variables: HashMap<String, Value> = instance
        .variables
        .iter()
        .filter(|(name, _)| selected.is_none_or(|names| names.contains(*name)))
        .map(|(name, value)| {
            (
                name.clone(),
                encode_variable(value, instance.variable_types.get(name).map(String::as_str)),
            )
        })
        .collect();
    Ok(json!({
        "id": task.id, "activityId": task.node_id, "activityInstanceId": task.token_id,
        "processInstanceId": task.instance_id, "processDefinitionId": task.definition_key,
        "processDefinitionKey": definition.map(|definition| definition.id.clone()),
        "processDefinitionVersionTag": null, "topicName": task.topic, "workerId": task.worker_id,
        "lockExpirationTime": task.lock_expiration.map(|date| date.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string()),
        "retries": task.retries, "errorMessage": task.error_message, "errorDetails": task.error_details,
        "businessKey": instance.business_key, "tenantId": null, "priority": 0, "suspended": false,
        "variables": variables
    }))
}

async fn owned(state: &AppState, task_id: Uuid, worker_id: &str) -> Result<(), CamundaError> {
    worker(worker_id)?;
    let lease = state
        .engine
        .service_task_lock(task_id)
        .ok_or_else(|| CamundaError(StatusCode::NOT_FOUND, "External task not found".into()))?;
    if lease.worker_id.as_deref() != Some(worker_id)
        || lease.lock_expiration.is_none_or(|date| date <= Utc::now())
    {
        return Err(CamundaError::bad(
            "Worker does not own an active lock on this task",
        ));
    }
    if matches!(
        state.engine.get_instance_state(lease.instance_id).await?,
        engine_core::InstanceState::Suspended { .. }
    ) {
        return Err(CamundaError::bad("Process instance is suspended"));
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CompleteRequest {
    worker_id: String,
    variables: Option<HashMap<String, TypedVariable>>,
    local_variables: Option<HashMap<String, TypedVariable>>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}
pub(super) async fn complete(
    State(state): State<Arc<AppState>>,
    Path(raw): Path<String>,
    payload: Result<Json<CompleteRequest>, JsonRejection>,
) -> Result<StatusCode, CamundaError> {
    let payload = body(payload)?;
    unsupported(&payload.extra)?;
    if payload
        .local_variables
        .is_some_and(|variables| !variables.is_empty())
    {
        return Err(CamundaError::bad("Local variable scopes are not supported"));
    }
    let (values, types) = decode_variables(payload.variables.unwrap_or_default())?;
    let task = id(&raw)?;
    owned(&state, task, &payload.worker_id).await?;
    state
        .engine
        .complete_service_task_with_types(task, &payload.worker_id, values, types)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FailureRequest {
    worker_id: String,
    retries: Option<i32>,
    retry_timeout: Option<i64>,
    error_message: Option<String>,
    error_details: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}
pub(super) async fn failure(
    State(state): State<Arc<AppState>>,
    Path(raw): Path<String>,
    payload: Result<Json<FailureRequest>, JsonRejection>,
) -> Result<StatusCode, CamundaError> {
    let payload = body(payload)?;
    unsupported(&payload.extra)?;
    if payload.retries.is_some_and(|retries| retries < 0) {
        return Err(CamundaError::bad("retries must not be negative"));
    }
    let timeout = payload.retry_timeout.unwrap_or(0);
    duration(timeout, true)?;
    let task = id(&raw)?;
    owned(&state, task, &payload.worker_id).await?;
    state
        .engine
        .fail_service_task_with_retry_timeout(
            task,
            &payload.worker_id,
            payload.retries,
            payload.error_message,
            payload.error_details,
            timeout,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ExtendRequest {
    worker_id: String,
    new_duration: i64,
}
pub(super) async fn extend(
    State(state): State<Arc<AppState>>,
    Path(raw): Path<String>,
    payload: Result<Json<ExtendRequest>, JsonRejection>,
) -> Result<StatusCode, CamundaError> {
    let payload = body(payload)?;
    duration(payload.new_duration, false)?;
    let task = id(&raw)?;
    owned(&state, task, &payload.worker_id).await?;
    state
        .engine
        .extend_lock_ms(task, &payload.worker_id, payload.new_duration)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ErrorRequest {
    worker_id: String,
    error_code: String,
    error_message: Option<String>,
    variables: Option<HashMap<String, TypedVariable>>,
}
pub(super) async fn bpmn_error(
    State(state): State<Arc<AppState>>,
    Path(raw): Path<String>,
    payload: Result<Json<ErrorRequest>, JsonRejection>,
) -> Result<StatusCode, CamundaError> {
    let payload = body(payload)?;
    let (values, types) = decode_variables(payload.variables.unwrap_or_default())?;
    let task = id(&raw)?;
    owned(&state, task, &payload.worker_id).await?;
    state
        .engine
        .handle_bpmn_error_with_variables(
            task,
            &payload.worker_id,
            &payload.error_code,
            values,
            types,
            payload.error_message,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
