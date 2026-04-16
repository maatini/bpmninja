//! E2E tests for instance lifecycle operations:
//! - POST /api/instances/{id}/suspend
//! - POST /api/instances/{id}/resume
//! - POST /api/instances/{id}/move-token

use serde_json::Value;

/// Two-task process: instance parks at the first user task "task_a".
const TWO_TASK_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="TwoTaskProcess" isExecutable="true">
    <startEvent id="start" />
    <userTask id="task_a" />
    <userTask id="task_b" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start"  targetRef="task_a" />
    <sequenceFlow id="f2" sourceRef="task_a" targetRef="task_b" />
    <sequenceFlow id="f3" sourceRef="task_b" targetRef="end" />
  </process>
</definitions>"#;

async fn start_server() -> String {
    let app = engine_server::build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{}", addr);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

/// Deploy + start, return (def_key, instance_id).
async fn deploy_and_start(base: &str, client: &reqwest::Client) -> (String, String) {
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": TWO_TASK_BPMN, "name": "test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let def_key = res.json::<Value>().await.unwrap()["definition_key"]
        .as_str()
        .unwrap()
        .to_string();

    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let inst_id = res.json::<Value>().await.unwrap()["instance_id"]
        .as_str()
        .unwrap()
        .to_string();

    (def_key, inst_id)
}

/// Suspending a running instance returns 204.
#[tokio::test]
async fn suspend_instance_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let (_, inst_id) = deploy_and_start(&base, &client).await;

    let res = client
        .post(format!("{}/api/instances/{}/suspend", base, inst_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "suspending a running instance should return 204");
}

/// After suspend, GET returns a Suspended state.
#[tokio::test]
async fn suspend_reflects_suspended_state() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let (_, inst_id) = deploy_and_start(&base, &client).await;

    client
        .post(format!("{}/api/instances/{}/suspend", base, inst_id))
        .send()
        .await
        .unwrap();

    let inst = client
        .get(format!("{}/api/instances/{}", base, inst_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    let state = &inst["state"];
    assert!(
        state.is_object() && state.get("Suspended").is_some(),
        "state should be Suspended after suspend, got: {}",
        state
    );
}

/// Suspended instance can be resumed; returns 204.
#[tokio::test]
async fn resume_suspended_instance_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let (_, inst_id) = deploy_and_start(&base, &client).await;

    // Suspend first
    client
        .post(format!("{}/api/instances/{}/suspend", base, inst_id))
        .send()
        .await
        .unwrap();

    // Then resume
    let res = client
        .post(format!("{}/api/instances/{}/resume", base, inst_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "resuming a suspended instance should return 204");

    // State should no longer be Suspended
    let inst = client
        .get(format!("{}/api/instances/{}", base, inst_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let state = &inst["state"];
    assert!(
        state.get("Suspended").is_none(),
        "state should not be Suspended after resume, got: {}",
        state
    );
}

/// Moving a token to a known node in the definition returns 204 and the
/// instance's current_node reflects the new position.
#[tokio::test]
async fn move_token_to_existing_node_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let (_, inst_id) = deploy_and_start(&base, &client).await;

    // Instance is waiting at task_a; jump token directly to task_b
    let res = client
        .post(format!("{}/api/instances/{}/move-token", base, inst_id))
        .json(&serde_json::json!({
            "target_node_id": "task_b",
            "variables": {},
            "cancel_current": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "move-token to a known node should return 204");

    let inst = client
        .get(format!("{}/api/instances/{}", base, inst_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(
        inst["current_node"].as_str().unwrap(),
        "task_b",
        "current_node should reflect the target after move-token"
    );
}

/// Moving a token to a node that does not exist in the definition returns 404.
#[tokio::test]
async fn move_token_to_nonexistent_node_returns_404() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let (_, inst_id) = deploy_and_start(&base, &client).await;

    let res = client
        .post(format!("{}/api/instances/{}/move-token", base, inst_id))
        .json(&serde_json::json!({
            "target_node_id": "no_such_node",
            "variables": {},
            "cancel_current": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        404,
        "move-token to a nonexistent node should return 404"
    );
}
