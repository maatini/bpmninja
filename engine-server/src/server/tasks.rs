use crate::server::state::{ApiJson, AppError, AppState, lock_duration_ms_to_secs, parse_uuid};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use engine_core::{MAX_SERVICE_TASK_LONG_POLL, PendingServiceTask, PendingUserTask};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub(crate) struct CompleteRequest {
    pub variables: Option<HashMap<String, Value>>,
}

pub(crate) async fn get_tasks(State(state): State<Arc<AppState>>) -> Json<Vec<PendingUserTask>> {
    let engine = &state.engine;
    let tasks = engine.get_pending_user_tasks_enriched().await;
    Json(tasks)
}

pub(crate) async fn get_service_tasks(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<PendingServiceTask>> {
    let engine = &state.engine;
    let tasks = engine.get_pending_service_tasks_enriched().await;
    Json(tasks)
}

pub(crate) async fn complete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<CompleteRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let task_id = parse_uuid(&id)?;
    let vars = payload.variables.unwrap_or_default();

    engine.complete_user_task(task_id, vars).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TopicRequest {
    pub topic_name: String,
    pub lock_duration: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchAndLockRequest {
    pub worker_id: String,
    pub max_tasks: usize,
    pub topics: Vec<TopicRequest>,
    pub async_response_timeout: Option<u64>,
}

/// Accepted bounds for `lockDuration` in the HTTP contract (milliseconds).
const MIN_LOCK_DURATION_MS: i64 = 1_000;
const MAX_LOCK_DURATION_MS: i64 = 3_600_000;
/// Accepted bounds for `maxTasks` and `topics` in a fetch-and-lock request.
const MAX_FETCH_TASKS: usize = 100;
const MAX_FETCH_TOPICS: usize = 32;
/// Re-check interval while long-polling, for locks that expire by time.
const LONG_POLL_FALLBACK_TICK: std::time::Duration = std::time::Duration::from_millis(500);

/// Validates a fetch-and-lock request and returns `(topic, lock seconds)` pairs.
///
/// Out-of-range values would otherwise reach `engine-core` (or `chrono`) and
/// either lock tasks for months or panic on extreme `TimeDelta`s.
fn validated_topics(payload: &FetchAndLockRequest) -> Result<Vec<(String, i64)>, AppError> {
    if payload.topics.is_empty() {
        return Err(AppError::BadRequest(
            "topics must contain at least one topic".to_string(),
        ));
    }
    if payload.topics.len() > MAX_FETCH_TOPICS {
        return Err(AppError::BadRequest(format!(
            "topics must contain at most {MAX_FETCH_TOPICS} entries, got {}",
            payload.topics.len()
        )));
    }
    if !(1..=MAX_FETCH_TASKS).contains(&payload.max_tasks) {
        return Err(AppError::BadRequest(format!(
            "maxTasks must be between 1 and {MAX_FETCH_TASKS}, got {}",
            payload.max_tasks
        )));
    }

    payload
        .topics
        .iter()
        .enumerate()
        .map(|(index, topic)| {
            let name = topic.topic_name.trim();
            if name.is_empty() {
                return Err(AppError::BadRequest(format!(
                    "topics[{index}].topicName must not be empty"
                )));
            }
            if !(MIN_LOCK_DURATION_MS..=MAX_LOCK_DURATION_MS).contains(&topic.lock_duration) {
                return Err(AppError::BadRequest(format!(
                    "topics[{index}].lockDuration must be between \
                     {MIN_LOCK_DURATION_MS} and {MAX_LOCK_DURATION_MS} ms, got {}",
                    topic.lock_duration
                )));
            }
            Ok((
                name.to_string(),
                lock_duration_ms_to_secs(topic.lock_duration),
            ))
        })
        .collect()
}

pub(crate) async fn fetch_and_lock_service_tasks(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<FetchAndLockRequest>,
) -> Result<Json<Vec<PendingServiceTask>>, AppError> {
    let topics = validated_topics(&payload)?;
    // Clamp before `Duration::from_millis`: an unclamped `u64::MAX` would panic.
    // Values above the engine cap are shortened, not rejected, so existing
    // workers that send a larger `asyncResponseTimeout` keep working.
    let max_timeout_ms = u64::try_from(MAX_SERVICE_TASK_LONG_POLL.as_millis()).unwrap_or(30_000);
    let timeout_ms = payload
        .async_response_timeout
        .unwrap_or(0)
        .min(max_timeout_ms);
    let timeout = std::time::Duration::from_millis(timeout_ms);

    // Woken when a task is inserted or unlocked. The tick only covers lock
    // expiries (time-based, not signalled). A zero timeout returns after one fetch.
    let tasks = state
        .engine
        .fetch_and_lock_service_tasks_long_poll(
            &payload.worker_id,
            payload.max_tasks,
            &topics,
            timeout,
            LONG_POLL_FALLBACK_TICK,
        )
        .await;
    Ok(Json(tasks))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompleteServiceTaskRequest {
    pub worker_id: String,
    pub variables: Option<HashMap<String, Value>>,
}

pub(crate) async fn complete_service_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<CompleteServiceTaskRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let task_id = parse_uuid(&id)?;
    let vars = payload.variables.unwrap_or_default();

    engine
        .complete_service_task(task_id, &payload.worker_id, vars)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FailServiceTaskRequest {
    pub worker_id: String,
    pub retries: Option<i32>,
    pub error_message: Option<String>,
    pub error_details: Option<String>,
}

pub(crate) async fn fail_service_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<FailServiceTaskRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let task_id = parse_uuid(&id)?;

    engine
        .fail_service_task(
            task_id,
            &payload.worker_id,
            payload.retries,
            payload.error_message,
            payload.error_details,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetryIncidentRequest {
    pub retries: Option<i32>,
}

pub(crate) async fn retry_incident(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<RetryIncidentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let task_id = parse_uuid(&id)?;

    engine.retry_incident(task_id, payload.retries).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ResolveIncidentRequest {
    pub variables: Option<HashMap<String, Value>>,
}

pub(crate) async fn resolve_incident(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<ResolveIncidentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let task_id = parse_uuid(&id)?;
    let vars = payload.variables.unwrap_or_default();

    engine.resolve_incident(task_id, vars).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExtendLockRequest {
    pub worker_id: String,
    pub new_duration: i64,
}

pub(crate) async fn extend_lock(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<ExtendLockRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let task_id = parse_uuid(&id)?;

    engine
        .extend_lock(
            task_id,
            &payload.worker_id,
            lock_duration_ms_to_secs(payload.new_duration),
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BpmnErrorRequest {
    pub worker_id: String,
    pub error_code: String,
}

pub(crate) async fn bpmn_error(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<BpmnErrorRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let task_id = parse_uuid(&id)?;

    engine
        .handle_bpmn_error(task_id, &payload.worker_id, &payload.error_code)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
