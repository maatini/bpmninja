//! X12 collection deployment: validation, commit, version identity and recovery.
use engine_core::{WorkflowEngine, WorkflowPersistence};
use engine_server::{AppBuildConfig, ModelCheckMode};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

fn xml() -> String {
    r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" id="defs">
    <process id="first" isExecutable="true"><startEvent id="s1"/><userTask id="t1"/><endEvent id="e1"/><sequenceFlow id="f1" sourceRef="s1" targetRef="t1"/><sequenceFlow id="f2" sourceRef="t1" targetRef="e1"/></process>
    <process id="second" isExecutable="true"><startEvent id="s2"/><endEvent id="e2"/><sequenceFlow id="f3" sourceRef="s2" targetRef="e2"/></process>
    <collaboration id="c"><participant id="p1" processRef="first"/><participant id="p2" processRef="second"/></collaboration>
    </definitions>"#.into()
}

async fn serve(
    engine: Arc<WorkflowEngine>,
    persistence: Option<Arc<dyn WorkflowPersistence>>,
    mode: ModelCheckMode,
) -> String {
    let app = engine_server::build_app_with_config(
        engine,
        persistence,
        HashMap::new(),
        None,
        Arc::new(engine_server::LogBuffer::new()),
        AppBuildConfig {
            require_nats: Some(false),
            api_key: Some(None),
            model_check: Some(mode),
            model_check_timeout_ms: Some(10_000),
            ..Default::default()
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}
async fn deploy(base: &str, xml: &str) -> (u16, Value) {
    let response = reqwest::Client::new()
        .post(format!("{base}/api/deploy/collection"))
        .json(&json!({"xml":xml,"name":"pools.bpmn"}))
        .send()
        .await
        .unwrap();
    (response.status().as_u16(), response.json().await.unwrap())
}

#[tokio::test]
async fn collection_preserves_order_versions_xml_and_metadata() {
    let persistence = Arc::new(engine_core::adapter::InMemoryPersistence::new());
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let base = serve(
        engine.clone(),
        Some(persistence.clone()),
        ModelCheckMode::Enforce,
    )
    .await;
    let (status, first) = deploy(&base, &xml()).await;
    assert_eq!(status, 200, "{first}");
    assert_eq!(first["processes"][0]["process_id"], "first");
    assert_eq!(first["processes"][1]["process_id"], "second");
    for process in first["processes"].as_array().unwrap() {
        assert_eq!(process["model_check"]["status"], "passed", "{first}");
        assert_eq!(process["version"], 1);
        let key = process["definition_key"].as_str().unwrap();
        assert_eq!(persistence.load_bpmn_xml(key).await.unwrap(), xml());
    }
    let (status, second) = deploy(&base, &xml()).await;
    assert_eq!(status, 200, "{second}");
    assert_eq!(second["processes"][0]["version"], 2);
    assert_ne!(
        first["processes"][0]["definition_key"],
        second["processes"][0]["definition_key"]
    );
    let commits = persistence.list_definition_collections().await.unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].collaborations[0]["id"], "c");
    let restored = WorkflowEngine::new();
    for commit in &commits {
        restored.restore_definition_collection(commit).unwrap();
    }
    assert_eq!(restored.list_definitions().await.len(), 4);
    for (_, id, version, _) in engine.list_definitions().await {
        assert!(
            restored
                .list_definition_versions(&id)
                .await
                .iter()
                .any(|(_, v, _)| *v == version)
        );
    }
    engine.shutdown().await;
}

#[tokio::test]
async fn later_invalid_pool_and_no_executable_pool_publish_nothing() {
    let engine = Arc::new(WorkflowEngine::new());
    let base = serve(engine.clone(), None, ModelCheckMode::Off).await;
    let bad = xml().replace("targetRef=\"e2\"", "targetRef=\"missing\"");
    assert_eq!(deploy(&base, &bad).await.0, 400);
    assert_eq!(
        deploy(
            &base,
            &xml().replace("isExecutable=\"true\"", "isExecutable=\"false\"")
        )
        .await
        .0,
        400
    );
    assert!(engine.list_definitions().await.is_empty());
}

#[tokio::test]
async fn later_pool_model_check_violation_rejects_whole_batch() {
    let engine = Arc::new(WorkflowEngine::new());
    let base = serve(engine.clone(), None, ModelCheckMode::Enforce).await;
    let bad = xml().replace("<endEvent id=\"e2\"/>", "<parallelGateway id=\"join\"/><endEvent id=\"e2\"/><userTask id=\"unreachable\"/><sequenceFlow id=\"blocked\" sourceRef=\"unreachable\" targetRef=\"join\"/><sequenceFlow id=\"last\" sourceRef=\"join\" targetRef=\"e2\"/>")
        .replace("sourceRef=\"s2\" targetRef=\"e2\"", "sourceRef=\"s2\" targetRef=\"join\"");
    let (status, body) = deploy(&base, &bad).await;
    assert_eq!(status, 400, "{body}");
    assert!(body["error"].as_str().unwrap().contains("second"), "{body}");
    assert!(engine.list_definitions().await.is_empty());
}

#[tokio::test]
async fn concurrent_collections_get_distinct_versions() {
    let engine = Arc::new(WorkflowEngine::new());
    let base = serve(engine.clone(), None, ModelCheckMode::Off).await;
    let input = xml();
    let (a, b) = tokio::join!(deploy(&base, &input), deploy(&base, &input));
    assert_eq!(a.0, 200);
    assert_eq!(b.0, 200);
    let versions: Vec<_> = engine
        .list_definition_versions("first")
        .await
        .into_iter()
        .map(|(_, v, _)| v)
        .collect();
    assert_eq!(versions, vec![1, 2]);
}

struct NatsServer {
    port: u16,
    child: std::process::Child,
    directory: std::path::PathBuf,
}
impl Drop for NatsServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
impl NatsServer {
    async fn restart(&mut self) -> Arc<persistence_nats::NatsPersistence> {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
        self.child = std::process::Command::new("nats-server")
            .args([
                "-js",
                "-a",
                "127.0.0.1",
                "-p",
                &self.port.to_string(),
                "-sd",
            ])
            .arg(&self.directory)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        for _ in 0..50 {
            if tokio::net::TcpStream::connect(("127.0.0.1", self.port))
                .await
                .is_ok()
            {
                return Arc::new(
                    persistence_nats::NatsPersistence::connect(
                        &format!("nats://127.0.0.1:{}", self.port),
                        "COLLECTION_TEST",
                    )
                    .await
                    .unwrap(),
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("NATS restart timed out");
    }
}
async fn nats() -> (NatsServer, Arc<persistence_nats::NatsPersistence>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let directory =
        std::env::temp_dir().join(format!("bpmninja-collection-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let child = std::process::Command::new("nats-server")
        .args(["-js", "-a", "127.0.0.1", "-p", &port.to_string(), "-sd"])
        .arg(&directory)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("nats-server is required for collection recovery tests");
    let server = NatsServer {
        port,
        child,
        directory,
    };
    let url = format!("nats://127.0.0.1:{port}");
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return (
                server,
                Arc::new(
                    persistence_nats::NatsPersistence::connect(&url, "COLLECTION_TEST")
                        .await
                        .unwrap(),
                ),
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("NATS startup timed out");
}

#[tokio::test]
async fn jetstream_commit_recovery_deletion_and_corruption() {
    let (mut server, persistence) = nats().await;
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let base = serve(
        engine.clone(),
        Some(persistence.clone()),
        ModelCheckMode::Off,
    )
    .await;
    let (status, response) = deploy(&base, &xml()).await;
    assert_eq!(status, 200, "{response}");
    let key: uuid::Uuid = response["processes"][0]["definition_key"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    // Interleave a legacy v2, collection v3, and legacy v4. Startup must not
    // close the version gaps before restoring collection records.
    let mut legacy = bpmn_parser::parse_bpmn_xml(&xml()).unwrap();
    let (legacy_key, version) = engine.deploy_definition(legacy.clone()).await;
    assert_eq!(version, 2);
    persistence
        .save_bpmn_xml(&legacy_key.to_string(), &xml())
        .await
        .unwrap();
    let (status, third) = deploy(&base, &xml()).await;
    assert_eq!(status, 200, "{third}");
    assert_eq!(third["processes"][0]["version"], 3);
    legacy.key = uuid::Uuid::new_v4();
    let (fourth_key, version) = engine.deploy_definition(legacy).await;
    assert_eq!(version, 4);
    persistence
        .save_bpmn_xml(&fourth_key.to_string(), &xml())
        .await
        .unwrap();
    let persistence = server.restart().await;
    // A durable commit exists independently of the original engine's memory.
    for _ in 0..2 {
        let mut restored = WorkflowEngine::new();
        let mut cache = HashMap::new();
        let stats = engine_server::StartupCoordinator::new(persistence.clone())
            .restore(&mut restored, &mut cache)
            .await
            .unwrap();
        assert_eq!(stats.definitions, 6);
        assert_eq!(
            restored.get_definition(&fourth_key).await.unwrap().version,
            4
        );
        assert_eq!(restored.get_definition(&key).await.unwrap().version, 1);
        assert_eq!(cache[&key.to_string()], xml());
    }
    engine.delete_definition(key, false).await.unwrap();
    let mut restored = WorkflowEngine::new();
    let mut cache = HashMap::new();
    let stats = engine_server::StartupCoordinator::new(persistence.clone())
        .restore(&mut restored, &mut cache)
        .await
        .unwrap();
    assert_eq!(stats.definitions, 5);
    assert!(restored.get_definition(&key).await.is_none());
    let store = persistence
        .jetstream()
        .get_key_value("definition_collections")
        .await
        .unwrap();
    store.put("corrupt", "not-json".into()).await.unwrap();
    assert!(
        engine_server::StartupCoordinator::new(persistence.clone())
            .restore(&mut WorkflowEngine::new(), &mut HashMap::new())
            .await
            .is_err()
    );
    engine.shutdown().await;
}

#[tokio::test]
async fn jetstream_rejects_oversized_commit_without_partial_publication() {
    let (_server, persistence) = nats().await;
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let base = serve(
        engine.clone(),
        Some(persistence.clone()),
        ModelCheckMode::Off,
    )
    .await;
    let large = xml().replace(
        "</definitions>",
        &format!("<!--{}--></definitions>", "x".repeat(1_100_000)),
    );
    let (status, body) = deploy(&base, &large).await;
    assert_eq!(status, 500, "{body}");
    assert!(engine.list_definitions().await.is_empty());
    assert!(
        persistence
            .list_definition_collections()
            .await
            .unwrap()
            .is_empty()
    );
    engine.shutdown().await;
}

#[tokio::test]
async fn jetstream_restores_routes_pinned_to_the_original_pool_versions() {
    let (mut server, persistence) = nats().await;
    let engine = Arc::new(WorkflowEngine::new().with_persistence(persistence.clone()));
    let base = serve(
        engine.clone(),
        Some(persistence.clone()),
        ModelCheckMode::Off,
    )
    .await;
    let input = r#"<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL" id="d"><message id="m" name="order"/>
    <process id="sender" isExecutable="true"><startEvent id="ss"/><sendTask id="send" messageRef="m"/><endEvent id="se"/><sequenceFlow id="sf1" sourceRef="ss" targetRef="send"/><sequenceFlow id="sf2" sourceRef="send" targetRef="se"/></process>
    <process id="target" isExecutable="true"><startEvent id="receive"><messageEventDefinition messageRef="m"/></startEvent><userTask id="work"/><endEvent id="te"/><sequenceFlow id="tf1" sourceRef="receive" targetRef="work"/><sequenceFlow id="tf2" sourceRef="work" targetRef="te"/></process>
    <collaboration id="c"><participant id="ps" processRef="sender"/><participant id="pt" processRef="target"/><messageFlow id="flow" sourceRef="send" targetRef="receive"/></collaboration></definitions>"#;
    let (status, first) = deploy(&base, input).await;
    assert_eq!(status, 200, "{first}");
    let (status, second) = deploy(&base, input).await;
    assert_eq!(status, 200, "{second}");
    let source = first["processes"][0]["definition_key"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let target: uuid::Uuid = first["processes"][1]["definition_key"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_ne!(
        first["processes"][1]["definition_key"],
        second["processes"][1]["definition_key"]
    );
    let persistence = server.restart().await;
    let mut restored = WorkflowEngine::new();
    engine_server::StartupCoordinator::new(persistence)
        .restore(&mut restored, &mut HashMap::new())
        .await
        .unwrap();
    let plans = restored
        .plan_internal_messages(
            source,
            "send",
            "order-1",
            HashMap::from([("payload".into(), json!({"total":5}))]),
        )
        .await
        .unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].route.target.definition_key, target);
    assert_eq!(plans[0].route.target.version, 1);
    assert_eq!(plans[0].route.message_id, "m");
    assert_eq!(
        plans[0].receiver,
        engine_core::PlannedMessageReceiver::Start
    );
    assert!(restored.list_instances().await.is_empty());
    engine.shutdown().await;
}
