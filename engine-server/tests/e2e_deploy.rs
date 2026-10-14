//! E2E test: Deploy a minimal BPMN workflow (StartEvent → EndEvent)
//! and verify the instance completes immediately.

use serde_json::Value;

/// Minimal BPMN 2.0 XML with only a StartEvent, EndEvent, and one SequenceFlow.
const MINIMAL_BPMN_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="MinimalProcess">
    <startEvent id="Start_1" />
    <endEvent id="End_1" />
    <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="End_1" />
  </process>
</definitions>"#;

/// Helper: starts the engine-server on a random port and returns the base URL.
async fn start_server() -> String {
    let app = engine_server::build_app();

    // Bind to port 0 → OS assigns a free port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr = listener.local_addr().expect("failed to get local addr");
    let base_url = format!("http://{}", addr);

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server error");
    });

    base_url
}

#[tokio::test]
async fn deploy_minimal_workflow_returns_definition_id() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // Deploy the minimal BPMN process
    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({
            "xml": MINIMAL_BPMN_XML,
            "name": "MinimalProcess"
        }))
        .send()
        .await
        .expect("deploy request failed");

    assert_eq!(res.status(), 200, "deploy should return 200 OK");

    let body: Value = res.json().await.expect("failed to parse deploy response");
    let def_key = body["definition_key"]
        .as_str()
        .expect("response should contain definition_key");

    // The returned key should be a valid UUID
    assert!(
        uuid::Uuid::parse_str(def_key).is_ok(),
        "definition_key should be a valid UUID"
    );
}

#[tokio::test]
async fn deploy_and_start_minimal_workflow_completes_immediately() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // Step 1: Deploy
    let deploy_res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({
            "xml": MINIMAL_BPMN_XML,
            "name": "MinimalProcess"
        }))
        .send()
        .await
        .expect("deploy request failed");

    assert_eq!(deploy_res.status(), 200);
    let deploy_body: Value = deploy_res.json().await.expect("parse deploy response");
    let def_key = deploy_body["definition_key"]
        .as_str()
        .expect("definition_key missing")
        .to_string();

    // Step 2: Start instance
    let start_res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key }))
        .send()
        .await
        .expect("start request failed");

    assert_eq!(start_res.status(), 200, "start should return 200 OK");

    let start_body: Value = start_res.json().await.expect("parse start response");
    let instance_id = start_body["instance_id"]
        .as_str()
        .expect("instance_id missing")
        .to_string();

    // Step 3: Verify instance is Completed (Start → End = immediate)
    let details_res = client
        .get(format!("{}/api/instances/{}", base, instance_id))
        .send()
        .await
        .expect("get instance request failed");

    assert_eq!(details_res.status(), 200, "get instance should return 200");

    let details: Value = details_res.json().await.expect("parse instance details");
    assert_eq!(
        details["state"], "Completed",
        "instance should be Completed after Start→End workflow"
    );
    assert_eq!(
        details["definition_key"].as_str().map(|s| s.to_string()),
        Some(def_key),
        "definition_key should match the deployed key"
    );
}

#[tokio::test]
async fn deploy_invalid_xml_returns_400() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({
            "xml": "<invalid>not valid bpmn</invalid>",
            "name": "BadProcess"
        }))
        .send()
        .await
        .expect("deploy request failed");

    assert_eq!(
        res.status(),
        400,
        "deploying invalid XML should return 400 BAD REQUEST"
    );
}

/// Regression: an embedded sub-process from BPMN XML used to park every
/// instance on a "noop" service task at the sub-process start.
#[tokio::test]
async fn xml_embedded_subprocess_runs_to_completion() {
    let xml = r#"<definitions id="d" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
      <process id="sub_flow" isExecutable="true">
        <startEvent id="s" />
        <subProcess id="sp">
          <startEvent id="sp_s" />
          <scriptTask id="inner" scriptFormat="rhai"><script>let visited = true;</script></scriptTask>
          <endEvent id="sp_e" />
          <sequenceFlow id="a" sourceRef="sp_s" targetRef="inner" />
          <sequenceFlow id="b" sourceRef="inner" targetRef="sp_e" />
        </subProcess>
        <endEvent id="e" />
        <sequenceFlow id="f1" sourceRef="s" targetRef="sp" />
        <sequenceFlow id="f2" sourceRef="sp" targetRef="e" />
      </process>
    </definitions>"#;
    let def = bpmn_parser::parse_bpmn_xml(xml).unwrap();
    let engine = engine_core::WorkflowEngine::new();
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();

    assert_eq!(
        engine.get_instance_state(id).await.unwrap(),
        engine_core::InstanceState::Completed
    );
    assert!(engine.get_pending_service_tasks().is_empty());
    let inst = engine.get_instance_details(id).await.unwrap();
    assert_eq!(
        inst.variables.get("visited"),
        Some(&serde_json::json!(true))
    );
}

/// X15: bpmn-js style export with pool, lanes, group/category, text annotation,
/// association, data object and data store (incl. data associations) and DI.
const SWIMLANES_ARTIFACTS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" xmlns:dc="http://www.omg.org/spec/DD/20100524/DC" xmlns:di="http://www.omg.org/spec/DD/20100524/DI" id="Definitions_X15" targetNamespace="http://bpmn.io/schema/bpmn">
  <bpmn:collaboration id="Collaboration_1">
    <bpmn:participant id="Participant_1" name="Order handling" processRef="SwimlaneProcess" />
    <bpmn:group id="Group_1" categoryValueRef="CategoryValue_1" />
  </bpmn:collaboration>
  <bpmn:category id="Category_1">
    <bpmn:categoryValue id="CategoryValue_1" value="Automated part" />
  </bpmn:category>
  <bpmn:dataStore id="DataStore_1" name="Orders DB" />
  <bpmn:process id="SwimlaneProcess" isExecutable="true">
    <bpmn:laneSet id="LaneSet_1">
      <bpmn:lane id="Lane_Sales" name="Sales">
        <bpmn:flowNodeRef>start</bpmn:flowNodeRef>
        <bpmn:flowNodeRef>calc</bpmn:flowNodeRef>
      </bpmn:lane>
      <bpmn:lane id="Lane_Done" name="Archive">
        <bpmn:flowNodeRef>end</bpmn:flowNodeRef>
      </bpmn:lane>
    </bpmn:laneSet>
    <bpmn:startEvent id="start"><bpmn:outgoing>f1</bpmn:outgoing></bpmn:startEvent>
    <bpmn:scriptTask id="calc" scriptFormat="rhai">
      <bpmn:incoming>f1</bpmn:incoming>
      <bpmn:outgoing>f2</bpmn:outgoing>
      <bpmn:property id="Property_1" name="__targetRef_placeholder" />
      <bpmn:dataInputAssociation id="DataIn_1">
        <bpmn:sourceRef>DataObjectReference_1</bpmn:sourceRef>
        <bpmn:targetRef>Property_1</bpmn:targetRef>
      </bpmn:dataInputAssociation>
      <bpmn:property id="Property_Total" name="total" />
      <bpmn:dataOutputAssociation id="DataOut_1">
        <bpmn:sourceRef>Property_Total</bpmn:sourceRef>
        <bpmn:targetRef>DataObjectReference_1</bpmn:targetRef>
      </bpmn:dataOutputAssociation>
      <bpmn:script>let total = 40 + 2;</bpmn:script>
    </bpmn:scriptTask>
    <bpmn:endEvent id="end"><bpmn:incoming>f2</bpmn:incoming></bpmn:endEvent>
    <bpmn:sequenceFlow id="f1" sourceRef="start" targetRef="calc" />
    <bpmn:sequenceFlow id="f2" sourceRef="calc" targetRef="end" />
    <bpmn:dataObjectReference id="DataObjectReference_1" name="Order" dataObjectRef="DataObject_1" />
    <bpmn:dataObject id="DataObject_1" />
    <bpmn:dataStoreReference id="DataStoreReference_1" dataStoreRef="DataStore_1" />
    <bpmn:textAnnotation id="TextAnnotation_1"><bpmn:text>Computes the order total</bpmn:text></bpmn:textAnnotation>
    <bpmn:association id="Association_1" sourceRef="calc" targetRef="TextAnnotation_1" />
  </bpmn:process>
  <bpmndi:BPMNDiagram id="BPMNDiagram_1">
    <bpmndi:BPMNPlane id="BPMNPlane_1" bpmnElement="Collaboration_1">
      <bpmndi:BPMNShape id="Participant_1_di" bpmnElement="Participant_1" isHorizontal="true">
        <dc:Bounds x="100" y="80" width="600" height="250" />
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Lane_Sales_di" bpmnElement="Lane_Sales" isHorizontal="true">
        <dc:Bounds x="130" y="80" width="570" height="125" />
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Lane_Done_di" bpmnElement="Lane_Done" isHorizontal="true">
        <dc:Bounds x="130" y="205" width="570" height="125" />
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="start_di" bpmnElement="start"><dc:Bounds x="172" y="122" width="36" height="36" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="calc_di" bpmnElement="calc"><dc:Bounds x="260" y="100" width="100" height="80" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="end_di" bpmnElement="end"><dc:Bounds x="412" y="252" width="36" height="36" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Group_1_di" bpmnElement="Group_1"><dc:Bounds x="240" y="90" width="140" height="100" /></bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="TextAnnotation_1_di" bpmnElement="TextAnnotation_1"><dc:Bounds x="400" y="100" width="150" height="30" /></bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="f1_di" bpmnElement="f1"><di:waypoint x="208" y="140" /><di:waypoint x="260" y="140" /></bpmndi:BPMNEdge>
      <bpmndi:BPMNEdge id="f2_di" bpmnElement="f2"><di:waypoint x="310" y="180" /><di:waypoint x="430" y="252" /></bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</bpmn:definitions>"#;

#[tokio::test]
async fn swimlanes_and_artifacts_deploy_run_and_export_unchanged() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let res = client
        .post(format!("{}/api/deploy", base))
        .json(&serde_json::json!({ "xml": SWIMLANES_ARTIFACTS_XML, "name": "Swimlanes" }))
        .send()
        .await
        .unwrap();
    let text = res.text().await.unwrap();
    assert!(text.contains("definition_key"), "deploy failed: {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let def_key = body["definition_key"].as_str().unwrap().to_string();

    // Export returns the deployed document byte for byte (lanes, artifacts, DI).
    let exported = client
        .get(format!("{}/api/definitions/{}/xml", base, def_key))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(exported, SWIMLANES_ARTIFACTS_XML);

    // Visual and data elements do not change the flow.
    let res = client
        .post(format!("{}/api/start", base))
        .json(&serde_json::json!({ "definition_key": def_key, "variables": { "DataObject_1": {} } }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
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
    assert_eq!(details["variables"]["DataObject_1"], 42);
}
