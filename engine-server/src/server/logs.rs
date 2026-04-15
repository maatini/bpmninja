//! GET /api/logs — gibt gefilterte Einträge aus dem In-Memory-Log-Buffer zurück.

use axum::{Json, extract::{Query, State}};
use serde::Deserialize;
use std::sync::Arc;

use crate::log_buffer::LogEntry;
use super::state::AppState;

#[derive(Deserialize)]
pub(crate) struct LogQuery {
    /// Mindest-Level: "error" | "warn" | "info" | "debug" | "trace"
    pub level: Option<String>,
    /// Substring-Filter auf Nachricht und Target (case-insensitive)
    pub search: Option<String>,
    /// Maximale Anzahl zurückgegebener Einträge (default: 500)
    pub limit: Option<usize>,
}

pub(crate) async fn get_logs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LogQuery>,
) -> Json<Vec<LogEntry>> {
    let limit = q.limit.unwrap_or(500).min(5_000);
    let mut entries = state
        .log_buffer
        .entries(q.level.as_deref(), q.search.as_deref());

    // Neueste zuerst, dann auf limit kürzen
    entries.reverse();
    entries.truncate(limit);

    Json(entries)
}
