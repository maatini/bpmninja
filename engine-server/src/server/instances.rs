use crate::server::state::{ApiJson, AppError, AppState, parse_uuid};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
};
use engine_core::{Incident, ProcessInstance};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub(crate) struct StartRequest {
    pub definition_key: String,
    /// Optional business key; also accepted inside `variables` (legacy clients).
    #[serde(default)]
    pub business_key: Option<String>,
    #[serde(default)]
    pub variables: Option<HashMap<String, Value>>,
}

/// The engine reads the business key from the variables map and removes it
/// before seeding instance variables. A top-level `business_key` wins.
fn with_business_key(
    mut variables: HashMap<String, Value>,
    business_key: Option<String>,
) -> HashMap<String, Value> {
    if let Some(key) = business_key.filter(|key| !key.trim().is_empty()) {
        variables.insert("business_key".to_string(), Value::String(key));
    }
    variables
}

#[derive(Serialize)]
pub(crate) struct StartResponse {
    pub instance_id: String,
}

pub(crate) async fn start_instance(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<StartRequest>,
) -> Result<Json<StartResponse>, AppError> {
    let engine = &state.engine;
    let def_key = parse_uuid(&payload.definition_key)?;
    let vars = with_business_key(payload.variables.unwrap_or_default(), payload.business_key);
    let id = engine.start_instance_with_variables(def_key, vars).await?;

    Ok(Json(StartResponse {
        instance_id: id.to_string(),
    }))
}

#[derive(Serialize, Deserialize)]
pub(crate) struct StartLatestRequest {
    /// BPMN process id. `bpmn_id` is still accepted for older clients.
    #[serde(alias = "bpmn_id")]
    pub bpmn_process_id: String,
    #[serde(default)]
    pub business_key: Option<String>,
    #[serde(default)]
    pub variables: Option<HashMap<String, Value>>,
}

#[derive(Serialize)]
pub(crate) struct StartLatestResponse {
    pub instance_id: String,
    pub definition_key: String,
    pub version: i32,
}

pub(crate) async fn start_instance_latest(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<StartLatestRequest>,
) -> Result<Json<StartLatestResponse>, AppError> {
    let engine = &state.engine;
    let vars = with_business_key(payload.variables.unwrap_or_default(), payload.business_key);
    let (inst_id, def_key) = engine
        .start_instance_latest(&payload.bpmn_process_id, vars)
        .await?;

    let def = engine
        .get_definition(&def_key)
        .await
        .ok_or_else(|| AppError::BadRequest("Definition not found".into()))?;
    let version = def.version;

    Ok(Json(StartLatestResponse {
        instance_id: inst_id.to_string(),
        definition_key: def_key.to_string(),
        version,
    }))
}

pub(crate) async fn start_timer_instance(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<StartRequest>,
) -> Result<Json<StartResponse>, AppError> {
    let engine = Arc::clone(&state.engine);
    let def_key = parse_uuid(&payload.definition_key)?;
    let vars = with_business_key(payload.variables.unwrap_or_default(), payload.business_key);
    let id = engine.start_timer_instance(def_key, vars).await?;

    Ok(Json(StartResponse {
        instance_id: id.to_string(),
    }))
}

#[derive(Deserialize, Default)]
pub(crate) struct ListInstancesQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub(crate) async fn list_instances(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListInstancesQuery>,
) -> impl IntoResponse {
    let engine = &state.engine;
    let paginated = query.limit.is_some() || query.offset.is_some();

    if !paginated {
        let instances = engine.list_instances().await;
        return Json(instances).into_response();
    }

    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.map(|l| l.clamp(1, 1000));
    let page = engine.list_instances_page(offset, limit).await;

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("x-total-count"),
        HeaderValue::from(page.total),
    );

    (headers, Json(page.items)).into_response()
}

pub(crate) async fn get_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProcessInstance>, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;

    let instance = engine.get_instance_details(instance_id).await?;

    Ok(Json(instance))
}

#[derive(Deserialize)]
pub(crate) struct UpdateVariablesRequest {
    pub variables: HashMap<String, Value>,
}

/// Liefert alle Incidents einer Instanz (offen und aufgelöst).
pub(crate) async fn get_instance_incidents(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Incident>>, AppError> {
    let instance_id = parse_uuid(&id)?;
    let incidents = state.engine.get_instance_incidents(instance_id).await?;
    Ok(Json(incidents))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolveInstanceIncidentRequest {
    pub resolved_by: Option<String>,
}

/// Löst einen Incident manuell auf (Admin-Rolle).
pub(crate) async fn resolve_instance_incident(
    State(state): State<Arc<AppState>>,
    Path((id, incident_id)): Path<(String, String)>,
    payload: Option<ApiJson<ResolveInstanceIncidentRequest>>,
) -> Result<Json<Incident>, AppError> {
    let instance_id = parse_uuid(&id)?;
    let incident_id = parse_uuid(&incident_id)?;
    let resolved_by = payload.and_then(|ApiJson(payload)| payload.resolved_by);

    let incident = state
        .engine
        .resolve_instance_incident(instance_id, incident_id, resolved_by)
        .await?;

    Ok(Json(incident))
}

pub(crate) async fn update_instance_variables(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<UpdateVariablesRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;

    engine
        .update_instance_variables(instance_id, payload.variables)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn suspend_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;
    engine.suspend_instance(instance_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn resume_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;
    engine.resume_instance(instance_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct MoveTokenRequest {
    pub target_node_id: String,
    #[serde(default)]
    pub variables: Option<HashMap<String, Value>>,
    #[serde(default = "default_true")]
    pub cancel_current: bool,
}

fn default_true() -> bool {
    true
}

pub(crate) async fn move_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<MoveTokenRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;

    engine
        .move_token(
            instance_id,
            &payload.target_node_id,
            payload.variables.unwrap_or_default(),
            payload.cancel_current,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct MigrateInstanceRequest {
    pub target_definition_key: String,
    #[serde(default)]
    pub node_mapping: HashMap<String, String>,
}

pub(crate) async fn migrate_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(payload): ApiJson<MigrateInstanceRequest>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;
    let target_key = parse_uuid(&payload.target_definition_key)?;

    engine
        .migrate_instance(instance_id, target_key, payload.node_mapping)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn delete_instance(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;

    engine.delete_instance(instance_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
