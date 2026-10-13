use serde_json::Value;

async fn start_server() -> String {
    let app = engine_server::build_app();
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

const PARALLEL_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="ParallelProcess" isExecutable="true">
    <startEvent id="start" />
    <parallelGateway id="split" />
    <serviceTask id="t1" data-topic="worker1" />
    <serviceTask id="t2" data-topic="worker2" />
    <parallelGateway id="join" />
    <endEvent id="end" />
    
    <sequenceFlow id="f1" sourceRef="start" targetRef="split" />
    <sequenceFlow id="f2" sourceRef="split" targetRef="t1" />
    <sequenceFlow id="f3" sourceRef="split" targetRef="t2" />
    <sequenceFlow id="f4" sourceRef="t1" targetRef="join" />
    <sequenceFlow id="f5" sourceRef="t2" targetRef="join" />
    <sequenceFlow id="f6" sourceRef="join" targetRef="end" />
  </process>
</definitions>"#;

#[tokio::test]
async fn deploy_and_execute_parallel_gateway() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // Deploy
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": PARALLEL_XML, "name": "Parallel" }))
        .send()
        .await
        .expect("deploy request failed");

    let text = res.text().await.unwrap();
    assert!(
        text.contains("definition_key"),
        "deploy failed with response: {}",
        text
    );
    let body: Value = serde_json::from_str(&text).unwrap();
    let def_key = body["definition_key"].as_str().unwrap();

    // Start instance
    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .expect("start failed");
    assert_eq!(res.status(), 200, "start instance failed");

    let body: Value = res.json().await.unwrap();
    let instance_id = body["instance_id"].as_str().unwrap();

    // Fetch and complete t1
    let fetch_res = client.post(format!("{}/api/service-task/fetchAndLock", base))
        .json(&serde_json::json!({ "workerId": "w1", "maxTasks": 1, "topics": [{ "topicName": "worker1", "lockDuration": 1000 }] }))
        .send().await.expect("fetch failed");
    assert_eq!(fetch_res.status(), 200, "fetch w1 failed");
    let tasks: Vec<Value> = fetch_res.json().await.unwrap();
    assert_eq!(tasks.len(), 1, "Expected 1 task for worker1");
    let t1_id = tasks[0]["id"].as_str().unwrap();

    client
        .post(format!("{}/api/service-task/{}/complete", base, t1_id))
        .json(&serde_json::json!({ "workerId": "w1" }))
        .send()
        .await
        .expect("complete t1 failed");

    // Fetch and complete t2
    let fetch_res = client.post(format!("{}/api/service-task/fetchAndLock", base))
        .json(&serde_json::json!({ "workerId": "w2", "maxTasks": 1, "topics": [{ "topicName": "worker2", "lockDuration": 1000 }] }))
        .send().await.unwrap();
    assert_eq!(fetch_res.status(), 200, "fetch w2 failed");
    let tasks: Vec<Value> = fetch_res.json().await.unwrap();
    assert_eq!(tasks.len(), 1, "Expected 1 task for worker2");
    let t2_id = tasks[0]["id"].as_str().unwrap();

    client
        .post(format!("{}/api/service-task/{}/complete", base, t2_id))
        .json(&serde_json::json!({ "workerId": "w2" }))
        .send()
        .await
        .expect("complete t2 failed");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Check complete
    let details_res = client
        .get(format!("{}/api/instances/{}", base, instance_id))
        .send()
        .await
        .unwrap();
    let details: Value = details_res.json().await.unwrap();
    assert_eq!(details["state"], "Completed");
}

const LINK_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="LinkProcess" isExecutable="true">
    <startEvent id="start" />
    <exclusiveGateway id="gw" default="f_small" />
    <intermediateThrowEvent id="to_big"><linkEventDefinition name="big" /></intermediateThrowEvent>
    <intermediateThrowEvent id="to_small"><linkEventDefinition name="small" /></intermediateThrowEvent>
    <intermediateCatchEvent id="from_big"><linkEventDefinition name="big" /></intermediateCatchEvent>
    <intermediateCatchEvent id="from_small"><linkEventDefinition name="small" /></intermediateCatchEvent>
    <endEvent id="end_big" />
    <endEvent id="end_small" />

    <sequenceFlow id="f1" sourceRef="start" targetRef="gw" />
    <sequenceFlow id="f_big" sourceRef="gw" targetRef="to_big">
      <conditionExpression>amount &gt; 100</conditionExpression>
    </sequenceFlow>
    <sequenceFlow id="f_small" sourceRef="gw" targetRef="to_small" />
    <sequenceFlow id="f2" sourceRef="from_big" targetRef="end_big" />
    <sequenceFlow id="f3" sourceRef="from_small" targetRef="end_small" />
  </process>
</definitions>"#;

#[tokio::test]
async fn link_events_route_to_matching_catch() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": LINK_XML, "name": "Link" }))
        .send()
        .await
        .expect("deploy request failed");
    let text = res.text().await.unwrap();
    assert!(text.contains("definition_key"), "deploy failed: {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let def_key = body["definition_key"].as_str().unwrap().to_string();

    for (amount, expected_end) in [(500, "end_big"), (5, "end_small")] {
        let res = client
            .post(format!("{}/api/start", base))
            .json(&serde_json::json!({
                "definition_key": def_key,
                "variables": { "amount": amount }
            }))
            .send()
            .await
            .expect("start failed");
        assert_eq!(res.status(), 200, "start instance failed");
        let body: Value = res.json().await.unwrap();
        let instance_id = body["instance_id"].as_str().unwrap();

        let details: Value = client
            .get(format!("{}/api/instances/{}", base, instance_id))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(details["state"], "Completed");
        assert_eq!(details["current_node"], expected_end, "amount={amount}");
    }
}
