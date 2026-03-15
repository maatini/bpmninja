use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use engine_core::engine::{PendingUserTask, ProcessInstance, WorkflowEngine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
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
}

#[derive(Serialize)]
struct StartResponse {
    instance_id: String,
}

#[derive(Serialize, Deserialize)]
struct CompleteRequest {
    variables: Option<HashMap<String, Value>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initialize tracing
    // tracing_subscriber::fmt::init();

    let engine = WorkflowEngine::new();
    
    #[cfg(feature = "nats")]
    {
        // Placeholder: Initialize NATS persistence and wrapper
        // let client = async_nats::connect("nats://localhost:4222").await?;
        // let nats_persistence = Arc::new(persistence_nats::NatsPersistence::new(client).await?);
        // engine = engine.with_persistence(nats_persistence);
    }

    let state = Arc::new(AppState {
        engine: Arc::new(Mutex::new(engine)),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/deploy", post(deploy_definition))
        .route("/api/start", post(start_instance))
        .route("/api/tasks", get(get_tasks))
        .route("/api/complete/:id", post(complete_task))
        .route("/api/instances", get(list_instances))
        .route("/api/instances/:id", get(get_instance))
        .layer(cors)
        .with_state(state);

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("Server starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
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
    let id = engine
        .start_instance(&payload.definition_id)
        .await
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
