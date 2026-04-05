use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;
use engine_core::persistence::StorageInfo;
use crate::server::state::AppState;

#[derive(Serialize)]
pub(crate) struct BackendInfo {
    pub backend_type: String,
    pub nats_url: Option<String>,
    pub connected: bool,
}

#[derive(Serialize)]
pub(crate) struct MonitoringData {
    pub definitions_count: usize,
    pub instances_total: usize,
    pub instances_running: usize,
    pub instances_completed: usize,
    pub pending_user_tasks: usize,
    pub pending_service_tasks: usize,
    pub pending_timers: usize,
    pub pending_message_catches: usize,
    pub persistence_errors: u64,
    pub storage_info: Option<StorageInfo>,
}

pub(crate) async fn ready_endpoint(State(state): State<Arc<AppState>>) -> axum::response::Response {
    if let Some(ref p) = state.persistence {
        if p.get_storage_info().await.is_err() {
            return (axum::http::StatusCode::SERVICE_UNAVAILABLE, "NATS disconnected").into_response();
        }
    }
    (axum::http::StatusCode::OK, "Ready").into_response()
}

pub(crate) async fn get_backend_info(
    State(state): State<Arc<AppState>>,
) -> Json<BackendInfo> {
    if let Some(ref p) = state.persistence {
        let info = p.get_storage_info().await.ok().flatten();
        Json(BackendInfo {
            backend_type: "persistent".to_string(),
            nats_url: info.as_ref().map(|i| format!("{}:{}", i.host, i.port)),
            connected: true,
        })
    } else {
        Json(BackendInfo {
            backend_type: "in-memory".to_string(),
            nats_url: Some(state.nats_url.clone()),
            connected: false,
        })
    }
}

pub(crate) async fn get_monitoring_data(
    State(state): State<Arc<AppState>>,
) -> Json<MonitoringData> {
    let engine = &state.engine;

    let stats = engine.get_stats().await;

    let storage_info = if let Some(ref persistence) = state.persistence {
        persistence.get_storage_info().await.unwrap_or(None)
    } else {
        None
    };

    Json(MonitoringData {
        definitions_count: stats.definitions_count,
        instances_total: stats.instances_total,
        instances_running: stats.instances_running + stats.instances_waiting_user + stats.instances_waiting_service,
        instances_completed: stats.instances_completed,
        pending_user_tasks: stats.pending_user_tasks,
        pending_service_tasks: stats.pending_service_tasks,
        pending_timers: stats.pending_timers,
        pending_message_catches: stats.pending_message_catches,
        persistence_errors: stats.persistence_errors,
        storage_info,
    })
}
