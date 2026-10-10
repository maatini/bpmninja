//! E2E tests for optional API-key auth and CORS allowlist (Welle 3 S1/S2).
//!
//! Tests must not read process env (`BPMNINJA_API_KEY`, `CORS_ORIGINS`) —
//! all values are injected via `AppBuildConfig`.

use engine_server::AppBuildConfig;
use serde_json::Value;

const TEST_KEY: &str = "test-secret-key";

async fn start_server(config: AppBuildConfig) -> String {
    let app = engine_server::build_app_with_options(config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind failed");
    let addr = listener.local_addr().expect("addr failed");
    let base = format!("http://{addr}");
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

fn auth_on() -> AppBuildConfig {
    AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(Some(TEST_KEY.to_string())),
        cors_origins: Some(vec![
            "http://localhost:1420".to_string(),
            "http://127.0.0.1:1420".to_string(),
        ]),
        ..Default::default()
    }
}

fn auth_off() -> AppBuildConfig {
    AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(None),
        cors_origins: Some(vec![
            "http://localhost:1420".to_string(),
            "http://127.0.0.1:1420".to_string(),
        ]),
        ..Default::default()
    }
}

#[tokio::test]
async fn auth_off_info_is_200_without_header() {
    let base = start_server(auth_off()).await;
    let res = reqwest::get(format!("{base}/api/info")).await.unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn build_app_has_no_auth() {
    let app = engine_server::build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind failed");
    let addr = listener.local_addr().expect("addr failed");
    let base = format!("http://{addr}");
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let res = reqwest::get(format!("{base}/api/info")).await.unwrap();
    assert_eq!(res.status(), 200, "build_app() must stay unauthenticated");
}

#[tokio::test]
async fn auth_on_info_without_header_is_401() {
    let base = start_server(auth_on()).await;
    let res = reqwest::get(format!("{base}/api/info")).await.unwrap();
    assert_eq!(res.status(), 401);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["error"], "Unauthorized");
}

#[tokio::test]
async fn auth_on_wrong_key_is_401() {
    let base = start_server(auth_on()).await;
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{base}/api/info"))
        .header("Authorization", "Bearer wrong")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);

    let res = client
        .get(format!("{base}/api/info"))
        .header("X-API-Key", "wrong")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn auth_on_bearer_and_x_api_key_succeed() {
    let base = start_server(auth_on()).await;
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{base}/api/info"))
        .header("Authorization", format!("Bearer {TEST_KEY}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "Bearer must be accepted");

    let res = client
        .get(format!("{base}/api/info"))
        .header("X-API-Key", TEST_KEY)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "X-API-Key must be accepted");
}

#[tokio::test]
async fn health_and_ready_are_public_when_auth_on() {
    let base = start_server(auth_on()).await;
    let health = reqwest::get(format!("{base}/api/health")).await.unwrap();
    assert_eq!(health.status(), 200, "/api/health must stay public");
    let ready = reqwest::get(format!("{base}/api/ready")).await.unwrap();
    assert_eq!(ready.status(), 200, "/api/ready must stay public");
}

#[tokio::test]
async fn cors_allowlist_echoes_known_origin() {
    let base = start_server(auth_off()).await;
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base}/api/health"))
        .header("Origin", "http://localhost:1420")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:1420")
    );
}

#[tokio::test]
async fn cors_allowlist_rejects_unknown_origin() {
    let base = start_server(auth_off()).await;
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base}/api/health"))
        .header("Origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert!(
        res.headers().get("access-control-allow-origin").is_none(),
        "unknown origin must not receive ACAO (never silent Any)"
    );
}

#[tokio::test]
async fn cors_star_allows_any_origin() {
    let base = start_server(AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(None),
        cors_origins: Some(vec!["*".to_string()]),
        ..Default::default()
    })
    .await;
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{base}/api/health"))
        .header("Origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("*")
    );
}

#[tokio::test]
async fn cors_preflight_allows_authorization_header() {
    let base = start_server(auth_on()).await;
    let client = reqwest::Client::new();
    let res = client
        .request(reqwest::Method::OPTIONS, format!("{base}/api/info"))
        .header("Origin", "http://localhost:1420")
        .header("Access-Control-Request-Method", "GET")
        .header("Access-Control-Request-Headers", "authorization,x-api-key")
        .send()
        .await
        .unwrap();
    assert!(
        res.status().is_success(),
        "preflight should succeed, got {}",
        res.status()
    );
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:1420")
    );
}
