use crate::log_buffer::LogBuffer;
use axum::{
    Json,
    extract::{FromRequest, OptionalFromRequest, Request, rejection::JsonRejection},
    http::StatusCode,
    response::IntoResponse,
};
use engine_core::WorkflowEngine;
use engine_core::error::EngineError;
use engine_core::persistence::WorkflowPersistence;
use serde::Serialize;
use serde::de::DeserializeOwned;
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
    /// API key missing or invalid (auth enabled or fail-closed).
    Unauthorized,
    /// Valid key, but the role is not allowed to use this route.
    Forbidden,
    /// Axum extractor rejection (invalid JSON, missing content type, oversized body).
    Rejected { status: StatusCode, message: String },
    /// Requested resource does not exist (e.g. no stored BPMN XML).
    NotFound(String),
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::Rejected {
            status: rejection.status(),
            message: rejection.body_text(),
        }
    }
}

/// JSON body extractor that renders rejections as `{ "error": "..." }`.
///
/// Axum's built-in `Json` rejection is plain text; the REST contract promises
/// the same error object for every failing request, including 400/413/415/422.
pub(crate) struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        <Json<T> as FromRequest<S>>::from_request(req, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(AppError::from)
    }
}

impl<T, S> OptionalFromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Option<Self>, Self::Rejection> {
        <Json<T> as OptionalFromRequest<S>>::from_request(req, state)
            .await
            .map(|value| value.map(|Json(payload)| Self(payload)))
            .map_err(AppError::from)
    }
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
            Self::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, msg),
            Self::Rejected { status, message } => (status, message),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
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
            Self::Engine(EngineError::IncidentNotFound(id)) => {
                (StatusCode::NOT_FOUND, format!("Incident not found: {id}"))
            }
            Self::Engine(EngineError::IncidentAlreadyResolved(id)) => (
                StatusCode::CONFLICT,
                format!("Incident already resolved: {id}"),
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

/// Zustand des Persistenz-Bootstraps, festgehalten beim Serverstart.
///
/// Der NATS-Bootstrap ist fail-closed: Schlägt er fehl und ist `REQUIRE_NATS`
/// aktiv, startet der Server nicht. Nur im expliziten In-Memory-Modus
/// (`REQUIRE_NATS=false`) läuft der Server mit [`StorageState::Failed`] weiter;
/// `/api/ready` liefert dann 503.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageStatus {
    pub state: StorageState,
    /// Menschenlesbare Details (Bootstrap-Zusammenfassung oder Fehlerursache).
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageState {
    /// Alle JetStream-Ressourcen wurden verifiziert oder angelegt.
    Ready,
    /// Bootstrap fehlgeschlagen; Server läuft ohne Persistenz weiter.
    Failed,
    /// Kein Persistenz-Backend konfiguriert (In-Memory-Modus).
    InMemory,
}

impl StorageStatus {
    pub fn ready(detail: impl Into<String>) -> Self {
        Self {
            state: StorageState::Ready,
            detail: detail.into(),
        }
    }

    pub fn failed(detail: impl Into<String>) -> Self {
        Self {
            state: StorageState::Failed,
            detail: detail.into(),
        }
    }

    pub fn in_memory(detail: impl Into<String>) -> Self {
        Self {
            state: StorageState::InMemory,
            detail: detail.into(),
        }
    }

    /// Fallback, wenn der Aufrufer keinen Bootstrap-Bericht mitgibt (Tests).
    pub(crate) fn inferred(has_persistence: bool) -> Self {
        if has_persistence {
            Self::ready("persistence attached without bootstrap report")
        } else {
            Self::in_memory("no persistence backend configured")
        }
    }
}

pub struct AppState {
    pub(crate) engine: Arc<WorkflowEngine>,
    pub(crate) persistence: Option<Arc<dyn WorkflowPersistence>>,
    pub(crate) deployed_xml: Arc<RwLock<HashMap<String, String>>>,
    /// Für API-Ausgaben maskierte NATS-URL (`nats://***@host:4222`, Query → `?***`);
    /// wird in `build_app_with_config` über [`crate::redact::redact_nats_url`] gesetzt.
    pub(crate) nats_url: String,
    pub(crate) log_buffer: Arc<LogBuffer>,
    /// Ergebnis des Persistenz-Bootstraps aus `main`.
    pub(crate) storage_status: StorageStatus,
    /// When true, `/api/ready` fails if no persistence backend is configured.
    pub(crate) require_nats: bool,
    /// Maximum multipart upload size in bytes (instance files).
    pub(crate) max_upload_bytes: usize,
    /// Maximum BPMN XML / JSON body size in bytes (`MAX_XML_BYTES`, default 10 MiB).
    pub(crate) max_xml_bytes: usize,
    /// When `Some`, admin key (`BPMNINJA_API_KEY`) with full access.
    pub(crate) api_key: Option<String>,
    /// When `Some`, worker key (`BPMNINJA_WORKER_API_KEY`) limited to service-task routes.
    pub(crate) worker_api_key: Option<String>,
    /// Whether the API-key gate is active. `true` also without keys when the
    /// server binds to a non-loopback address (fail-closed, see
    /// [`resolve_auth_required`]).
    pub(crate) auth_required: bool,
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

/// Reads `BPMNINJA_WORKER_API_KEY`. Empty / unset → `None`.
pub(crate) fn worker_api_key_from_env() -> Option<String> {
    parse_api_key(std::env::var("BPMNINJA_WORKER_API_KEY").ok().as_deref())
}

/// `BPMNINJA_ALLOW_UNAUTHENTICATED` — explicit dev escape hatch, default `false`.
pub(crate) fn allow_unauthenticated_from_env() -> bool {
    parse_allow_unauthenticated(
        std::env::var("BPMNINJA_ALLOW_UNAUTHENTICATED")
            .ok()
            .as_deref(),
    )
}

pub(crate) fn parse_allow_unauthenticated(value: Option<&str>) -> bool {
    match value {
        None => false,
        Some(raw) => matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
    }
}

/// Default listen host (Docker-compatible).
pub const DEFAULT_BIND_ADDR: &str = "0.0.0.0";

/// Reads `BIND_ADDR` (alias `BIND_ADDRESS`), default [`DEFAULT_BIND_ADDR`].
pub(crate) fn bind_addr_from_env() -> String {
    std::env::var("BIND_ADDR")
        .or_else(|_| std::env::var("BIND_ADDRESS"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string())
}

/// `true` for `127.0.0.0/8`, `::1` and `localhost` — anything else is unsafe
/// without authentication.
pub(crate) fn is_loopback_bind(host: &str) -> bool {
    let host = host.trim();
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return ip.is_loopback();
    }
    // Accept "host:port" forms as well.
    host.parse::<std::net::SocketAddr>()
        .is_ok_and(|addr| addr.ip().is_loopback())
}

/// Decides whether the API-key gate is active.
///
/// Fail-closed: without any key the gate stays active unless the server binds
/// to loopback, auth was explicitly disabled via [`AppBuildConfig`], or the
/// dev escape hatch `BPMNINJA_ALLOW_UNAUTHENTICATED` is set.
///
/// [`AppBuildConfig`]: crate::AppBuildConfig
pub(crate) fn resolve_auth_required(
    has_keys: bool,
    explicit_auth_off: bool,
    allow_unauthenticated: bool,
    bind_addr: &str,
) -> bool {
    if has_keys {
        return true;
    }
    if explicit_auth_off || allow_unauthenticated {
        return false;
    }
    !is_loopback_bind(bind_addr)
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
        MAX_XML_BYTES, api_keys_equal, default_cors_origins, is_loopback_bind,
        lock_duration_ms_to_secs as ms_to_secs, parse_allow_unauthenticated, parse_api_key,
        parse_cors_origins, parse_positive_usize, resolve_auth_required,
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

    #[test]
    fn allow_unauthenticated_requires_explicit_opt_in() {
        assert!(!parse_allow_unauthenticated(None));
        assert!(!parse_allow_unauthenticated(Some("")));
        assert!(!parse_allow_unauthenticated(Some("false")));
        for v in ["true", "1", "YES", " On "] {
            assert!(parse_allow_unauthenticated(Some(v)), "{v}");
        }
    }

    #[test]
    fn loopback_detection() {
        for host in [
            "127.0.0.1",
            "127.0.0.2",
            "::1",
            "[::1]",
            "localhost",
            "LOCALHOST",
        ] {
            assert!(is_loopback_bind(host), "{host}");
        }
        assert!(is_loopback_bind("127.0.0.1:8081"));
        for host in ["0.0.0.0", "::", "192.168.1.10", "example.com", ""] {
            assert!(!is_loopback_bind(host), "{host}");
        }
    }

    #[test]
    fn auth_is_fail_closed_without_keys() {
        // Non-loopback bind and no key → gate active (all /api → 401).
        assert!(resolve_auth_required(false, false, false, "0.0.0.0"));
        assert!(resolve_auth_required(false, false, false, "192.168.1.10"));
        // Loopback or explicit opt-outs keep auth off.
        assert!(!resolve_auth_required(false, false, false, "127.0.0.1"));
        assert!(!resolve_auth_required(false, true, false, "0.0.0.0"));
        assert!(!resolve_auth_required(false, false, true, "0.0.0.0"));
        // Configured keys always activate the gate.
        assert!(resolve_auth_required(true, false, false, "127.0.0.1"));
    }
}
