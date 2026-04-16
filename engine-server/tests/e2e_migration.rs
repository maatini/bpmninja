//! E2E tests for process instance migration.
//! POST /api/instances/{id}/migrate

use serde_json::Value;

// V1: OrderProcess with single user task "approve"
const BPMN_V1: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="OrderProcess" isExecutable="true">
    <startEvent id="start" />
    <userTask id="approve" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="approve" />
    <sequenceFlow id="f2" sourceRef="approve" targetRef="end" />
  </process>
</definitions>"#;

// V2: Same bpmn_id, same node IDs → migration without mapping is safe
const BPMN_V2_COMPAT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="OrderProcess" isExecutable="true">
    <startEvent id="start" />
    <userTask id="approve" />
    <userTask id="notify" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="approve" />
    <sequenceFlow id="f2" sourceRef="approve" targetRef="notify" />
    <sequenceFlow id="f3" sourceRef="notify" targetRef="end" />
  </process>
</definitions>"#;

// V2-renamed: "approve" has been renamed to "review"
const BPMN_V2_RENAMED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="OrderProcess" isExecutable="true">
    <startEvent id="start" />
    <userTask id="review" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="review" />
    <sequenceFlow id="f2" sourceRef="review" targetRef="end" />
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

/// Deploy BPMN and return the definition key.
async fn deploy(base: &str, client: &reqwest::Client, xml: &str) -> String {
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": xml, "name": "test" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "deploy failed");
    res.json::<Value>().await.unwrap()["definition_key"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Deploy + start, returning (def_key, instance_id). Instance pauses at user task.
async fn deploy_and_start(base: &str, client: &reqwest::Client, xml: &str) -> (String, String) {
    let def_key = deploy(base, client, xml).await;
    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "start failed");
    let inst_id = res.json::<Value>().await.unwrap()["instance_id"]
        .as_str()
        .unwrap()
        .to_string();
    (def_key, inst_id)
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// Migrating an instance where both versions share the same node IDs succeeds
/// without providing a node mapping.
#[tokio::test]
async fn migrate_instance_same_node_ids_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let (_, inst_id) = deploy_and_start(&base, &client, BPMN_V1).await;
    let key_v2 = deploy(&base, &client, BPMN_V2_COMPAT).await;

    let res = client
        .post(format!("{}/api/instances/{}/migrate", base, inst_id))
        .json(&serde_json::json!({
            "target_definition_key": key_v2,
            "node_mapping": {}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "migration with identical node IDs should succeed");
}

/// After migration the instance reflects the new definition key.
#[tokio::test]
async fn migrate_instance_updates_definition_key() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let (_, inst_id) = deploy_and_start(&base, &client, BPMN_V1).await;
    let key_v2 = deploy(&base, &client, BPMN_V2_COMPAT).await;

    client
        .post(format!("{}/api/instances/{}/migrate", base, inst_id))
        .json(&serde_json::json!({
            "target_definition_key": key_v2,
            "node_mapping": {}
        }))
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

    assert_eq!(
        inst["definition_key"].as_str().unwrap(),
        key_v2,
        "definition_key should point to the new version after migration"
    );
}

/// When a node has been renamed in the new version, providing an explicit
/// mapping (old_id → new_id) allows migration to succeed.
#[tokio::test]
async fn migrate_instance_with_node_mapping_returns_204() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let (_, inst_id) = deploy_and_start(&base, &client, BPMN_V1).await;
    // V2 renamed "approve" → "review"
    let key_v2 = deploy(&base, &client, BPMN_V2_RENAMED).await;

    let res = client
        .post(format!("{}/api/instances/{}/migrate", base, inst_id))
        .json(&serde_json::json!({
            "target_definition_key": key_v2,
            "node_mapping": { "approve": "review" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "migration with explicit node mapping should succeed");

    // Instance should now be waiting at the renamed node
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
        "review",
        "token should be remapped to 'review' after migration"
    );
}

/// An instance token sitting on a node that does not exist in the target
/// definition and has no mapping entry is rejected with 422 Unprocessable Entity.
#[tokio::test]
async fn migrate_orphaned_token_without_mapping_returns_422() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let (_, inst_id) = deploy_and_start(&base, &client, BPMN_V1).await;
    // V2 renamed "approve" → "review" — no mapping provided
    let key_v2 = deploy(&base, &client, BPMN_V2_RENAMED).await;

    let res = client
        .post(format!("{}/api/instances/{}/migrate", base, inst_id))
        .json(&serde_json::json!({
            "target_definition_key": key_v2,
            "node_mapping": {}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        422,
        "orphaned token without mapping should return 422 Unprocessable Entity"
    );

    let body: Value = res.json().await.unwrap();
    assert!(
        body["error"].as_str().unwrap_or("").contains("approve"),
        "error message should name the orphaned node"
    );
}

/// Migrating to a non-existent definition UUID returns 404.
#[tokio::test]
async fn migrate_instance_unknown_target_returns_404() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let (_, inst_id) = deploy_and_start(&base, &client, BPMN_V1).await;
    let fake_key = uuid::Uuid::new_v4();

    let res = client
        .post(format!("{}/api/instances/{}/migrate", base, inst_id))
        .json(&serde_json::json!({
            "target_definition_key": fake_key,
            "node_mapping": {}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        404,
        "migrating to unknown definition should return 404"
    );
}
