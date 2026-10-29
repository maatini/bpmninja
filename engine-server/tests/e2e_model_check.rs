//! Deploy-time model checking (`MODEL_CHECK`, feature `model-check`).

use engine_server::{AppBuildConfig, ModelCheckMode};
use serde_json::{Value, json};

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

/// XOR split followed by a parallel join. One branch is taken, so the join
/// waits forever: the analyzer's deadlock/livelock property
/// `option_to_complete`.
const DEADLOCK_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_Deadlock" targetNamespace="http://bpmn.io/schema/bpmn">
  <bpmn:process id="DeadlockProcess" isExecutable="true">
    <bpmn:startEvent id="Start" />
    <bpmn:exclusiveGateway id="Split" />
    <bpmn:userTask id="A" />
    <bpmn:userTask id="B" />
    <bpmn:parallelGateway id="Join" />
    <bpmn:endEvent id="End" />
    <bpmn:sequenceFlow id="F1" sourceRef="Start" targetRef="Split" />
    <bpmn:sequenceFlow id="FA" sourceRef="Split" targetRef="A" />
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
        worker_api_key: Some(None),
        bind_addr: Some("127.0.0.1".to_string()),
        cors_origins: Some(engine_server::default_cors_origins()),
        model_check: Some(mode),
        model_check_timeout_ms: Some(10_000),
        ..Default::default()
    }
}

async fn deploy(mode: ModelCheckMode, xml: &str) -> (reqwest::StatusCode, Value) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let app = engine_server::build_app_with_options(config(mode));
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let response = reqwest::Client::new()
        .post(format!("{base}/api/deploy"))
        .json(&json!({ "xml": xml, "name": "model-check.bpmn" }))
        .send()
        .await
        .expect("deploy request");
    let status = response.status();
    let body = response.json().await.unwrap_or(Value::Null);
    (status, body)
}

#[cfg(feature = "model-check")]
fn violation_properties(body: &Value) -> Vec<&str> {
    body["model_check"]["violations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|violation| violation["property"].as_str())
        .collect()
}

async fn definition_count(mode: ModelCheckMode) -> usize {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let base = format!("http://{}", listener.local_addr().expect("addr"));
    let app = engine_server::build_app_with_options(config(mode));
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("{base}/api/deploy"))
        .json(&json!({ "xml": DEADLOCK_BPMN, "name": "model-check.bpmn" }))
        .send()
        .await
        .expect("deploy");
    let listed: Value = client
        .get(format!("{base}/api/definitions"))
        .send()
        .await
        .expect("list")
        .json()
        .await
        .expect("list json");
    listed.as_array().map(Vec::len).unwrap_or(0)
}

#[tokio::test]
async fn off_mode_omits_the_report_and_deploys() {
    let (status, body) = deploy(ModelCheckMode::Off, DEADLOCK_BPMN).await;
    assert_eq!(status, 200, "{body}");
    assert!(body.get("model_check").is_none(), "{body}");
    assert!(body["definition_key"].is_string(), "{body}");
}

#[cfg(not(feature = "model-check"))]
#[tokio::test]
async fn warn_without_the_feature_deploys_with_an_unavailable_report() {
    let (status, body) = deploy(ModelCheckMode::Warn, SOUND_BPMN).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["model_check"]["status"], "unavailable", "{body}");
    assert!(body["definition_key"].is_string(), "{body}");
}

#[cfg(not(feature = "model-check"))]
#[tokio::test]
async fn enforce_without_the_feature_is_a_structured_rejection() {
    let (status, body) = deploy(ModelCheckMode::Enforce, SOUND_BPMN).await;
    assert_eq!(status, 400, "{body}");
    let error = body["error"].as_str().unwrap_or("");
    assert!(error.contains("Model check failed"), "{body}");
    assert!(error.contains("model-check"), "{body}");
    assert_eq!(body["model_check"]["status"], "unavailable", "{body}");
    assert_eq!(definition_count(ModelCheckMode::Enforce).await, 0);
}

#[cfg(feature = "model-check")]
#[tokio::test]
async fn sound_model_passes() {
    let (status, body) = deploy(ModelCheckMode::Warn, SOUND_BPMN).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["model_check"]["status"], "passed", "{body}");
}

#[cfg(feature = "model-check")]
#[tokio::test]
async fn deadlock_is_reported_in_warn_mode_and_still_deploys() {
    let (status, body) = deploy(ModelCheckMode::Warn, DEADLOCK_BPMN).await;
    assert_eq!(status, 200, "warn mode must still deploy: {body}");
    assert_eq!(body["model_check"]["status"], "violations", "{body}");
    let properties = violation_properties(&body);
    assert!(
        properties.contains(&"option_to_complete"),
        "deadlock/livelock must violate option_to_complete: {body}"
    );
    assert_eq!(definition_count(ModelCheckMode::Warn).await, 1);
}

#[cfg(feature = "model-check")]
#[tokio::test]
async fn deadlock_is_rejected_in_enforce_mode_with_findings() {
    let (status, body) = deploy(ModelCheckMode::Enforce, DEADLOCK_BPMN).await;
    assert_eq!(status, 400, "{body}");
    let error = body["error"].as_str().unwrap_or("");
    assert!(error.contains("Model check failed"), "{body}");
    assert!(error.contains("option_to_complete"), "{body}");
    assert_eq!(body["model_check"]["status"], "violations", "{body}");
    assert!(
        violation_properties(&body).contains(&"option_to_complete"),
        "{body}"
    );
    assert_eq!(definition_count(ModelCheckMode::Enforce).await, 0);
}
