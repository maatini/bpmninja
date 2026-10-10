use crate::log_buffer::LogBuffer;
use axum::{Json, http::StatusCode, response::IntoResponse};
use engine_core::WorkflowEngine;
use engine_core::error::EngineError;
use engine_core::persistence::WorkflowPersistence;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unified error type for all REST handlers.
pub(crate) enum AppError {
    /// Wraps an `EngineError` with automatic status-code mapping.
    Engine(EngineError),
    /// Client sent a malformed request (invalid UUID, bad XML, etc.).
    BadRequest(String),
    /// Request body / multipart payload exceeds configured limit.
    PayloadTooLarge(String),
    /// API key missing or invalid (`BPMNINJA_API_KEY` is set).
    Unauthorized,
}

impl From<EngineError> for AppError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, msg),
            Self::Engine(EngineError::InvalidDefinition(msg)) => (StatusCode::BAD_REQUEST, msg),
            Self::Engine(EngineError::NoMatchingCondition(msg)) => (
                StatusCode::BAD_REQUEST,
                format!("No matching condition at gateway '{msg}'"),
            ),
            Self::Engine(EngineError::NoSuchDefinition(id)) => {
                (StatusCode::NOT_FOUND, format!("Definition not found: {id}"))
            }
            Self::Engine(EngineError::NoSuchInstance(id)) => {
                (StatusCode::NOT_FOUND, format!("Instance not found: {id}"))
            }
            Self::Engine(EngineError::NoSuchNode(id)) => {
                (StatusCode::NOT_FOUND, format!("Node not found: {id}"))
            }
            Self::Engine(EngineError::ServiceTaskNotFound(id)) => (
                StatusCode::NOT_FOUND,
                format!("Service task not found: {id}"),
            ),
            Self::Engine(EngineError::TaskNotPending {
                task_id,
                actual_state,
            }) => (
                StatusCode::CONFLICT,
                format!("Task '{task_id}' is not pending (state: {actual_state})"),
            ),
            Self::Engine(EngineError::ServiceTaskLocked { task_id, worker_id }) => (
                StatusCode::CONFLICT,
                format!("Task '{task_id}' locked by worker '{worker_id}'"),
            ),
            Self::Engine(EngineError::ServiceTaskNotLocked(id)) => (
                StatusCode::CONFLICT,
                format!("Service task '{id}' is not locked"),
            ),
            Self::Engine(EngineError::AlreadyCompleted) => (
                StatusCode::CONFLICT,
                "Process instance already completed".to_string(),
            ),
            Self::Engine(EngineError::DefinitionHasInstances(count)) => (
                StatusCode::CONFLICT,
                format!("Cannot delete definition: {count} instances still exist"),
            ),
            Self::Engine(EngineError::InstanceSuspended(id)) => (
                StatusCode::CONFLICT,
                format!("Instance '{id}' is suspended"),
            ),
            Self::Engine(EngineError::OrphanedToken(node)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!(
                    "Orphaned token: node '{node}' not found in target definition — provide a mapping"
                ),
            ),
            Self::Engine(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
        };

        let body = serde_json::json!({ "error": message });
        (status, Json(body)).into_response()
    }
}

/// Parse a UUID from a path segment, returning `AppError::BadRequest` on failure.
pub(crate) fn parse_uuid(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest("Invalid UUID format".to_string()))
}

pub struct AppState {
    pub(crate) engine: Arc<WorkflowEngine>,
    pub(crate) persistence: Option<Arc<dyn WorkflowPersistence>>,
    pub(crate) deployed_xml: Arc<RwLock<HashMap<String, String>>>,
    pub(crate) nats_url: String,
    pub(crate) log_buffer: Arc<LogBuffer>,
    /// When true, `/api/ready` fails if no persistence backend is configured.
    pub(crate) require_nats: bool,
    /// Maximum multipart upload size in bytes (instance files).
    pub(crate) max_upload_bytes: usize,
    /// Maximum BPMN XML / JSON body size in bytes (`MAX_XML_BYTES`, default 10 MiB).
    pub(crate) max_xml_bytes: usize,
    /// When `Some`, every route except `/api/health` and `/api/ready` requires this key.
    pub(crate) api_key: Option<String>,
    /// Configured CORS origins (`*` means allow any — dev escape hatch only).
    pub(crate) cors_origins: Vec<String>,
}

/// `REQUIRE_NATS` is fail-closed: unset defaults to **true**.
/// Opt into silent in-memory mode with `false` / `0` / `no` / `off`.
pub fn require_nats_from_env() -> bool {
    parse_require_nats(std::env::var("REQUIRE_NATS").ok().as_deref())
}

pub(crate) fn parse_require_nats(value: Option<&str>) -> bool {
    match value {
        None => true,
        Some(raw) => !matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "no" | "off"
        ),
    }
}

#[cfg(test)]
mod require_nats_tests {
    use super::parse_require_nats;

    #[test]
    fn unset_is_fail_closed() {
        assert!(parse_require_nats(None));
    }

    #[test]
    fn explicit_false_opts_into_in_memory() {
        for v in ["false", "0", "no", "OFF", " False "] {
            assert!(!parse_require_nats(Some(v)), "{v}");
        }
    }

    #[test]
    fn explicit_true_stays_required() {
        for v in ["true", "1", "yes", "on"] {
            assert!(parse_require_nats(Some(v)), "{v}");
        }
    }
}

/// Default max upload size: 5 MiB (multipart instance files).
pub(crate) const DEFAULT_MAX_UPLOAD_BYTES: usize = 5 * 1024 * 1024;

/// Default JSON/XML body limit and BPMN deploy cap: 10 MiB (OpenAPI contract).
pub const MAX_XML_BYTES: usize = 10 * 1024 * 1024;

/// Default CORS allowlist used when `CORS_ORIGINS` is unset (never `Any`).
pub const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "http://localhost:1420",
    "http://127.0.0.1:1420",
    "tauri://localhost",
    "https://tauri.localhost",
];

/// Reads `MAX_UPLOAD_BYTES` from the environment, falling back to 5 MiB.
pub(crate) fn max_upload_bytes_from_env() -> usize {
    parse_positive_usize(
        std::env::var("MAX_UPLOAD_BYTES").ok().as_deref(),
        DEFAULT_MAX_UPLOAD_BYTES,
    )
}

/// Reads `MAX_XML_BYTES` from the environment, falling back to 10 MiB.
pub(crate) fn max_xml_bytes_from_env() -> usize {
    parse_positive_usize(
        std::env::var("MAX_XML_BYTES").ok().as_deref(),
        MAX_XML_BYTES,
    )
}

pub(crate) fn parse_positive_usize(value: Option<&str>, default: usize) -> usize {
    value
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
}

/// Reads `BPMNINJA_API_KEY`. Empty / unset → `None` (auth off).
pub(crate) fn api_key_from_env() -> Option<String> {
    parse_api_key(std::env::var("BPMNINJA_API_KEY").ok().as_deref())
}

pub(crate) fn parse_api_key(value: Option<&str>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Constant-time-ish compare without extra crates (length still leaks).
pub(crate) fn api_keys_equal(provided: &str, expected: &str) -> bool {
    let a = provided.as_bytes();
    let b = expected.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn default_cors_origins() -> Vec<String> {
    DEFAULT_CORS_ORIGINS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Reads `CORS_ORIGINS` (comma-separated). Unset/empty → [`default_cors_origins`].
/// Explicit `*` is preserved as a single origin (dev escape hatch for `Allow Any`).
pub(crate) fn cors_origins_from_env() -> Vec<String> {
    parse_cors_origins(std::env::var("CORS_ORIGINS").ok().as_deref())
}

pub(crate) fn parse_cors_origins(value: Option<&str>) -> Vec<String> {
    match value {
        None => default_cors_origins(),
        Some(raw) => {
            let origins: Vec<String> = raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            if origins.is_empty() {
                default_cors_origins()
            } else {
                origins
            }
        }
    }
}

/// Converts HTTP lock duration (milliseconds, Camunda/OpenAPI) to engine-core seconds.
/// `ms <= 0 → 0`, otherwise ceil: `(ms + 999) / 1000`.
pub(crate) fn lock_duration_ms_to_secs(ms: i64) -> i64 {
    if ms <= 0 {
        0
    } else {
        ms.saturating_add(999) / 1000
    }
}

#[cfg(test)]
mod config_parse_tests {
    use super::{
        MAX_XML_BYTES, api_keys_equal, default_cors_origins,
        lock_duration_ms_to_secs as ms_to_secs, parse_api_key, parse_cors_origins,
        parse_positive_usize,
    };

    #[test]
    fn default_body_limit_equals_max_xml_bytes() {
        assert_eq!(MAX_XML_BYTES, 10 * 1024 * 1024);
        assert_eq!(parse_positive_usize(None, MAX_XML_BYTES), MAX_XML_BYTES);
        assert_eq!(
            parse_positive_usize(Some("0"), MAX_XML_BYTES),
            MAX_XML_BYTES
        );
        assert_eq!(
            parse_positive_usize(Some("1048576"), MAX_XML_BYTES),
            1_048_576
        );
    }

    #[test]
    fn cors_unset_is_allowlist_not_any() {
        let origins = parse_cors_origins(None);
        assert_eq!(origins, default_cors_origins());
        assert!(!origins.iter().any(|o| o == "*"));
        assert_eq!(origins.len(), 4);
    }

    #[test]
    fn cors_empty_falls_back_to_defaults() {
        assert_eq!(parse_cors_origins(Some("")), default_cors_origins());
        assert_eq!(parse_cors_origins(Some("  ,  ")), default_cors_origins());
    }

    #[test]
    fn cors_star_is_explicit_any_escape() {
        assert_eq!(parse_cors_origins(Some("*")), vec!["*".to_string()]);
        assert_eq!(
            parse_cors_origins(Some(" http://a.example , http://b.example ")),
            vec![
                "http://a.example".to_string(),
                "http://b.example".to_string()
            ]
        );
    }

    #[test]
    fn api_key_empty_or_unset_is_off() {
        assert_eq!(parse_api_key(None), None);
        assert_eq!(parse_api_key(Some("")), None);
        assert_eq!(parse_api_key(Some("   ")), None);
        assert_eq!(parse_api_key(Some(" secret ")).as_deref(), Some("secret"));
    }

    #[test]
    fn api_keys_equal_rejects_mismatch() {
        assert!(api_keys_equal("abc", "abc"));
        assert!(!api_keys_equal("abc", "abd"));
        assert!(!api_keys_equal("abc", "ab"));
    }

    #[test]
    fn lock_duration_ms_to_secs() {
        assert_eq!(ms_to_secs(0), 0);
        assert_eq!(ms_to_secs(-5), 0);
        assert_eq!(ms_to_secs(1), 1);
        assert_eq!(ms_to_secs(1000), 1);
        assert_eq!(ms_to_secs(1500), 2);
        assert_eq!(ms_to_secs(30_000), 30);
    }
}
