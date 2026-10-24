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

// ---------------------------------------------------------------------------
// Fail-closed gate + role separation (R5)
// ---------------------------------------------------------------------------

const ADMIN_KEY: &str = "admin-secret-key";
const WORKER_KEY: &str = "worker-secret-key";

/// No keys configured (read from env, normally unset) on a non-loopback bind.
fn fail_closed() -> AppBuildConfig {
    AppBuildConfig {
        require_nats: Some(false),
        worker_api_key: Some(None),
        bind_addr: Some("0.0.0.0".to_string()),
        ..Default::default()
    }
}

fn admin_and_worker() -> AppBuildConfig {
    AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(Some(ADMIN_KEY.to_string())),
        worker_api_key: Some(Some(WORKER_KEY.to_string())),
        bind_addr: Some("127.0.0.1".to_string()),
        ..Default::default()
    }
}

fn admin_only() -> AppBuildConfig {
    AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(Some(ADMIN_KEY.to_string())),
        worker_api_key: Some(None),
        bind_addr: Some("0.0.0.0".to_string()),
        ..Default::default()
    }
}

#[tokio::test]
async fn fail_closed_without_key_on_non_loopback_bind() {
    let base = start_server(fail_closed()).await;

    let res = reqwest::get(format!("{base}/api/info")).await.unwrap();
    assert_eq!(res.status(), 401, "/api/info must be closed");
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["error"], "Unauthorized");

    let res = reqwest::get(format!("{base}/api/definitions"))
        .await
        .unwrap();
    assert_eq!(res.status(), 401, "/api/definitions must be closed");

    // Health and readiness stay public.
    let health = reqwest::get(format!("{base}/api/health")).await.unwrap();
    assert_eq!(health.status(), 200);
    let ready = reqwest::get(format!("{base}/api/ready")).await.unwrap();
    assert_eq!(ready.status(), 200);
}

#[tokio::test]
async fn loopback_without_key_stays_open() {
    let base = start_server(AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(None),
        worker_api_key: Some(None),
        bind_addr: Some("127.0.0.1".to_string()),
        ..Default::default()
    })
    .await;
    let res = reqwest::get(format!("{base}/api/info")).await.unwrap();
    assert_eq!(res.status(), 200, "loopback dev bind keeps auth off");
}

#[tokio::test]
async fn explicit_escape_hatch_allows_unauthenticated_non_loopback() {
    if std::env::var("BPMNINJA_API_KEY").is_ok() {
        // `api_key: None` reads the env; a configured key would (correctly) win.
        return;
    }
    let base = start_server(AppBuildConfig {
        require_nats: Some(false),
        api_key: None,
        worker_api_key: Some(None),
        bind_addr: Some("0.0.0.0".to_string()),
        allow_unauthenticated: Some(true),
        ..Default::default()
    })
    .await;
    let res = reqwest::get(format!("{base}/api/info")).await.unwrap();
    assert_eq!(
        res.status(),
        200,
        "BPMNINJA_ALLOW_UNAUTHENTICATED is explicit"
    );
}

#[tokio::test]
async fn worker_key_is_rejected_on_admin_routes() {
    let base = start_server(admin_and_worker()).await;
    let client = reqwest::Client::new();

    for (method, path) in [
        (reqwest::Method::GET, "/api/info"),
        (reqwest::Method::GET, "/api/instances"),
        (reqwest::Method::GET, "/api/definitions"),
        (reqwest::Method::GET, "/api/logs"),
        (reqwest::Method::POST, "/api/deploy"),
        (reqwest::Method::POST, "/api/start"),
    ] {
        let res = client
            .request(method.clone(), format!("{base}{path}"))
            .header("Authorization", format!("Bearer {WORKER_KEY}"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            403,
            "worker key must not reach {method} {path}"
        );
        let body: Value = res.json().await.unwrap();
        assert_eq!(body["error"], "Forbidden");
    }
}

#[tokio::test]
async fn worker_key_is_rejected_on_incident_admin_routes() {
    let base = start_server(admin_and_worker()).await;
    let client = reqwest::Client::new();
    let id = uuid::Uuid::new_v4();

    for action in ["retry", "resolve"] {
        let res = client
            .post(format!("{base}/api/service-task/{id}/{action}"))
            .header("Authorization", format!("Bearer {WORKER_KEY}"))
            .json(&serde_json::json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            403,
            "worker key must not reach /{action} (admin only)"
        );
    }
}

#[tokio::test]
async fn worker_key_can_fetch_and_lock() {
    let base = start_server(admin_and_worker()).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/api/service-task/fetchAndLock"))
        .header("Authorization", format!("Bearer {WORKER_KEY}"))
        .json(&serde_json::json!({
            "workerId": "worker-1",
            "maxTasks": 1,
            "topics": [{ "topicName": "nothing_here", "lockDuration": 30_000 }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "fetchAndLock is a worker route");

    // Path parameters must not bypass the role check.
    let id = uuid::Uuid::new_v4();
    let res = client
        .post(format!("{base}/api/service-task/{id}/complete"))
        .header("X-API-Key", WORKER_KEY)
        .json(&serde_json::json!({ "workerId": "worker-1" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404, "worker route reached the handler (404)");
}

#[tokio::test]
async fn admin_key_keeps_full_access() {
    let base = start_server(admin_and_worker()).await;
    let client = reqwest::Client::new();

    let res = client
        .get(format!("{base}/api/info"))
        .header("Authorization", format!("Bearer {ADMIN_KEY}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let res = client
        .post(format!("{base}/api/service-task/fetchAndLock"))
        .header("X-API-Key", ADMIN_KEY)
        .json(&serde_json::json!({
            "workerId": "admin-worker",
            "maxTasks": 1,
            "topics": [{ "topicName": "nothing_here", "lockDuration": 30_000 }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "admin may use worker routes");
}

#[tokio::test]
async fn admin_key_still_covers_worker_routes_without_worker_key() {
    let base = start_server(admin_only()).await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/api/service-task/fetchAndLock"))
        .header("Authorization", format!("Bearer {ADMIN_KEY}"))
        .json(&serde_json::json!({
            "workerId": "admin-worker",
            "maxTasks": 1,
            "topics": [{ "topicName": "nothing_here", "lockDuration": 30_000 }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        200,
        "backward compatible when no worker key is set"
    );
}

#[tokio::test]
async fn auth_failures_are_counted_in_metrics() {
    let handle = engine_server::observability::install_prometheus_recorder();
    let base = start_server(admin_and_worker()).await;
    let client = reqwest::Client::new();

    let res = client.get(format!("{base}/api/info")).send().await.unwrap();
    assert_eq!(res.status(), 401, "missing key must be rejected");

    let res = client
        .get(format!("{base}/api/info"))
        .header("X-API-Key", WORKER_KEY)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403, "worker on admin route must be forbidden");

    let rendered = handle.render();
    assert!(
        rendered.contains("bpmn_auth_failures_total"),
        "metric missing in: {rendered}"
    );
    assert!(
        rendered.contains("reason=\"invalid_or_missing_key\""),
        "401 reason label missing in: {rendered}"
    );
    assert!(
        rendered.contains("reason=\"worker_role_forbidden\""),
        "403 reason label missing in: {rendered}"
    );
}
