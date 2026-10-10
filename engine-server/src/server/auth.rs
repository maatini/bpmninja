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

/// Optional API-key gate. No-op when `AppState.api_key` is `None`.
///
/// Public without a key: `GET /api/health`, `GET /api/ready`. CORS preflight
/// (`OPTIONS`) is always forwarded so the CORS layer can answer it.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let Some(expected) = state.api_key.as_deref() else {
        return Ok(next.run(request).await);
    };

    if request.method() == Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();
    if path == HEALTH_PATH || path == READY_PATH {
        return Ok(next.run(request).await);
    }

    match extract_api_key(request.headers()) {
        Some(provided) if api_keys_equal(provided, expected) => Ok(next.run(request).await),
        _ => Err(AppError::Unauthorized),
    }
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
    use super::strip_bearer;

    #[test]
    fn strip_bearer_accepts_bearer_and_trims() {
        assert_eq!(strip_bearer("Bearer secret"), Some("secret"));
        assert_eq!(strip_bearer("bearer secret"), Some("secret"));
        assert_eq!(strip_bearer("BEARER  secret  "), Some("secret"));
        assert_eq!(strip_bearer("Bearer"), None);
        assert_eq!(strip_bearer("Basic secret"), None);
        assert_eq!(strip_bearer("Bearer "), None);
    }
}
