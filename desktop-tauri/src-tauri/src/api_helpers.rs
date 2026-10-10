use crate::state::AppState;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;

/// Builds `Authorization: Bearer` + `X-API-Key` from state. Empty/None → no headers.
pub fn auth_headers(state: &AppState) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    let Some(key) = crate::state::get_api_key(state)? else {
        return Ok(headers);
    };
    let bearer = HeaderValue::from_str(&format!("Bearer {key}"))
        .map_err(|e| format!("Invalid API key: {e}"))?;
    let x_api_key = HeaderValue::from_str(&key).map_err(|e| format!("Invalid API key: {e}"))?;
    headers.insert(AUTHORIZATION, bearer);
    headers.insert("X-API-Key", x_api_key);
    Ok(headers)
}

pub fn with_auth(
    builder: reqwest::RequestBuilder,
    state: &AppState,
) -> Result<reqwest::RequestBuilder, String> {
    Ok(builder.headers(auth_headers(state)?))
}

/// GET request helper - eliminates identical code blocks
pub async fn api_get(state: &AppState, path: &str) -> Result<Value, String> {
    let base = crate::state::get_base_url(state)?;
    let url = format!("{}{}", base, path);
    let res = with_auth(state.client.get(&url), state)?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let status = res.status();
        let body_text = res.text().await.unwrap_or_default();
        let message = serde_json::from_str::<serde_json::Value>(&body_text)
            .ok()
            .and_then(|v| v["error"].as_str().map(|s| s.to_string()))
            .unwrap_or(body_text);
        return Err(format!("Request failed: {} {}: {}", status, path, message));
    }
    res.json().await.map_err(|e| e.to_string())
}

/// POST request helper with JSON body
pub async fn api_post(state: &AppState, path: &str, body: &Value) -> Result<Value, String> {
    let base = crate::state::get_base_url(state)?;
    let url = format!("{}{}", base, path);
    let res = with_auth(state.client.post(&url).json(body), state)?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let status = res.status();
        let body_text = res.text().await.unwrap_or_default();
        return Err(format!(
            "Request failed: {} {}: {}",
            status, path, body_text
        ));
    }
    res.json().await.map_err(|e| e.to_string())
}

/// POST request returning no body (204 No Content)
pub async fn api_post_no_body(state: &AppState, path: &str, body: &Value) -> Result<(), String> {
    let base = crate::state::get_base_url(state)?;
    let url = format!("{}{}", base, path);
    let res = with_auth(state.client.post(&url).json(body), state)?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let status = res.status();
        let body_text = res.text().await.unwrap_or_default();
        // Extract "error" field from JSON if present
        let message = serde_json::from_str::<serde_json::Value>(&body_text)
            .ok()
            .and_then(|v| v["error"].as_str().map(|s| s.to_string()))
            .unwrap_or(body_text);
        return Err(format!("Request failed: {} {}: {}", status, path, message));
    }
    Ok(())
}

/// PUT request
pub async fn api_put(state: &AppState, path: &str, body: &Value) -> Result<(), String> {
    let base = crate::state::get_base_url(state)?;
    let url = format!("{}{}", base, path);
    let res = with_auth(state.client.put(&url).json(body), state)?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let status = res.status();
        let body_text = res.text().await.unwrap_or_default();
        let message = serde_json::from_str::<serde_json::Value>(&body_text)
            .ok()
            .and_then(|v| v["error"].as_str().map(|s| s.to_string()))
            .unwrap_or(body_text);
        return Err(format!("Request failed: {} {}: {}", status, path, message));
    }
    Ok(())
}

/// DELETE request
pub async fn api_delete(state: &AppState, path: &str) -> Result<(), String> {
    let base = crate::state::get_base_url(state)?;
    let url = format!("{}{}", base, path);
    let res = with_auth(state.client.delete(&url), state)?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let status = res.status();
        let body_text = res.text().await.unwrap_or_default();
        let message = serde_json::from_str::<serde_json::Value>(&body_text)
            .ok()
            .and_then(|v| v["error"].as_str().map(|s| s.to_string()))
            .unwrap_or(body_text);
        return Err(format!("Request failed: {} {}: {}", status, path, message));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(key: Option<&str>) -> AppState {
        AppState {
            client: reqwest::Client::new(),
            base_url: std::sync::Mutex::new("http://localhost:8081".into()),
            api_key: std::sync::Mutex::new(key.map(str::to_string)),
        }
    }

    #[test]
    fn empty_key_sends_no_headers() {
        let headers = auth_headers(&test_state(None)).unwrap();
        assert!(headers.is_empty());
    }

    #[test]
    fn blank_key_sends_no_headers() {
        let headers = auth_headers(&test_state(Some("  "))).unwrap();
        assert!(headers.is_empty());
    }

    #[test]
    fn key_sets_bearer_and_x_api_key() {
        let headers = auth_headers(&test_state(Some("secret"))).unwrap();
        assert_eq!(headers.get(AUTHORIZATION).unwrap(), "Bearer secret");
        assert_eq!(headers.get("X-API-Key").unwrap(), "secret");
        assert_eq!(headers.len(), 2);
    }
}
