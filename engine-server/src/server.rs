use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use engine_core::engine::{ExternalTaskItem, PendingUserTask, ProcessInstance, WorkflowEngine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

struct AppState {
    engine: Arc<Mutex<WorkflowEngine>>,
}

#[derive(Serialize, Deserialize)]
struct DeployRequest {
    xml: String,
    name: String,
}

#[derive(Serialize)]
struct DeployResponse {
    definition_id: String,
}

#[derive(Serialize, Deserialize)]
struct StartRequest {
    definition_id: String,
    #[serde(default)]
    variables: Option<HashMap<String, Value>>,
}

#[derive(Serialize)]
struct StartResponse {
    instance_id: String,
}

#[derive(Serialize, Deserialize)]
struct CompleteRequest {
    variables: Option<HashMap<String, Value>>,
}

// ---------------------------------------------------------------------------
// External Task request/response types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TopicRequest {
    topic_name: String,
    lock_duration: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchAndLockRequest {
    worker_id: String,
    max_tasks: usize,
    topics: Vec<TopicRequest>,
    /// Optional timeout for long-polling (milliseconds).
    async_response_timeout: Option<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteExternalTaskRequest {
    worker_id: String,
    variables: Option<HashMap<String, Value>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FailExternalTaskRequest {
    worker_id: String,
    retries: Option<i32>,
    error_message: Option<String>,
    error_details: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtendLockRequest {
    worker_id: String,
    new_duration: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BpmnErrorRequest {
    worker_id: String,
    error_code: String,
}

/// Builds the Axum router with all routes and middleware.
///
/// Exposed as `pub` so integration tests can create the app without
/// starting a full server binary.
pub fn build_app() -> Router {
    let engine = WorkflowEngine::new();

    let state = Arc::new(AppState {
        engine: Arc::new(Mutex::new(engine)),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    Router::new()
        .route("/api/deploy", post(deploy_definition))
        .route("/api/start", post(start_instance))
        .route("/api/tasks", get(get_tasks))
        .route("/api/complete/:id", post(complete_task))
        .route("/api/instances", get(list_instances))
        .route("/api/instances/:id", get(get_instance))
        .route("/api/instances/:id/variables", put(update_instance_variables))
        // External Task endpoints
        .route("/api/external-task/fetchAndLock", post(fetch_and_lock))
        .route("/api/external-task/:id/complete", post(complete_external_task))
        .route("/api/external-task/:id/failure", post(fail_external_task))
        .route("/api/external-task/:id/extendLock", post(extend_lock))
        .route("/api/external-task/:id/bpmnError", post(bpmn_error))
        .layer(cors)
        .with_state(state)
}


async fn deploy_definition(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeployRequest>,
) -> Result<Json<DeployResponse>, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    
    let def = bpmn_parser::parse_bpmn_xml(&payload.xml)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid BPMN XML: {:?}", e)))?;
        
    let def_id = def.id.clone();
    engine.deploy_definition(def);
    
    Ok(Json(DeployResponse { definition_id: def_id }))
}

async fn start_instance(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartRequest>,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    let id = match payload.variables {
        Some(vars) if !vars.is_empty() => {
            engine
                .start_instance_with_variables(&payload.definition_id, vars)
                .await
        }
        _ => engine.start_instance(&payload.definition_id).await,
    }
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)))?;
        
    Ok(Json(StartResponse { instance_id: id.to_string() }))
}

async fn get_tasks(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<PendingUserTask>> {
    let engine = state.engine.lock().await;
    let tasks = engine.get_pending_user_tasks().to_vec();
    Json(tasks)
}

async fn complete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<CompleteRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid task ID format".to_string()))?;
        
    let vars = payload.variables.unwrap_or_default();
    
    engine
        .complete_user_task(task_id, vars)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)))?;
        
    Ok(StatusCode::NO_CONTENT)
}

async fn list_instances(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ProcessInstance>> {
    let engine = state.engine.lock().await;
    let instances = engine.list_instances();
    Json(instances)
}

async fn get_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProcessInstance>, (StatusCode, String)> {
    let engine = state.engine.lock().await;
    let instance_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid instance ID format".to_string()))?;
        
    let instance = engine
        .get_instance_details(instance_id)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("{:?}", e)))?;
        
    Ok(Json(instance))
}

#[derive(Deserialize)]
struct UpdateVariablesRequest {
    variables: HashMap<String, Value>,
}

async fn update_instance_variables(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateVariablesRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    let instance_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid instance ID format".to_string()))?;

    engine
        .update_instance_variables(instance_id, payload.variables)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("{:?}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}

// ---------------------------------------------------------------------------
// External Task REST handlers
// ---------------------------------------------------------------------------

/// POST /api/external-task/fetchAndLock
///
/// Long-polling variant: if `asyncResponseTimeout` is set, retries up to that
/// duration (polling every 500ms).
async fn fetch_and_lock(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FetchAndLockRequest>,
) -> Json<Vec<ExternalTaskItem>> {
    let topics: Vec<String> = payload.topics.iter().map(|t| t.topic_name.clone()).collect();
    // Use the first topic's lock duration, or default 30s
    let lock_duration = payload.topics.first().map(|t| t.lock_duration).unwrap_or(30);
    let timeout_ms = payload.async_response_timeout.unwrap_or(0);

    // Simple long-polling: retry until tasks found or timeout
    let start = tokio::time::Instant::now();
    loop {
        let mut engine = state.engine.lock().await;
        let tasks = engine.fetch_and_lock(
            &payload.worker_id,
            payload.max_tasks,
            &topics,
            lock_duration,
        );

        if !tasks.is_empty() || timeout_ms == 0 {
            return Json(tasks);
        }

        // Release lock before sleeping
        drop(engine);

        if start.elapsed().as_millis() as u64 >= timeout_ms {
            return Json(vec![]);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

/// POST /api/external-task/:id/complete
async fn complete_external_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<CompleteExternalTaskRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid task ID format".to_string()))?;

    let vars = payload.variables.unwrap_or_default();

    engine
        .complete_external_task(task_id, &payload.worker_id, vars)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/external-task/:id/failure
async fn fail_external_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<FailExternalTaskRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid task ID format".to_string()))?;

    engine
        .fail_external_task(
            task_id,
            &payload.worker_id,
            payload.retries,
            payload.error_message,
            payload.error_details,
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/external-task/:id/extendLock
async fn extend_lock(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<ExtendLockRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid task ID format".to_string()))?;

    engine
        .extend_lock(task_id, &payload.worker_id, payload.new_duration)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/external-task/:id/bpmnError
async fn bpmn_error(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<BpmnErrorRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut engine = state.engine.lock().await;
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid task ID format".to_string()))?;

    engine
        .handle_bpmn_error(task_id, &payload.worker_id, &payload.error_code)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

