use engine_core::adapter::InMemoryPersistence;
use engine_core::{WorkflowEngine, WorkflowPersistence};
use engine_server::AppBuildConfig;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

const MAPPED: &str = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn" xmlns:bpmninja="https://bpmninja.dev/schema/bpmn">
<process id="data" isExecutable="true"><property id="customer" name="customerId" bpmninja:default="7"/>
<startEvent id="start"/><serviceTask id="work" camunda:topic="mapped"><extensionElements><camunda:inputOutput>
<camunda:inputParameter name="id">${customerId}</camunda:inputParameter><camunda:outputParameter name="receipt">${result}</camunda:outputParameter>
</camunda:inputOutput></extensionElements></serviceTask><endEvent id="end"/>
<sequenceFlow id="f1" sourceRef="start" targetRef="work"/><sequenceFlow id="f2" sourceRef="work" targetRef="end"/>
</process></definitions>"#;

async fn serve(
    engine: Arc<WorkflowEngine>,
    persistence: Arc<InMemoryPersistence>,
) -> (String, tokio::task::JoinHandle<()>) {
    let app = engine_server::build_app_with_config(
        engine,
        Some(persistence),
        HashMap::new(),
        None,
        Arc::new(engine_server::LogBuffer::new()),
        AppBuildConfig {
            require_nats: Some(false),
            api_key: Some(None),
            cors_origins: Some(engine_server::default_cors_origins()),
            ..Default::default()
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, handle)
}
async fn post(client: &reqwest::Client, base: &str, path: &str, value: Value) -> Value {
    let res = client
        .post(format!("{base}{path}"))
        .json(&value)
        .send()
        .await
        .unwrap();
    assert!(
        res.status().is_success(),
        "{path}: {}",
        res.text().await.unwrap()
    );
    res.json().await.unwrap()
}
async fn fetch(client: &reqwest::Client, base: &str) -> Value {
    post(client, base, "/api/service-task/fetchAndLock", json!({"workerId":"worker", "maxTasks":1,"topics":[{"topicName":"mapped","lockDuration":60000}]})).await[0].clone()
}

#[tokio::test]
async fn x11_rest_defaults_output_failure_retry_and_restart() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (base, server) = serve(engine.clone(), persistence.clone()).await;
    let client = reqwest::Client::new();
    let key = post(
        &client,
        &base,
        "/api/deploy",
        json!({"xml":MAPPED, "name":"data"}),
    )
    .await["definition_key"]
        .as_str()
        .unwrap()
        .to_string();
    let instance = post(
        &client,
        &base,
        "/api/start",
        json!({"definition_key":key,"variables":{"customerId":9}}),
    )
    .await["instance_id"]
        .as_str()
        .unwrap()
        .to_string();
    let task = fetch(&client, &base).await;
    assert_eq!(task["variables_snapshot"]["id"], json!(9));
    let task_id = task["id"].as_str().unwrap();
    let failure = client
        .post(format!("{base}/api/service-task/{task_id}/complete"))
        .json(&json!({"workerId":"worker"}))
        .send()
        .await
        .unwrap();
    assert_eq!(failure.status(), 400);
    server.abort();
    drop(engine);

    let restored = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    for def in persistence.list_definitions().await.unwrap() {
        restored.deploy_definition(def).await;
    }
    for instance in persistence.list_instances().await.unwrap() {
        restored
            .restore_instance(
                serde_json::from_value(serde_json::to_value(instance).unwrap()).unwrap(),
            )
            .await;
    }
    for task in persistence.list_service_tasks().await.unwrap() {
        restored.restore_service_task(task);
    }
    let (base, server) = serve(restored, persistence).await;
    let res = client
        .post(format!("{base}/api/service-task/{task_id}/complete"))
        .json(&json!({"workerId":"worker","variables":{"result":42}}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);
    let details: Value = client
        .get(format!("{base}/api/instances/{instance}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(details["state"], "Completed");
    assert_eq!(details["variables"]["receipt"], 42);
    assert_eq!(details["variables"]["customerId"], 9);
    assert!(details["variables"].get("id").is_none());
    assert!(details["variables"].get("result").is_none());
    server.abort();
}

#[tokio::test]
async fn x11_rest_call_activity_mappings_and_invalid_deploy() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (base, server) = serve(engine, persistence).await;
    let client = reqwest::Client::new();
    let child = MAPPED
        .replace("id=\"data\"", "id=\"child\"")
        .replace("${customerId}", "${childId}")
        .replace(
            "<serviceTask id=\"work\" camunda:topic=\"mapped\">",
            "<scriptTask id=\"work\" scriptFormat=\"rhai\"><script>let result = id * 2;</script>",
        )
        .replace("</serviceTask>", "</scriptTask>");
    post(
        &client,
        &base,
        "/api/deploy",
        json!({"xml":child, "name":"child"}),
    )
    .await;
    let parent = MAPPED.replace("<serviceTask id=\"work\" camunda:topic=\"mapped\">", "<callActivity id=\"work\" calledElement=\"child\">")
        .replace("</serviceTask>", "</callActivity>")
        .replace("</camunda:inputOutput>", "</camunda:inputOutput><camunda:in source=\"id\" target=\"childId\"/><camunda:out source=\"receipt\" target=\"result\"/>");
    let key = post(
        &client,
        &base,
        "/api/deploy",
        json!({"xml":parent, "name":"data"}),
    )
    .await["definition_key"]
        .as_str()
        .unwrap()
        .to_string();
    let instance =
        post(&client, &base, "/api/start", json!({"definition_key":key})).await["instance_id"]
            .as_str()
            .unwrap()
            .to_string();
    let details: Value = client
        .get(format!("{base}/api/instances/{instance}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(details["state"], "Completed");
    assert_eq!(details["variables"]["receipt"], 14);
    assert!(details["variables"].get("childId").is_none());
    let res = client
        .post(format!("{base}/api/deploy"))
        .json(
            &json!({"name":"invalid", "xml":MAPPED.replace("${customerId}", "${customerId + 1}")}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
    server.abort();
}

#[tokio::test]
async fn x11_camunda_worker_receives_local_inputs_and_completes_mapped_outputs() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (base, server) = serve(engine, persistence).await;
    let client = reqwest::Client::new();
    let key = post(
        &client,
        &base,
        "/api/deploy",
        json!({"xml":MAPPED,"name":"data"}),
    )
    .await["definition_key"]
        .as_str()
        .unwrap()
        .to_string();
    let id = post(&client, &base, "/api/start", json!({"definition_key":key})).await["instance_id"]
        .as_str()
        .unwrap()
        .to_string();
    let tasks = post(&client, &base, "/engine-rest/external-task/fetchAndLock", json!({"workerId":"worker","maxTasks":1,"topics":[{"topicName":"mapped","lockDuration":60000,"variables":["id"]}]})).await;
    assert_eq!(tasks[0]["variables"]["id"]["value"], 7);
    assert_eq!(tasks[0]["variables"]["id"]["type"], "Integer");
    assert!(tasks[0]["variables"].get("customerId").is_none());
    let task = tasks[0]["id"].as_str().unwrap();
    let res = client
        .post(format!("{base}/engine-rest/external-task/{task}/complete"))
        .json(&json!({"workerId":"worker","variables":{"result":{"value":42,"type":"Integer"}}}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);
    let inst: Value = client
        .get(format!("{base}/api/instances/{id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inst["variables"]["receipt"], 42);
    assert!(inst["variables"].get("id").is_none());
    server.abort();
}
