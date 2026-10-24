//! E2E tests for error paths when starting a process instance.

use serde_json::Value;

/// Minimal BPMN with a Timer Start Event.
const TIMER_START_BPMN_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:timer="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="TimerStartProcess" isExecutable="true">
    <startEvent id="Start_1">
      <timerEventDefinition>
        <timeDuration>PT15S</timeDuration>
      </timerEventDefinition>
    </startEvent>
    <endEvent id="End_1" />
    <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="End_1" />
  </process>
</definitions>"#;

/// Helper: starts the engine-server on a random port and returns the base URL.
async fn start_server() -> String {
    start_server_with(engine_server::build_app()).await
}

async fn start_server_with(app: axum::Router) -> String {
    // Bind to port 0 → OS assigns a free port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr = listener.local_addr().expect("failed to get local addr");
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server error");
    });

    base_url
}

/// JSON-body errors must keep the documented `{ "error": ... }` shape.
async fn assert_json_error(res: reqwest::Response, expected_status: u16) {
    assert_eq!(res.status(), expected_status);
    let body: Value = res.json().await.expect("error body must be JSON");
    assert!(
        body["error"].as_str().is_some_and(|msg| !msg.is_empty()),
        "missing error message: {body}"
    );
}

#[tokio::test]
async fn start_with_invalid_uuid_format_returns_400() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({
            "definition_key": "this-is-not-a-valid-uuid"
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        res.status(),
        400,
        "Should return 400 Bad Request for invalid UUID"
    );

    let body: Value = res.json().await.expect("parse response");
    assert_eq!(body["error"], "Invalid UUID format");
}

#[tokio::test]
async fn start_with_unknown_definition_uuid_returns_404() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let random_id = uuid::Uuid::new_v4().to_string();

    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({
            "definition_key": random_id
        }))
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        res.status(),
        404,
        "Should return 404 Not Found for unknown definition"
    );

    let body: Value = res.json().await.expect("parse response");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("Definition not found")
    );
}

#[tokio::test]
async fn start_latest_with_unknown_bpmn_id_returns_400() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/api/start/latest", base))
        .json(&serde_json::json!({
            "bpmn_id": "NonExistentProcess"
        }))
        .send()
        .await
        .expect("request failed");

    // Our engine maps EngineError::InvalidDefinition to 400 Bad Request
    assert_eq!(
        res.status(),
        400,
        "Should return 400 Bad Request for unknown latest BPMN id"
    );

    let body: Value = res.json().await.expect("parse response");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("No definition found with BPMN ID")
    );
}

#[tokio::test]
async fn start_instance_with_timer_start_returns_400() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // 1. Deploy definition with Timer Start
    let deploy_res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({
            "xml": TIMER_START_BPMN_XML,
            "name": "TimerStartProcess"
        }))
        .send()
        .await
        .expect("deploy request failed");

    assert_eq!(deploy_res.status(), 200);
    let deploy_body: Value = deploy_res.json().await.expect("parse deploy response");
    let def_key = deploy_body["definition_key"].as_str().unwrap().to_string();

    // 2. Normal Start should FAIL because it's a timer start event
    let start_res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({
            "definition_key": def_key
        }))
        .send()
        .await
        .expect("start request failed");

    assert_eq!(
        start_res.status(),
        400,
        "Should not be able to normal-start a timer-start process"
    );

    let body: Value = start_res.json().await.expect("parse response");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("Use trigger_timer_start")
    );
}

#[tokio::test]
async fn malformed_json_returns_error_object() {
    let base = start_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/start", base))
        .header("content-type", "application/json")
        .body("{\"definition_key\": ")
        .send()
        .await
        .expect("request failed");

    assert_json_error(res, 400).await;
}

#[tokio::test]
async fn json_type_mismatch_returns_422_error_object() {
    let base = start_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": 42 }))
        .send()
        .await
        .expect("request failed");

    assert_json_error(res, 422).await;
}

#[tokio::test]
async fn non_json_content_type_returns_415_error_object() {
    let base = start_server().await;
    let res = reqwest::Client::new()
        .post(format!("{}/api/start", base))
        .header("content-type", "text/plain")
        .body("{}")
        .send()
        .await
        .expect("request failed");

    assert_json_error(res, 415).await;
}

#[tokio::test]
async fn oversized_json_body_returns_413_error_object() {
    let app = engine_server::build_app_with_options(engine_server::AppBuildConfig {
        require_nats: Some(false),
        max_xml_bytes: Some(128),
        api_key: Some(None),
        bind_addr: Some("127.0.0.1".to_string()),
        ..Default::default()
    });
    let base = start_server_with(app).await;

    let res = reqwest::Client::new()
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({
            "definition_key": "x".repeat(4096)
        }))
        .send()
        .await
        .expect("request failed");

    assert_json_error(res, 413).await;
}
