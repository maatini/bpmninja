//! E2E tests for signal correlation:
//! - POST /api/signal broadcasts to all waiting catches
//! - GET /api/signals lists pending signal catches
//! - Signal start events are started via correlation, not via POST /api/start

use serde_json::Value;

const SIGNAL_CATCH_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_Signal" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <signal id="sig_order_ready" name="ORDER_READY" />
  <process id="SignalCatchProcess" isExecutable="true">
    <startEvent id="start" />
    <intermediateCatchEvent id="wait_signal">
      <signalEventDefinition signalRef="sig_order_ready" />
    </intermediateCatchEvent>
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="wait_signal" />
    <sequenceFlow id="f2" sourceRef="wait_signal" targetRef="end" />
  </process>
</definitions>"#;

const SIGNAL_START_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_SignalStart" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <signal id="sig_batch" name="BATCH_START" />
  <process id="SignalStartProcess" isExecutable="true">
    <startEvent id="start">
      <signalEventDefinition signalRef="sig_batch" />
    </startEvent>
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="end" />
  </process>
</definitions>"#;

async fn start_server() -> String {
    let app = engine_server::build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind failed");
    let addr = listener.local_addr().expect("addr failed");
    let base = format!("http://{}", addr);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

async fn deploy(base: &str, client: &reqwest::Client, xml: &str) -> String {
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": xml, "name": "signals" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    body["definition_key"].as_str().unwrap().to_string()
}

async fn start(base: &str, client: &reqwest::Client, def_key: &str) -> String {
    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    body["instance_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn signal_broadcast_affects_all_waiting_instances() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let def_key = deploy(&base, &client, SIGNAL_CATCH_BPMN).await;

    let inst_a = start(&base, &client, &def_key).await;
    let inst_b = start(&base, &client, &def_key).await;

    let res = client
        .get(format!("{}/api/signals", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let pending: Value = res.json().await.unwrap();
    let pending_list = pending.as_array().expect("pending signals must be a list");
    assert_eq!(pending_list.len(), 2);
    assert!(
        pending_list
            .iter()
            .all(|c| c["signal_name"] == "ORDER_READY"),
        "unexpected pending signals: {pending}"
    );

    let res = client
        .post(format!("{}/api/signal", base))
        .json(&serde_json::json!({
            "signalName": "ORDER_READY",
            "variables": { "batch": 7 }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let affected = body["affectedInstances"]
        .as_array()
        .expect("affectedInstances must be a list");
    assert_eq!(affected.len(), 2, "unexpected body: {body}");
    let affected: Vec<&str> = affected.iter().filter_map(|v| v.as_str()).collect();
    assert!(affected.contains(&inst_a.as_str()));
    assert!(affected.contains(&inst_b.as_str()));

    for inst_id in [&inst_a, &inst_b] {
        let res = client
            .get(format!("{}/api/instances/{}", base, inst_id))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let details: Value = res.json().await.unwrap();
        assert_eq!(details["state"], "Completed");
    }

    let res = client
        .get(format!("{}/api/signals", base))
        .send()
        .await
        .unwrap();
    let pending: Value = res.json().await.unwrap();
    assert!(pending.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn signal_start_instance_requires_correlation() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let def_key = deploy(&base, &client, SIGNAL_START_BPMN).await;

    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
    let body: Value = res.json().await.unwrap();
    assert!(
        body["error"]
            .as_str()
            .is_some_and(|msg| msg.contains("correlate_signal")),
        "unexpected error body: {body}"
    );

    let res = client
        .post(format!("{}/api/signal", base))
        .json(&serde_json::json!({ "signalName": "BATCH_START" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    let affected = body["affectedInstances"].as_array().unwrap();
    assert_eq!(affected.len(), 1);

    let inst_id = affected[0].as_str().unwrap();
    let res = client
        .get(format!("{}/api/instances/{}", base, inst_id))
        .send()
        .await
        .unwrap();
    let details: Value = res.json().await.unwrap();
    assert_eq!(details["state"], "Completed");
}

#[tokio::test]
async fn signal_without_match_returns_empty_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/api/signal", base))
        .json(&serde_json::json!({ "signalName": "NOBODY_WAITS" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert!(body["affectedInstances"].as_array().unwrap().is_empty());
}
