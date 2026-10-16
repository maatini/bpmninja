use engine_core::adapter::InMemoryPersistence;
use engine_core::{WorkflowEngine, WorkflowPersistence};
use engine_server::AppBuildConfig;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

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

fn xml(signal: bool) -> String {
    let (declaration, event) = if signal {
        (
            r#"<signal id="trigger" name="alert"/>"#,
            r#"<signalEventDefinition signalRef="trigger"/>"#,
        )
    } else {
        (
            r#"<message id="trigger" name="alert"/>"#,
            r#"<messageEventDefinition messageRef="trigger"/>"#,
        )
    };
    format!(
        r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn">{declaration}<process id="events" isExecutable="true"><startEvent id="start"/><serviceTask id="main" camunda:topic="main"/><endEvent id="end"/><sequenceFlow id="f1" sourceRef="start" targetRef="main"/><sequenceFlow id="f2" sourceRef="main" targetRef="end"/><subProcess id="handler" triggeredByEvent="true"><startEvent id="hs" isInterrupting="false">{event}</startEvent><serviceTask id="work" camunda:topic="handler"/><endEvent id="he"/><sequenceFlow id="hf1" sourceRef="hs" targetRef="work"/><sequenceFlow id="hf2" sourceRef="work" targetRef="he"/></subProcess></process></definitions>"#
    )
}

async fn get(client: &reqwest::Client, base: &str, path: &str) -> Value {
    client
        .get(format!("{base}{path}"))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn finish(client: &reqwest::Client, base: &str, topic: &str, instance: &str) {
    let tasks = post(client, base, "/api/service-task/fetchAndLock", json!({"workerId":"worker","maxTasks":1,"topics":[{"topicName":topic,"lockDuration":60000}]})).await;
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["instance_id"], instance);
    let id = tasks[0]["id"].as_str().unwrap();
    let response = client
        .post(format!("{base}/api/service-task/{id}/complete"))
        .json(&json!({"workerId":"worker","variables":{format!("{topic}Result"):42}}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204, "{}", response.text().await.unwrap());
}

#[tokio::test]
async fn x9_rest_message_handlers_repeat_and_survive_restart() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (base, server) = serve(engine.clone(), persistence.clone()).await;
    let client = reqwest::Client::new();
    let xml = xml(false).replace(r#"<subProcess id="handler" triggeredByEvent="true">"#, r#"<subProcess id="handler" triggeredByEvent="true"><extensionElements><camunda:executionListener event="start"><camunda:script scriptFormat="rhai">starts += 1;</camunda:script></camunda:executionListener><camunda:executionListener event="end"><camunda:script scriptFormat="rhai">ends += 1;</camunda:script></camunda:executionListener></extensionElements>"#);
    let deployed = post(
        &client,
        &base,
        "/api/deploy",
        json!({"xml":xml,"name":"events"}),
    )
    .await;
    assert_eq!(deployed["model_check"]["status"], "skipped");
    let key = deployed["definition_key"].as_str().unwrap();
    let exported = client
        .get(format!("{base}/api/definitions/{key}/xml"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(exported, xml);
    let started = post(
        &client,
        &base,
        "/api/start",
        json!({"definition_key":key,"variables":{"starts":0,"ends":0}}),
    )
    .await;
    let id = started["instance_id"].as_str().unwrap();
    for _ in 0..2 {
        let triggered = post(
            &client,
            &base,
            "/api/message",
            json!({"messageName":"alert","variables":{"payload":7}}),
        )
        .await;
        assert_eq!(triggered["affectedInstances"], json!([id]));
    }
    server.abort();
    drop(engine);
    let restored = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    for definition in persistence.list_definitions().await.unwrap() {
        restored.deploy_definition(definition).await;
    }
    for instance in persistence.list_instances().await.unwrap() {
        restored.restore_instance(instance).await;
    }
    for task in persistence.list_service_tasks().await.unwrap() {
        restored.restore_service_task(task);
    }
    for catch in persistence.list_message_catches().await.unwrap() {
        restored.restore_message_catch(catch);
    }
    restored.reconcile_event_subprocesses().await.unwrap();
    let (base, server) = serve(restored, persistence).await;
    finish(&client, &base, "main", id).await;
    assert_ne!(
        get(&client, &base, &format!("/api/instances/{id}")).await["state"],
        "Completed"
    );
    finish(&client, &base, "handler", id).await;
    finish(&client, &base, "handler", id).await;
    let details = get(&client, &base, &format!("/api/instances/{id}")).await;
    assert_eq!(details["state"], "Completed");
    assert_eq!(details["variables"]["mainResult"], 42);
    assert_eq!(details["variables"]["handlerResult"], 42);
    assert_eq!(details["variables"]["starts"], 2);
    assert_eq!(details["variables"]["ends"], 2);
    let history = get(&client, &base, &format!("/api/instances/{id}/history?event_types=EventSubprocessActivated,EventSubprocessCompleted&node_id=handler")).await;
    let entries = history.as_array().unwrap();
    assert_eq!(entries.len(), 4);
    assert!(
        entries
            .iter()
            .all(|entry| entry["context"]["activation_id"].is_string())
    );
    server.abort();
}

#[tokio::test]
async fn x9_rest_interrupting_signal_cancels_original_task() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (base, server) = serve(engine, persistence).await;
    let client = reqwest::Client::new();
    let deployed = post(
        &client,
        &base,
        "/api/deploy",
        json!({"name":"events","xml":xml(true).replace("isInterrupting=\"false\"", "isInterrupting=\"true\"")}),
    )
    .await;
    let started = post(
        &client,
        &base,
        "/api/start",
        json!({"definition_key":deployed["definition_key"]}),
    )
    .await;
    let id = started["instance_id"].as_str().unwrap();
    let triggered = post(&client, &base, "/api/signal", json!({"signalName":"alert"})).await;
    assert_eq!(triggered["affectedInstances"], json!([id]));
    let cancelled = post(&client, &base, "/api/service-task/fetchAndLock", json!({"workerId":"worker","maxTasks":1,"topics":[{"topicName":"main","lockDuration":60000}]})).await;
    assert_eq!(cancelled, json!([]));
    finish(&client, &base, "handler", id).await;
    assert_eq!(
        get(&client, &base, &format!("/api/instances/{id}")).await["state"],
        "Completed"
    );
    server.abort();
}

#[tokio::test]
async fn x9b_rest_direct_updates_rearm_and_preserve_latch_across_restart() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (base, server) = serve(engine.clone(), persistence.clone()).await;
    let client = reqwest::Client::new();
    let xml = xml(false).replace("<messageEventDefinition messageRef=\"trigger\"/>", "<conditionalEventDefinition><condition>amount &gt; 100</condition></conditionalEventDefinition>");
    let deployed = post(
        &client,
        &base,
        "/api/deploy",
        json!({"xml":xml,"name":"conditional"}),
    )
    .await;
    assert_eq!(deployed["model_check"]["status"], "skipped");
    let started = post(
        &client,
        &base,
        "/api/start",
        json!({"definition_key":deployed["definition_key"]}),
    )
    .await;
    let id = started["instance_id"].as_str().unwrap();
    for amount in [150, 160, 90, 150] {
        let response = client
            .put(format!("{base}/api/instances/{id}/variables"))
            .json(&json!({"variables":{"amount":amount}}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 204, "{}", response.text().await.unwrap());
    }
    server.abort();
    drop(engine);
    let restored = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    for definition in persistence.list_definitions().await.unwrap() {
        restored.deploy_definition(definition).await;
    }
    for instance in persistence.list_instances().await.unwrap() {
        restored.restore_instance(instance).await;
    }
    for task in persistence.list_service_tasks().await.unwrap() {
        restored.restore_service_task(task);
    }
    restored.reconcile_event_subprocesses().await.unwrap();
    let (base, server) = serve(restored, persistence).await;
    let response = client
        .put(format!("{base}/api/instances/{id}/variables"))
        .json(&json!({"variables":{"amount":151}}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);
    finish(&client, &base, "main", id).await;
    finish(&client, &base, "handler", id).await;
    finish(&client, &base, "handler", id).await;
    assert_eq!(
        get(&client, &base, &format!("/api/instances/{id}")).await["state"],
        "Completed"
    );
    let history = get(
        &client,
        &base,
        &format!("/api/instances/{id}/history?event_types=EventSubprocessActivated"),
    )
    .await;
    assert_eq!(history.as_array().unwrap().len(), 2);
    assert!(
        history
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["context"]["trigger_kind"] == "conditional"
                && entry["context"]["variable_revision"].is_number())
    );
    server.abort();
}

#[tokio::test]
async fn x9b_rest_native_and_camunda_completions_trigger_conditional_handlers() {
    for camunda in [false, true] {
        let persistence = Arc::new(InMemoryPersistence::new());
        let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
        let (base, server) = serve(engine, persistence).await;
        let client = reqwest::Client::new();
        let xml = xml(false).replace("<messageEventDefinition messageRef=\"trigger\"/>", "<conditionalEventDefinition><condition>mainResult == 42</condition></conditionalEventDefinition>").replace("isInterrupting=\"false\"", "isInterrupting=\"true\"");
        let deployed = post(
            &client,
            &base,
            "/api/deploy",
            json!({"xml":xml,"name":"conditional"}),
        )
        .await;
        let started = post(
            &client,
            &base,
            "/api/start",
            json!({"definition_key":deployed["definition_key"]}),
        )
        .await;
        let id = started["instance_id"].as_str().unwrap();
        if camunda {
            let tasks = post(&client, &base, "/engine-rest/external-task/fetchAndLock", json!({"workerId":"worker","maxTasks":1,"topics":[{"topicName":"main","lockDuration":60000}]})).await;
            let task = tasks[0]["id"].as_str().unwrap();
            let response = client.post(format!("{base}/engine-rest/external-task/{task}/complete")).json(&json!({"workerId":"worker","variables":{"mainResult":{"value":42,"type":"Integer"}}})).send().await.unwrap();
            assert_eq!(response.status(), 204, "{}", response.text().await.unwrap());
        } else {
            finish(&client, &base, "main", id).await;
        }
        finish(&client, &base, "handler", id).await;
        assert_eq!(
            get(&client, &base, &format!("/api/instances/{id}")).await["state"],
            "Completed"
        );
        server.abort();
    }
}

#[tokio::test]
async fn x2a_rest_conditional_wait_is_visible_and_consumed_once() {
    let persistence = Arc::new(InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let (base, server) = serve(engine, persistence).await;
    let client = reqwest::Client::new();
    let xml = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn"><process id="conditional_catch" isExecutable="true"><startEvent id="s"/><intermediateCatchEvent id="wait"><conditionalEventDefinition><condition>amount &gt; 100</condition></conditionalEventDefinition></intermediateCatchEvent><serviceTask id="work" camunda:topic="review"/><endEvent id="end"/><sequenceFlow id="a" sourceRef="s" targetRef="wait"/><sequenceFlow id="b" sourceRef="wait" targetRef="work"/><sequenceFlow id="c" sourceRef="work" targetRef="end"/></process></definitions>"#;
    let deployed = post(
        &client,
        &base,
        "/api/deploy",
        json!({"xml":xml,"name":"conditional_catch"}),
    )
    .await;
    assert_eq!(deployed["model_check"]["status"], "skipped");
    let started = post(
        &client,
        &base,
        "/api/start",
        json!({"definition_key":deployed["definition_key"]}),
    )
    .await;
    let id = started["instance_id"].as_str().unwrap();
    let waits = get(&client, &base, "/api/conditional-catches").await;
    assert_eq!(waits.as_array().unwrap().len(), 1);
    assert_eq!(waits[0]["instance_id"], id);
    assert_eq!(waits[0]["node_id"], "wait");
    assert_eq!(
        get(&client, &base, &format!("/api/instances/{id}")).await["state"]["WaitingOnConditional"]
            ["node_id"],
        "wait"
    );
    for amount in [150, 160] {
        assert_eq!(
            client
                .put(format!("{base}/api/instances/{id}/variables"))
                .json(&json!({"variables":{"amount":amount}}))
                .send()
                .await
                .unwrap()
                .status(),
            204
        );
    }
    assert_eq!(
        get(&client, &base, "/api/conditional-catches").await,
        json!([])
    );
    finish(&client, &base, "review", id).await;
    assert_eq!(
        get(&client, &base, &format!("/api/instances/{id}")).await["state"],
        "Completed"
    );
    server.abort();
}

#[tokio::test]
async fn x2a_rest_native_and_camunda_commits_release_a_parallel_catch() {
    for camunda in [false, true] {
        let persistence = Arc::new(InMemoryPersistence::new());
        let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
        let (base, server) = serve(engine, persistence).await;
        let client = reqwest::Client::new();
        let xml = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn"><process id="parallel_conditional" isExecutable="true"><startEvent id="s"/><parallelGateway id="fork"/><serviceTask id="main" camunda:topic="main"/><intermediateCatchEvent id="wait"><conditionalEventDefinition><condition>mainResult == 42</condition></conditionalEventDefinition></intermediateCatchEvent><serviceTask id="work" camunda:topic="review"/><endEvent id="end_main"/><endEvent id="end_review"/><sequenceFlow id="a" sourceRef="s" targetRef="fork"/><sequenceFlow id="b" sourceRef="fork" targetRef="main"/><sequenceFlow id="c" sourceRef="fork" targetRef="wait"/><sequenceFlow id="d" sourceRef="main" targetRef="end_main"/><sequenceFlow id="e" sourceRef="wait" targetRef="work"/><sequenceFlow id="f" sourceRef="work" targetRef="end_review"/></process></definitions>"#;
        let deployed = post(
            &client,
            &base,
            "/api/deploy",
            json!({"xml":xml,"name":"parallel_conditional"}),
        )
        .await;
        let started = post(
            &client,
            &base,
            "/api/start",
            json!({"definition_key":deployed["definition_key"]}),
        )
        .await;
        let id = started["instance_id"].as_str().unwrap();
        assert_eq!(
            get(&client, &base, "/api/conditional-catches")
                .await
                .as_array()
                .unwrap()
                .len(),
            1
        );
        if camunda {
            let tasks = post(&client, &base, "/engine-rest/external-task/fetchAndLock", json!({"workerId":"worker","maxTasks":1,"topics":[{"topicName":"main","lockDuration":60000}]})).await;
            let task = tasks[0]["id"].as_str().unwrap();
            let response = client.post(format!("{base}/engine-rest/external-task/{task}/complete")).json(&json!({"workerId":"worker","variables":{"mainResult":{"value":42,"type":"Integer"}}})).send().await.unwrap();
            assert_eq!(response.status(), 204, "{}", response.text().await.unwrap());
        } else {
            finish(&client, &base, "main", id).await;
        }
        assert_eq!(
            get(&client, &base, "/api/conditional-catches").await,
            json!([])
        );
        finish(&client, &base, "review", id).await;
        assert_eq!(
            get(&client, &base, &format!("/api/instances/{id}")).await["state"],
            "Completed"
        );
        server.abort();
    }
}

#[tokio::test]
async fn x2b_rest_boundary_is_visible_and_completion_outputs_close_its_subscription() {
    for camunda in [false, true] {
        let persistence = Arc::new(InMemoryPersistence::new());
        let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
        let (base, server) = serve(engine, persistence).await;
        let client = reqwest::Client::new();
        let xml = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:camunda="http://camunda.org/schema/1.0/bpmn"><process id="boundary_condition" isExecutable="true"><startEvent id="s"/><serviceTask id="main" camunda:topic="main"/><boundaryEvent id="boundary" attachedToRef="main"><conditionalEventDefinition><condition>mainResult == 42</condition></conditionalEventDefinition></boundaryEvent><serviceTask id="work" camunda:topic="review"/><endEvent id="end"/><sequenceFlow id="a" sourceRef="s" targetRef="main"/><sequenceFlow id="b" sourceRef="main" targetRef="end"/><sequenceFlow id="c" sourceRef="boundary" targetRef="work"/><sequenceFlow id="d" sourceRef="work" targetRef="end"/></process></definitions>"#;
        let deployed = post(
            &client,
            &base,
            "/api/deploy",
            json!({"xml":xml,"name":"boundary_condition"}),
        )
        .await;
        assert_eq!(deployed["model_check"]["status"], "skipped");
        let started = post(
            &client,
            &base,
            "/api/start",
            json!({"definition_key":deployed["definition_key"]}),
        )
        .await;
        let id = started["instance_id"].as_str().unwrap();
        let waits = get(&client, &base, "/api/conditional-catches").await;
        assert_eq!(waits.as_array().unwrap().len(), 1);
        assert_eq!(waits[0]["kind"], "boundary");
        assert_eq!(waits[0]["attached_to"], "main");
        if camunda {
            let tasks = post(&client, &base, "/engine-rest/external-task/fetchAndLock", json!({"workerId":"worker","maxTasks":1,"topics":[{"topicName":"main","lockDuration":60000}]})).await;
            let task = tasks[0]["id"].as_str().unwrap();
            let response = client.post(format!("{base}/engine-rest/external-task/{task}/complete")).json(&json!({"workerId":"worker","variables":{"mainResult":{"value":42,"type":"Integer"}}})).send().await.unwrap();
            assert_eq!(response.status(), 204, "{}", response.text().await.unwrap());
        } else {
            finish(&client, &base, "main", id).await;
        }
        assert_eq!(
            get(&client, &base, "/api/conditional-catches").await,
            json!([])
        );
        assert_eq!(
            get(&client, &base, &format!("/api/instances/{id}")).await["state"],
            "Completed"
        );
        server.abort();
    }
}
