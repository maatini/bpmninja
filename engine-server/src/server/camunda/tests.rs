use super::*;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;

const CHAIN: &str = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
<process id="compat" isExecutable="true"><startEvent id="start"/>
<serviceTask id="first" camunda:topic="first"/><serviceTask id="second" camunda:topic="second"/><endEvent id="end"/>
<sequenceFlow id="f1" sourceRef="start" targetRef="first"/><sequenceFlow id="f2" sourceRef="first" targetRef="second"/><sequenceFlow id="f3" sourceRef="second" targetRef="end"/>
</process></definitions>"#;

async fn request(app: &Router, method: &str, path: &str, payload: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}
async fn setup(xml: &str) -> (Router, String) {
    let app = crate::build_app();
    let (status, definition) = request(
        &app,
        "POST",
        "/api/deploy",
        json!({"xml":xml,"name":"compat"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{definition}");
    let (status, instance) = request(&app, "POST", "/api/start", json!({"definition_key":definition["definition_key"],"variables":{"initial":1,"hidden":"secret"}})).await;
    assert_eq!(status, StatusCode::OK);
    (app, instance["instance_id"].as_str().unwrap().into())
}
fn fetch(topic: &str) -> Value {
    json!({"workerId":"worker","maxTasks":1,"usePriority":true,"topics":[{"topicName":topic,"lockDuration":30000,"deserializeValues":true}]})
}

#[tokio::test]
async fn bpmn_error_variables_arrive_on_the_boundary_path() {
    let xml = CHAIN.replace("<endEvent id=\"end\"/>", "<boundaryEvent id=\"error\" attachedToRef=\"first\" data-error-code=\"E\"><errorEventDefinition/></boundaryEvent><endEvent id=\"end\"/>")
        .replace("</process>", "<sequenceFlow id=\"error-flow\" sourceRef=\"error\" targetRef=\"second\"/></process>");
    let (app, instance) = setup(&xml).await;
    let (_, tasks) = request(
        &app,
        "POST",
        "/engine-rest/external-task/fetchAndLock",
        fetch("first"),
    )
    .await;
    let id = tasks[0]["id"].as_str().unwrap();
    let (status, error) = request(&app,"POST",&format!("/engine-rest/external-task/{id}/bpmnError"),json!({"workerId":"worker","errorCode":"E","errorMessage":"details","variables":{"reason":{"type":"Long","value":9,"valueInfo":null}}})).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{error}");
    let (_, tasks) = request(
        &app,
        "POST",
        "/engine-rest/external-task/fetchAndLock",
        fetch("second"),
    )
    .await;
    assert_eq!(tasks[0]["variables"]["reason"]["value"], 9);
    assert_eq!(tasks[0]["variables"]["reason"]["type"], "Long");
    let (_, details) = request(
        &app,
        "GET",
        &format!("/api/instances/{instance}"),
        Value::Null,
    )
    .await;
    assert!(
        details["audit_log"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry.as_str().unwrap().contains("details"))
    );
}

#[tokio::test]
async fn each_topic_uses_its_own_millisecond_lock_duration() {
    let (app, _) = setup(CHAIN).await;
    let xml = CHAIN.replace("camunda:topic=\"first\"", "camunda:topic=\"other\"");
    let (_, definition) = request(
        &app,
        "POST",
        "/api/deploy",
        json!({"xml":xml,"name":"other"}),
    )
    .await;
    request(
        &app,
        "POST",
        "/api/start",
        json!({"definition_key":definition["definition_key"]}),
    )
    .await;
    let before = Utc::now().timestamp_millis();
    let (_, tasks) = request(&app,"POST","/engine-rest/external-task/fetchAndLock",json!({"workerId":"worker","maxTasks":2,"topics":[{"topicName":"first","lockDuration":12345},{"topicName":"other","lockDuration":23456}]})).await;
    let after = Utc::now().timestamp_millis();
    assert_eq!(tasks.as_array().unwrap().len(), 2);
    for (index, duration) in [12345, 23456].into_iter().enumerate() {
        let expiry = chrono::DateTime::parse_from_str(
            tasks[index]["lockExpirationTime"].as_str().unwrap(),
            "%Y-%m-%dT%H:%M:%S%.f%z",
        )
        .unwrap()
        .timestamp_millis();
        assert!(expiry - duration >= before && expiry - duration <= after);
    }
}

#[tokio::test]
async fn lifecycle_preserves_typed_variables_and_camel_case_metadata() {
    let (app, instance) = setup(CHAIN).await;
    let mut payload = fetch("first");
    payload["topics"][0]["variables"] = json!(["initial"]);
    let (status, tasks) = request(
        &app,
        "POST",
        "/engine-rest/external-task/fetchAndLock",
        payload,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tasks[0]["processInstanceId"], instance);
    assert_eq!(tasks[0]["processDefinitionKey"], "compat");
    assert_eq!(tasks[0]["activityId"], "first");
    assert_eq!(
        tasks[0]["variables"]["initial"],
        json!({"type":"Integer","value":1,"valueInfo":{}})
    );
    assert!(tasks[0]["variables"].get("hidden").is_none());
    assert!(
        chrono::DateTime::parse_from_str(
            tasks[0]["lockExpirationTime"].as_str().unwrap(),
            "%Y-%m-%dT%H:%M:%S%.f%z"
        )
        .is_ok()
    );
    let id = tasks[0]["id"].as_str().unwrap();
    let (status, error) = request(
        &app,
        "POST",
        &format!("/engine-rest/external-task/{id}/complete"),
        json!({"workerId":"other"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["type"], "InvalidRequestException");
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/engine-rest/external-task/{id}/extendLock"),
            json!({"workerId":"worker","newDuration":12345})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let variables = json!({"long":{"type":"Long","value":5}, "short":{"type":"Short","value":2}, "double":{"type":"Double","value":1.5}, "date":{"type":"Date","value":"2026-10-13T00:00:00.000+0000"}, "json":{"type":"Json","value":"{\"nested\":[1,true]}","valueInfo":{"serializationDataFormat":"application/json","transient":false}}, "nullString":{"type":"String","value":null}});
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/engine-rest/external-task/{id}/complete"),
            json!({"workerId":"worker","variables":variables,"localVariables":{}})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let (_, tasks) = request(
        &app,
        "POST",
        "/engine-rest/external-task/fetchAndLock",
        fetch("second"),
    )
    .await;
    for (name, kind) in [
        ("long", "Long"),
        ("short", "Short"),
        ("double", "Double"),
        ("date", "Date"),
        ("json", "Json"),
        ("nullString", "String"),
    ] {
        assert_eq!(tasks[0]["variables"][name]["type"], kind);
    }
    assert_eq!(
        tasks[0]["variables"]["json"]["value"],
        "{\"nested\":[1,true]}"
    );
    let id = tasks[0]["id"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/engine-rest/external-task/{id}/complete"),
            json!({"workerId":"worker"})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("/api/instances/{instance}"),
            Value::Null
        )
        .await
        .1["state"],
        "Completed"
    );
}

#[tokio::test]
async fn failure_honors_retry_timeout() {
    let (app, _) = setup(CHAIN).await;
    let (_, tasks) = request(
        &app,
        "POST",
        "/engine-rest/external-task/fetchAndLock",
        fetch("first"),
    )
    .await;
    let id = tasks[0]["id"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/engine-rest/external-task/{id}/failure"),
            json!({"workerId":"worker","retries":2,"retryTimeout":60000,"errorMessage":"retry"})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/engine-rest/external-task/fetchAndLock",
            fetch("first")
        )
        .await
        .1,
        json!([])
    );
    let (_, native) = request(&app, "GET", "/api/service-tasks", Value::Null).await;
    assert_eq!(native[0]["worker_id"], Value::Null);
    assert_eq!(native[0]["retries"], 2);
    assert!(native[0]["lock_expiration"].is_string());
}

#[tokio::test]
async fn rejects_unsupported_options_before_locking_or_completing() {
    let (app, _) = setup(CHAIN).await;
    let mut invalid = fetch("first");
    invalid["topics"][0]["processDefinitionKey"] = json!("compat");
    assert_eq!(
        request(
            &app,
            "POST",
            "/engine-rest/external-task/fetchAndLock",
            invalid
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (_, tasks) = request(
        &app,
        "POST",
        "/engine-rest/external-task/fetchAndLock",
        fetch("first"),
    )
    .await;
    let id = tasks[0]["id"].as_str().unwrap();
    for variables in [
        json!({"bad":{"type":"Integer","value":2147483648_i64}}),
        json!({"bad":{"type":"Object","value":"java"}}),
    ] {
        assert_eq!(
            request(
                &app,
                "POST",
                &format!("/engine-rest/external-task/{id}/complete"),
                json!({"workerId":"worker","variables":variables})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/engine-rest/external-task/{id}/complete"),
            json!({"workerId":"worker","localVariables":{"local":{"value":1}}})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/engine-rest/external-task/{id}/complete"),
            json!({"workerId":"worker"})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/engine-rest/external-task/{id}/complete"),
            json!({"workerId":"worker"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn malformed_requests_and_auth_use_camunda_error_envelopes() {
    let app = crate::build_app();
    for payload in [json!({"workerId":"worker","maxTasks":0}), json!({})] {
        let (status, error) = request(
            &app,
            "POST",
            "/engine-rest/external-task/fetchAndLock",
            payload,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error["type"], "InvalidRequestException");
    }
    let app = crate::server::build_app_with_options(crate::server::AppBuildConfig {
        api_key: Some(Some("test-key".into())),
        require_nats: Some(false),
        ..Default::default()
    });
    let (status, error) = request(
        &app,
        "POST",
        "/engine-rest/external-task/fetchAndLock",
        fetch("first"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error["type"], "AuthorizationException");
}
