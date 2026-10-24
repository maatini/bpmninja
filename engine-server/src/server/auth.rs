//! API-key gate with role separation (admin / worker).
//!
//! The gate is active when a key is configured **or** the server binds to a
//! non-loopback address without a key (fail-closed). Public without a key:
//! `GET /api/health`, `GET /api/ready`. CORS preflight (`OPTIONS`) is always
//! forwarded so the CORS layer can answer it.

use super::state::{AppError, AppState, api_keys_equal};
use axum::{
    extract::{Request, State},
    http::{Method, header},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

const HEALTH_PATH: &str = "/api/health";
const READY_PATH: &str = "/api/ready";
const SERVICE_TASK_PREFIX: &str = "/api/service-task/";

/// Role granted by a presented API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// `BPMNINJA_API_KEY` — full access.
    Admin,
    /// `BPMNINJA_WORKER_API_KEY` — service-task routes only.
    Worker,
}

/// Routes a worker key may call: fetch, complete, failure, extendLock, bpmnError.
fn is_worker_route(method: &Method, path: &str) -> bool {
    if method != Method::POST {
        return false;
    }
    if path == "/api/service-task/fetchAndLock" {
        return true;
    }
    let Some(rest) = path.strip_prefix(SERVICE_TASK_PREFIX) else {
        return false;
    };
    matches!(
        rest.split_once('/').map(|(_, action)| action),
        Some("complete" | "failure" | "extendLock" | "bpmnError")
    )
}

/// Resolves the role of the presented key, if any.
fn presented_role(state: &AppState, headers: &axum::http::HeaderMap) -> Option<Role> {
    let provided = extract_api_key(headers)?;
    // Admin wins if both keys are configured identically.
    if let Some(expected) = state.api_key.as_deref()
        && api_keys_equal(provided, expected)
    {
        return Some(Role::Admin);
    }
    if let Some(expected) = state.worker_api_key.as_deref()
        && api_keys_equal(provided, expected)
    {
        return Some(Role::Worker);
    }
    None
}

fn record_auth_failure(reason: &'static str, request: &Request) {
    tracing::warn!(
        method = %request.method(),
        path = request.uri().path(),
        reason,
        "API auth rejected request"
    );
    metrics::counter!("bpmn_auth_failures_total", "reason" => reason).increment(1);
}

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    if request.method() == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if path == HEALTH_PATH || path == READY_PATH {
        return Ok(next.run(request).await);
    }

    if !state.auth_required {
        return Ok(next.run(request).await);
    }

    let Some(role) = presented_role(&state, request.headers()) else {
        record_auth_failure("invalid_or_missing_key", &request);
        return Err(AppError::Unauthorized);
    };

    if role == Role::Worker && !is_worker_route(request.method(), path) {
        record_auth_failure("worker_role_forbidden", &request);
        return Err(AppError::Forbidden);
    }

    Ok(next.run(request).await)
}

fn extract_api_key(headers: &axum::http::HeaderMap) -> Option<&str> {
    if let Some(value) = headers.get(header::AUTHORIZATION)
        && let Ok(raw) = value.to_str()
        && let Some(key) = strip_bearer(raw)
    {
        return Some(key);
    }
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn strip_bearer(value: &str) -> Option<&str> {
    let value = value.trim();
    let scheme = "bearer";
    if value.len() > scheme.len()
        && value[..scheme.len()].eq_ignore_ascii_case(scheme)
        && value.as_bytes().get(scheme.len()) == Some(&b' ')
    {
        let key = value[scheme.len() + 1..].trim();
        if key.is_empty() { None } else { Some(key) }
    } else {
        None
    }
}

#[cfg(test)]
mod extract_tests {
    use super::{is_worker_route, strip_bearer};
    use axum::http::Method;

    #[test]
    fn strip_bearer_accepts_bearer_and_trims() {
        assert_eq!(strip_bearer("Bearer secret"), Some("secret"));
        assert_eq!(strip_bearer("bearer secret"), Some("secret"));
        assert_eq!(strip_bearer("BEARER  secret  "), Some("secret"));
        assert_eq!(strip_bearer("Bearer"), None);
        assert_eq!(strip_bearer("Basic secret"), None);
        assert_eq!(strip_bearer("Bearer "), None);
    }

    #[test]
    fn worker_routes_are_service_task_writes_only() {
        let post = Method::POST;
        assert!(is_worker_route(&post, "/api/service-task/fetchAndLock"));
        for action in ["complete", "failure", "extendLock", "bpmnError"] {
            assert!(is_worker_route(
                &post,
                &format!("/api/service-task/0b0e7d1e-0000-0000-0000-000000000000/{action}")
            ));
        }
    }

    #[test]
    fn admin_and_unrelated_routes_are_not_worker_routes() {
        assert!(!is_worker_route(&Method::POST, "/api/deploy"));
        assert!(!is_worker_route(&Method::POST, "/api/start"));
        assert!(!is_worker_route(&Method::GET, "/api/service-tasks"));
        assert!(!is_worker_route(&Method::GET, "/api/logs"));
        assert!(!is_worker_route(
            &Method::POST,
            "/api/service-task/abc/retry"
        ));
        assert!(!is_worker_route(
            &Method::POST,
            "/api/service-task/abc/resolve"
        ));
        assert!(!is_worker_route(
            &Method::POST,
            "/api/service-task/abc/complete/extra"
        ));
        assert!(!is_worker_route(&Method::OPTIONS, "/api/deploy"));
    }
}
