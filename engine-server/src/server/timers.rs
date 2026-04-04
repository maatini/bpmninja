use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;
use crate::server::state::{AppError, AppState};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessTimersResponse {
    pub triggered: usize,
}

pub(crate) async fn process_timers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProcessTimersResponse>, AppError> {
    let mut engine = state.engine.write().await;
    let count = engine.process_timers().await?;
    Ok(Json(ProcessTimersResponse { triggered: count }))
}
