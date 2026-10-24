use crate::server::state::{ApiJson, AppError, AppState};
use axum::{Json, extract::State};
use engine_core::runtime::PendingSignalCatch;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorrelateSignalRequest {
    pub signal_name: String,
    pub variables: Option<HashMap<String, Value>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CorrelateSignalResponse {
    pub affected_instances: Vec<String>,
}

/// Broadcastet ein Signal an alle wartenden Catches und Signal-Start-Events.
pub(crate) async fn correlate_signal(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<CorrelateSignalRequest>,
) -> Result<Json<CorrelateSignalResponse>, AppError> {
    let engine = &state.engine;
    let vars = payload.variables.unwrap_or_default();
    let affected = engine.correlate_signal(payload.signal_name, vars).await?;
    let affected_strs = affected.into_iter().map(|id| id.to_string()).collect();
    Ok(Json(CorrelateSignalResponse {
        affected_instances: affected_strs,
    }))
}

/// Returns all currently pending signal catch events.
pub(crate) async fn get_pending_signals(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<PendingSignalCatch>> {
    Json(state.engine.get_pending_signal_catches())
}
