//! E2E: GET /api/instances Pagination (limit/offset + X-Total-Count).

use serde_json::Value;

const USER_TASK_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="PageProcess" isExecutable="true">
    <startEvent id="start" />
    <userTask id="task" data-assignee="tester" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="task" />
    <sequenceFlow id="f2" sourceRef="task" targetRef="end" />
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

async fn deploy_and_start_n(base: &str, client: &reqwest::Client, n: usize) {
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": USER_TASK_BPMN, "name": "page" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let def_key = body["definition_key"].as_str().unwrap().to_string();

    for _ in 0..n {
        let res = client
            .post(format!("{}/api/start", base))
            .json(&serde_json::json!({ "definition_key": def_key }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "start should return 200");
    }
}

#[tokio::test]
async fn list_instances_page_sets_x_total_count() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    deploy_and_start_n(&base, &client, 3).await;

    let res = client
        .get(format!("{}/api/instances?limit=1&offset=0", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let total = res
        .headers()
        .get("x-total-count")
        .expect("X-Total-Count header");
    assert_eq!(total.to_str().unwrap(), "3");
    let body: Vec<Value> = res.json().await.unwrap();
    assert_eq!(body.len(), 1);
}

#[tokio::test]
async fn list_instances_without_query_is_full_array_without_total_header() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    deploy_and_start_n(&base, &client, 3).await;

    let res = client
        .get(format!("{}/api/instances", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert!(
        res.headers().get("x-total-count").is_none(),
        "ohne Query-Params darf X-Total-Count nicht gesetzt sein"
    );
    let body: Vec<Value> = res.json().await.unwrap();
    assert_eq!(body.len(), 3);
}
