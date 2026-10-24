//! E2E tests for service task endpoints:
//! - GET /api/service-tasks
//! - POST /api/service-task/:id/failure
//! - POST /api/service-task/:id/extendLock
//! - POST /api/service-task/:id/bpmnError
//! - 409 lock conflict handling

use serde_json::Value;

const SERVICE_TASK_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="SvcTaskProcess" isExecutable="true">
    <startEvent id="start" />
    <serviceTask id="svc" data-topic="test_topic" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="svc" />
    <sequenceFlow id="f2" sourceRef="svc" targetRef="end" />
  </process>
</definitions>"#;

/// BPMN with a boundary error event on a service task.
const ERROR_BOUNDARY_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="ErrBoundaryProcess" isExecutable="true">
    <startEvent id="start" />
    <serviceTask id="svc" data-topic="err_topic" />
    <boundaryEvent id="berr" attachedToRef="svc" data-error-code="MY_ERR">
      <errorEventDefinition />
    </boundaryEvent>
    <endEvent id="end_ok" />
    <endEvent id="end_err" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="svc" />
    <sequenceFlow id="f2" sourceRef="svc" targetRef="end_ok" />
    <sequenceFlow id="f3" sourceRef="berr" targetRef="end_err" />
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

/// Helper: deploy + start + fetchAndLock, returns (base, client, instance_id, task_id).
async fn setup_locked_task(topic: &str, xml: &str) -> (String, reqwest::Client, String, String) {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // Deploy
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": xml, "name": "test" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let def_key = body["definition_key"].as_str().unwrap().to_string();

    // Start
    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let instance_id = body["instance_id"].as_str().unwrap().to_string();

    // Fetch & Lock
    let res = client
        .post(format!("{}/api/service-task/fetchAndLock", base))
        .json(&serde_json::json!({
            "workerId": "worker1",
            "maxTasks": 1,
            "topics": [{ "topicName": topic, "lockDuration": 30000 }]
        }))
        .send()
        .await
        .unwrap();
    let tasks: Vec<Value> = res.json().await.unwrap();
    assert_eq!(tasks.len(), 1, "Expected 1 locked task");
    let task_id = tasks[0]["id"].as_str().unwrap().to_string();

    (base, client, instance_id, task_id)
}

#[tokio::test]
async fn get_service_tasks_returns_list() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // Deploy & start a service task process
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": SERVICE_TASK_BPMN, "name": "test" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let def_key = body["definition_key"].as_str().unwrap();

    client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();

    // GET /api/service-tasks
    let res = client
        .get(format!("{}/api/service-tasks", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let tasks: Vec<Value> = res.json().await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["topic"], "test_topic");
}

#[tokio::test]
async fn fail_service_task_decrements_retries() {
    let (base, client, _inst_id, task_id) =
        setup_locked_task("test_topic", SERVICE_TASK_BPMN).await;

    let res = client
        .post(format!("{}/api/service-task/{}/failure", base, task_id))
        .json(&serde_json::json!({
            "workerId": "worker1",
            "errorMessage": "Something broke"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "fail should return 204");

    // Task should still be in list but unlocked with decremented retries
    let res = client
        .get(format!("{}/api/service-tasks", base))
        .send()
        .await
        .unwrap();
    let tasks: Vec<Value> = res.json().await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert!(
        tasks[0]["worker_id"].is_null(),
        "Should be unlocked after failure"
    );
    assert_eq!(
        tasks[0]["retries"].as_i64().unwrap(),
        2,
        "Retries should be decremented to 2"
    );
}

#[tokio::test]
async fn extend_lock_succeeds() {
    let (base, client, _inst_id, task_id) =
        setup_locked_task("test_topic", SERVICE_TASK_BPMN).await;

    let res = client
        .post(format!("{}/api/service-task/{}/extendLock", base, task_id))
        .json(&serde_json::json!({
            "workerId": "worker1",
            "newDuration": 120000
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "extendLock should return 204");
}

#[tokio::test]
async fn extend_lock_wrong_worker_returns_conflict() {
    let (base, client, _inst_id, task_id) =
        setup_locked_task("test_topic", SERVICE_TASK_BPMN).await;

    let res = client
        .post(format!("{}/api/service-task/{}/extendLock", base, task_id))
        .json(&serde_json::json!({
            "workerId": "wrong_worker",
            "newDuration": 120000
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 409, "Wrong worker should get 409 Conflict");
}

#[tokio::test]
async fn complete_wrong_worker_returns_conflict() {
    let (base, client, _inst_id, task_id) =
        setup_locked_task("test_topic", SERVICE_TASK_BPMN).await;

    let res = client
        .post(format!("{}/api/service-task/{}/complete", base, task_id))
        .json(&serde_json::json!({ "workerId": "wrong_worker" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 409, "Wrong worker completing should get 409");
}

#[tokio::test]
async fn bpmn_error_routes_to_boundary_event() {
    let (base, client, inst_id, task_id) =
        setup_locked_task("err_topic", ERROR_BOUNDARY_BPMN).await;

    let res = client
        .post(format!("{}/api/service-task/{}/bpmnError", base, task_id))
        .json(&serde_json::json!({
            "workerId": "worker1",
            "errorCode": "MY_ERR"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "bpmnError should return 204");

    // Instance should complete via the error boundary path
    let res = client
        .get(format!("{}/api/instances/{}", base, inst_id))
        .send()
        .await
        .unwrap();
    let details: Value = res.json().await.unwrap();
    assert_eq!(details["state"], "Completed");
    assert_eq!(
        details["current_node"], "end_err",
        "Should reach error end, not normal end"
    );
}

#[tokio::test]
async fn complete_nonexistent_task_returns_404() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let fake_id = uuid::Uuid::new_v4();

    let res = client
        .post(format!("{}/api/service-task/{}/complete", base, fake_id))
        .json(&serde_json::json!({ "workerId": "w1" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404, "Completing non-existent task should 404");
}

// ---------------------------------------------------------------------------
// fetchAndLock input validation (R2)
// ---------------------------------------------------------------------------

async fn fetch_and_lock(base: &str, body: Value) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{base}/api/service-task/fetchAndLock"))
        .json(&body)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn fetch_and_lock_rejects_lock_duration_above_one_hour() {
    let base = start_server().await;
    let res = fetch_and_lock(
        &base,
        serde_json::json!({
            "workerId": "w1",
            "maxTasks": 1,
            "topics": [{ "topicName": "t", "lockDuration": 3_600_001 }]
        }),
    )
    .await;
    assert_eq!(res.status(), 400);
    let body: Value = res.json().await.unwrap();
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("lockDuration"),
        "unexpected body: {body}"
    );
}

#[tokio::test]
async fn fetch_and_lock_rejects_huge_and_negative_lock_duration() {
    let base = start_server().await;
    for duration in [i64::MAX, 0, -1] {
        let res = fetch_and_lock(
            &base,
            serde_json::json!({
                "workerId": "w1",
                "maxTasks": 1,
                "topics": [{ "topicName": "t", "lockDuration": duration }]
            }),
        )
        .await;
        assert_eq!(
            res.status(),
            400,
            "lockDuration {duration} must be rejected"
        );
    }
}

#[tokio::test]
async fn fetch_and_lock_rejects_more_than_100_tasks() {
    let base = start_server().await;
    let res = fetch_and_lock(
        &base,
        serde_json::json!({
            "workerId": "w1",
            "maxTasks": 101,
            "topics": [{ "topicName": "t", "lockDuration": 30_000 }]
        }),
    )
    .await;
    assert_eq!(res.status(), 400);
    let body: Value = res.json().await.unwrap();
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("maxTasks"),
        "unexpected body: {body}"
    );

    let res = fetch_and_lock(
        &base,
        serde_json::json!({
            "workerId": "w1",
            "maxTasks": 0,
            "topics": [{ "topicName": "t", "lockDuration": 30_000 }]
        }),
    )
    .await;
    assert_eq!(res.status(), 400, "maxTasks 0 must be rejected");
}

#[tokio::test]
async fn fetch_and_lock_rejects_empty_topics_and_topic_name() {
    let base = start_server().await;

    let res = fetch_and_lock(
        &base,
        serde_json::json!({ "workerId": "w1", "maxTasks": 1, "topics": [] }),
    )
    .await;
    assert_eq!(res.status(), 400, "empty topics must be rejected");

    let res = fetch_and_lock(
        &base,
        serde_json::json!({
            "workerId": "w1",
            "maxTasks": 1,
            "topics": [{ "topicName": "  ", "lockDuration": 30_000 }]
        }),
    )
    .await;
    assert_eq!(res.status(), 400, "blank topicName must be rejected");
}

#[tokio::test]
async fn fetch_and_lock_applies_per_topic_duration() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{base}/api/deploy"))
        .json(&serde_json::json!({ "xml": SERVICE_TASK_BPMN, "name": "test" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let def_key = body["definition_key"].as_str().unwrap().to_string();
    client
        .post(format!("{base}/api/start"))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();

    let res = fetch_and_lock(
        &base,
        serde_json::json!({
            "workerId": "w1",
            "maxTasks": 5,
            "topics": [
                { "topicName": "other_topic", "lockDuration": 5_000 },
                { "topicName": "test_topic", "lockDuration": 60_000 }
            ]
        }),
    )
    .await;
    assert_eq!(res.status(), 200);
    let tasks: Vec<Value> = res.json().await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["topic"], "test_topic");

    let expiration: chrono::DateTime<chrono::Utc> = tasks[0]["lock_expiration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let remaining = (expiration - chrono::Utc::now()).num_seconds();
    assert!(
        (55..=60).contains(&remaining),
        "expected ~60s lock from topic duration, got {remaining}s"
    );
}

#[tokio::test]
async fn instance_incidents_can_be_listed_and_resolved() {
    let (base, client, instance_id, task_id) =
        setup_locked_task("test_topic", SERVICE_TASK_BPMN).await;

    let res = client
        .post(format!("{}/api/service-task/{}/failure", base, task_id))
        .json(&serde_json::json!({
            "workerId": "worker1",
            "retries": 0,
            "errorMessage": "boom",
            "errorDetails": "stack trace"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let res = client
        .get(format!("{}/api/instances/{}/incidents", base, instance_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let incidents: Vec<Value> = res.json().await.unwrap();
    assert_eq!(incidents.len(), 1);
    assert_eq!(incidents[0]["kind"], "service_task");
    assert_eq!(incidents[0]["message"], "boom");
    assert_eq!(incidents[0]["details"], "stack trace");
    assert!(incidents[0]["resolved_at"].is_null());
    let incident_id = incidents[0]["id"].as_str().unwrap().to_string();

    let resolve_url = format!(
        "{}/api/instances/{}/incidents/{}/resolve",
        base, instance_id, incident_id
    );
    let res = client
        .post(&resolve_url)
        .json(&serde_json::json!({ "resolvedBy": "admin" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let resolved: Value = res.json().await.unwrap();
    assert_eq!(resolved["resolved_by"], "admin");
    assert!(!resolved["resolved_at"].is_null());

    // Ein bereits aufgelöster Incident ist ein Konflikt.
    let res = client
        .post(&resolve_url)
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 409);

    // Unbekannter Incident liefert 404.
    let res = client
        .post(format!(
            "{}/api/instances/{}/incidents/00000000-0000-0000-0000-000000000001/resolve",
            base, instance_id
        ))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);

    let res = client
        .get(format!("{}/api/instances/{}/incidents", base, instance_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let incidents: Vec<Value> = res.json().await.unwrap();
    assert!(!incidents[0]["resolved_at"].is_null());
}
