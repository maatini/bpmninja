//! E2E-Tests für Call-Activity-Workflows:
//! - Eltern-Prozess ruft Kind-Prozess per <callActivity calledElement="..."> auf
//! - Eltern wartet bis Kind abgeschlossen ist (WaitingOnCallActivity)
//! - Variablen des Kinds werden in den Eltern propagiert
//! - Eltern schließt automatisch ab, sobald Kind fertig ist

use serde_json::Value;

/// Kind-Prozess: einfacher Workflow mit einem UserTask
const CHILD_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_child" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="ChildProcess" isExecutable="true">
    <startEvent id="start" />
    <userTask id="child_task" data-assignee="child_worker" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="child_task" />
    <sequenceFlow id="f2" sourceRef="child_task" targetRef="end" />
  </process>
</definitions>"#;

/// Eltern-Prozess: ruft ChildProcess per CallActivity auf
const PARENT_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_parent" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="ParentProcess" isExecutable="true">
    <startEvent id="start" />
    <callActivity id="call" calledElement="ChildProcess" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="call" />
    <sequenceFlow id="f2" sourceRef="call" targetRef="end" />
  </process>
</definitions>"#;

/// Kind-Prozess mit ServiceTask statt UserTask
const CHILD_SERVICE_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_child_svc" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="ChildServiceProcess" isExecutable="true">
    <startEvent id="start" />
    <serviceTask id="child_svc" data-topic="child_topic" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="child_svc" />
    <sequenceFlow id="f2" sourceRef="child_svc" targetRef="end" />
  </process>
</definitions>"#;

/// Eltern-Prozess der ChildServiceProcess aufruft
const PARENT_SERVICE_BPMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_parent_svc" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="ParentServiceProcess" isExecutable="true">
    <startEvent id="start" />
    <callActivity id="call" calledElement="ChildServiceProcess" />
    <endEvent id="end" />
    <sequenceFlow id="f1" sourceRef="start" targetRef="call" />
    <sequenceFlow id="f2" sourceRef="call" targetRef="end" />
  </process>
</definitions>"#;

async fn start_server() -> String {
    let app = engine_server::build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fehlgeschlagen");
    let addr = listener.local_addr().expect("addr fehlgeschlagen");
    let base = format!("http://{}", addr);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

/// Hilfsfunktion: BPMN-XML deployen und Prozess starten, gibt (def_key, instance_id) zurück
async fn deploy_and_start(base: &str, client: &reqwest::Client, xml: &str) -> (String, String) {
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": xml, "name": "test" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let def_key = body["definition_key"].as_str().unwrap().to_string();

    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    let inst_id = body["instance_id"].as_str().unwrap().to_string();

    (def_key, inst_id)
}

/// Hilfsfunktion: Prozess-Definition deployen ohne zu starten
async fn deploy(base: &str, client: &reqwest::Client, xml: &str) -> String {
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": xml, "name": "test" }))
        .send()
        .await
        .unwrap();
    let body: Value = res.json().await.unwrap();
    body["definition_key"].as_str().unwrap().to_string()
}

/// Eltern-Prozess wartet auf Call-Activity und Kind-Prozess läuft als Unterinstanz
#[tokio::test]
async fn call_activity_eltern_wartet_auf_kind() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // Kind zuerst deployen, dann Eltern
    deploy(&base, &client, CHILD_BPMN).await;
    let (_, parent_id) = deploy_and_start(&base, &client, PARENT_BPMN).await;

    // Eltern-Instanz abrufen → muss auf Call-Activity warten
    let res = client
        .get(format!("{}/api/instances/{}", base, parent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let parent: Value = res.json().await.unwrap();

    assert!(
        parent["state"]["WaitingOnCallActivity"].is_object(),
        "Eltern-Prozess muss auf Call-Activity warten: {:?}",
        parent["state"]
    );
}

/// Kind-Instanz ist eine Unterinstanz des Eltern-Prozesses
#[tokio::test]
async fn call_activity_kind_ist_unterinstanz() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    deploy(&base, &client, CHILD_BPMN).await;
    let (_, parent_id) = deploy_and_start(&base, &client, PARENT_BPMN).await;

    // Alle Instanzen abrufen → Kind und Eltern müssen existieren
    let res = client
        .get(format!("{}/api/instances", base))
        .send()
        .await
        .unwrap();
    let instances: Vec<Value> = res.json().await.unwrap();
    assert_eq!(instances.len(), 2, "Es müssen genau 2 Instanzen existieren (Eltern + Kind)");

    // Kind-Instanz finden: hat parent_instance_id gesetzt
    let kind = instances
        .iter()
        .find(|i| i["parent_instance_id"].is_string())
        .expect("Kind-Instanz mit parent_instance_id muss existieren");

    assert_eq!(
        kind["parent_instance_id"].as_str().unwrap(),
        parent_id,
        "Kind muss die Eltern-ID als parent_instance_id haben"
    );
}

/// Nach Abschluss des Kind-UserTasks werden Variablen in den Eltern propagiert
#[tokio::test]
async fn call_activity_variablen_propagation() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    deploy(&base, &client, CHILD_BPMN).await;
    let (_, parent_id) = deploy_and_start(&base, &client, PARENT_BPMN).await;

    // UserTask des Kinds abrufen
    let res = client
        .get(format!("{}/api/tasks", base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let tasks: Vec<Value> = res.json().await.unwrap();
    assert_eq!(tasks.len(), 1, "Genau ein UserTask muss ausstehen");
    let task_id = tasks[0]["task_id"].as_str().unwrap().to_string();

    // UserTask mit Variable abschließen
    let res = client
        .post(format!("{}/api/complete/{}", base, task_id))
        .json(&serde_json::json!({
            "variables": {
                "ergebnis_aus_kind": "hallo eltern"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "Complete UserTask muss 204 zurückgeben");

    // Eltern-Instanz muss jetzt abgeschlossen sein
    let res = client
        .get(format!("{}/api/instances/{}", base, parent_id))
        .send()
        .await
        .unwrap();
    let parent: Value = res.json().await.unwrap();

    assert_eq!(
        parent["state"].as_str().unwrap_or(&parent["state"].to_string()),
        "Completed",
        "Eltern-Prozess muss nach Kind-Abschluss Completed sein: {:?}",
        parent["state"]
    );

    // Variable muss im Eltern-Prozess ankommen
    assert_eq!(
        parent["variables"]["ergebnis_aus_kind"].as_str().unwrap(),
        "hallo eltern",
        "Variable aus Kind muss im Eltern-Prozess verfügbar sein"
    );
}

/// Eltern-Prozess schließt automatisch ab, sobald Kind fertig ist
#[tokio::test]
async fn call_activity_eltern_abschluss_nach_kind() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    deploy(&base, &client, CHILD_BPMN).await;
    let (_, parent_id) = deploy_and_start(&base, &client, PARENT_BPMN).await;

    // Kind-UserTask abschließen (ohne extra Variablen)
    let res = client
        .get(format!("{}/api/tasks", base))
        .send()
        .await
        .unwrap();
    let tasks: Vec<Value> = res.json().await.unwrap();
    let task_id = tasks[0]["task_id"].as_str().unwrap().to_string();

    client
        .post(format!("{}/api/complete/{}", base, task_id))
        .json(&serde_json::json!({ "variables": {} }))
        .send()
        .await
        .unwrap();

    // Eltern muss Completed sein
    let res = client
        .get(format!("{}/api/instances/{}", base, parent_id))
        .send()
        .await
        .unwrap();
    let parent: Value = res.json().await.unwrap();
    assert_eq!(
        parent["state"].as_str().unwrap_or(&parent["state"].to_string()),
        "Completed",
        "Eltern-Prozess muss Completed sein: {:?}",
        parent["state"]
    );

    // Keine offenen UserTasks mehr
    let res = client
        .get(format!("{}/api/tasks", base))
        .send()
        .await
        .unwrap();
    let tasks: Vec<Value> = res.json().await.unwrap();
    assert_eq!(tasks.len(), 0, "Nach Abschluss darf kein UserTask mehr offen sein");
}

/// Call-Activity mit Kind-ServiceTask: Eltern wartet, Kind-ServiceTask wird per fetchAndLock bearbeitet
#[tokio::test]
async fn call_activity_mit_kind_service_task() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    deploy(&base, &client, CHILD_SERVICE_BPMN).await;
    let (_, parent_id) = deploy_and_start(&base, &client, PARENT_SERVICE_BPMN).await;

    // Eltern muss warten
    let res = client
        .get(format!("{}/api/instances/{}", base, parent_id))
        .send()
        .await
        .unwrap();
    let parent: Value = res.json().await.unwrap();
    assert!(
        parent["state"]["WaitingOnCallActivity"].is_object(),
        "Eltern muss auf Call-Activity warten: {:?}",
        parent["state"]
    );

    // Kind-ServiceTask per fetchAndLock abholen
    let res = client
        .post(format!("{}/api/service-task/fetchAndLock", base))
        .json(&serde_json::json!({
            "workerId": "test_worker",
            "maxTasks": 1,
            "topics": [{ "topicName": "child_topic", "lockDuration": 30000 }]
        }))
        .send()
        .await
        .unwrap();
    let service_tasks: Vec<Value> = res.json().await.unwrap();
    assert_eq!(service_tasks.len(), 1, "Genau ein ServiceTask des Kinds muss verfügbar sein");
    let svc_task_id = service_tasks[0]["id"].as_str().unwrap().to_string();

    // ServiceTask abschließen
    let res = client
        .post(format!("{}/api/service-task/{}/complete", base, svc_task_id))
        .json(&serde_json::json!({
            "workerId": "test_worker",
            "variables": { "svc_ergebnis": 42 }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204, "ServiceTask-Complete muss 204 zurückgeben");

    // Eltern muss jetzt Completed sein
    let res = client
        .get(format!("{}/api/instances/{}", base, parent_id))
        .send()
        .await
        .unwrap();
    let parent: Value = res.json().await.unwrap();
    assert_eq!(
        parent["state"].as_str().unwrap_or(&parent["state"].to_string()),
        "Completed",
        "Eltern-Prozess muss nach Kind-ServiceTask-Abschluss Completed sein: {:?}",
        parent["state"]
    );
    assert_eq!(
        parent["variables"]["svc_ergebnis"].as_i64().unwrap(),
        42,
        "ServiceTask-Variable muss im Eltern-Prozess verfügbar sein"
    );
}
