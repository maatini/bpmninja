//! E2E: deploy-time model checking (V1, `rust_bpmn_analyzer`).

use engine_server::{AppBuildConfig, ModelCheckMode};
use serde_json::{Value, json};

/// start → task → end — sound.
const SOUND_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_Sound" targetNamespace="http://bpmn.io/schema/bpmn">
  <bpmn:process id="SoundProcess" isExecutable="true">
    <bpmn:startEvent id="Start" />
    <bpmn:userTask id="Task" />
    <bpmn:endEvent id="End" />
    <bpmn:sequenceFlow id="F1" sourceRef="Start" targetRef="Task" />
    <bpmn:sequenceFlow id="F2" sourceRef="Task" targetRef="End" />
  </bpmn:process>
</bpmn:definitions>"#;

/// XOR split followed by a parallel join: the join waits for a token that
/// never comes — the classic BPMN deadlock.
const DEADLOCK_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_Deadlock" targetNamespace="http://bpmn.io/schema/bpmn">
  <bpmn:process id="DeadlockProcess" isExecutable="true">
    <bpmn:startEvent id="Start" />
    <bpmn:exclusiveGateway id="Split" default="FB" />
    <bpmn:userTask id="A" />
    <bpmn:userTask id="B" />
    <bpmn:parallelGateway id="Join" />
    <bpmn:endEvent id="End" />
    <bpmn:sequenceFlow id="F1" sourceRef="Start" targetRef="Split" />
    <bpmn:sequenceFlow id="FA" sourceRef="Split" targetRef="A">
      <bpmn:conditionExpression>x &gt; 1</bpmn:conditionExpression>
    </bpmn:sequenceFlow>
    <bpmn:sequenceFlow id="FB" sourceRef="Split" targetRef="B" />
    <bpmn:sequenceFlow id="F2" sourceRef="A" targetRef="Join" />
    <bpmn:sequenceFlow id="F3" sourceRef="B" targetRef="Join" />
    <bpmn:sequenceFlow id="F4" sourceRef="Join" targetRef="End" />
  </bpmn:process>
</bpmn:definitions>"#;

fn config(mode: ModelCheckMode) -> AppBuildConfig {
    AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(None),
        cors_origins: Some(engine_server::default_cors_origins()),
        model_check: Some(mode),
        model_check_timeout_ms: Some(10_000),
        ..Default::default()
    }
}

async fn deploy(mode: ModelCheckMode, xml: &str) -> (reqwest::StatusCode, Value) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = engine_server::build_app_with_options(config(mode));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/deploy"))
        .json(&json!({ "xml": xml, "name": "model-check.bpmn" }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.json().await.unwrap_or(Value::Null))
}

#[tokio::test]
async fn sound_model_passes() {
    let (status, body) = deploy(ModelCheckMode::Warn, SOUND_BPMN).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["model_check"]["status"], "passed", "{body}");
}

#[tokio::test]
async fn deadlock_is_reported_in_warn_mode() {
    let (status, body) = deploy(ModelCheckMode::Warn, DEADLOCK_BPMN).await;
    assert_eq!(status, 200, "warn mode must still deploy: {body}");
    assert_eq!(body["model_check"]["status"], "violations", "{body}");
    let properties: Vec<&str> = body["model_check"]["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["property"].as_str())
        .collect();
    assert!(
        properties.contains(&"option_to_complete"),
        "deadlock must violate option_to_complete: {body}"
    );
}

#[tokio::test]
async fn deadlock_is_rejected_in_enforce_mode() {
    let (status, body) = deploy(ModelCheckMode::Enforce, DEADLOCK_BPMN).await;
    assert_eq!(status, 400, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Model check failed"),
        "{body}"
    );
}

#[tokio::test]
async fn off_mode_adds_no_report() {
    let (status, body) = deploy(ModelCheckMode::Off, DEADLOCK_BPMN).await;
    assert_eq!(status, 200, "{body}");
    assert!(body.get("model_check").is_none(), "{body}");
}
