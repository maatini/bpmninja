//! E2E: Deploy must fail closed when `save_bpmn_xml` errors (R4).
//!
//! Persistence configured + XML-save failure → HTTP 5xx, definition rolled back.
//! Without persistence → in-memory deploy stays 200.

use engine_core::error::{EngineError, EngineResult};
use engine_core::{
    BucketEntry, BucketEntryDetail, CompletedInstanceQuery, HistoryEntry, HistoryQuery,
    PendingMessageCatch, PendingServiceTask, PendingTimer, PendingUserTask, ProcessDefinition,
    ProcessInstance, StorageInfo, Token, WorkflowPersistence,
};
use engine_server::AppBuildConfig;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

const MINIMAL_BPMN_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="PersistFailProcess">
    <startEvent id="Start_1" />
    <endEvent id="End_1" />
    <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="End_1" />
  </process>
</definitions>"#;

/// Persistence that fails only on `save_bpmn_xml`; everything else succeeds.
struct FailSaveXmlPersistence;

#[async_trait::async_trait]
impl WorkflowPersistence for FailSaveXmlPersistence {
    async fn save_token(&self, _: uuid::Uuid, _: &Token) -> EngineResult<()> {
        Ok(())
    }
    async fn load_tokens(&self, _: uuid::Uuid) -> EngineResult<Vec<Token>> {
        Ok(vec![])
    }
    async fn delete_token(&self, _: uuid::Uuid, _: uuid::Uuid) -> EngineResult<()> {
        Ok(())
    }
    async fn save_instance(&self, _: &ProcessInstance) -> EngineResult<()> {
        Ok(())
    }
    async fn list_instances(&self) -> EngineResult<Vec<ProcessInstance>> {
        Ok(vec![])
    }
    async fn delete_instance(&self, _: &str) -> EngineResult<()> {
        Ok(())
    }
    async fn save_definition(&self, _: &ProcessDefinition) -> EngineResult<()> {
        Ok(())
    }
    async fn list_definitions(&self) -> EngineResult<Vec<ProcessDefinition>> {
        Ok(vec![])
    }
    async fn delete_definition(&self, _: &str) -> EngineResult<()> {
        Ok(())
    }
    async fn save_user_task(&self, _: &PendingUserTask) -> EngineResult<()> {
        Ok(())
    }
    async fn delete_user_task(&self, _: uuid::Uuid) -> EngineResult<()> {
        Ok(())
    }
    async fn list_user_tasks(&self) -> EngineResult<Vec<PendingUserTask>> {
        Ok(vec![])
    }
    async fn save_service_task(&self, _: &PendingServiceTask) -> EngineResult<()> {
        Ok(())
    }
    async fn delete_service_task(&self, _: uuid::Uuid) -> EngineResult<()> {
        Ok(())
    }
    async fn list_service_tasks(&self) -> EngineResult<Vec<PendingServiceTask>> {
        Ok(vec![])
    }
    async fn save_timer(&self, _: &PendingTimer) -> EngineResult<()> {
        Ok(())
    }
    async fn delete_timer(&self, _: uuid::Uuid) -> EngineResult<()> {
        Ok(())
    }
    async fn list_timers(&self) -> EngineResult<Vec<PendingTimer>> {
        Ok(vec![])
    }
    async fn save_message_catch(&self, _: &PendingMessageCatch) -> EngineResult<()> {
        Ok(())
    }
    async fn delete_message_catch(&self, _: uuid::Uuid) -> EngineResult<()> {
        Ok(())
    }
    async fn list_message_catches(&self) -> EngineResult<Vec<PendingMessageCatch>> {
        Ok(vec![])
    }
    async fn save_file(&self, _: &str, _: &[u8]) -> EngineResult<()> {
        Ok(())
    }
    async fn load_file(&self, _: &str) -> EngineResult<Vec<u8>> {
        Ok(vec![])
    }
    async fn delete_file(&self, _: &str) -> EngineResult<()> {
        Ok(())
    }
    async fn save_bpmn_xml(&self, _: &str, _: &str) -> EngineResult<()> {
        Err(EngineError::PersistenceError(
            "injected xml save failure".into(),
        ))
    }
    async fn load_bpmn_xml(&self, _: &str) -> EngineResult<String> {
        Ok(String::new())
    }
    async fn list_bpmn_xml_ids(&self) -> EngineResult<Vec<String>> {
        Ok(vec![])
    }
    async fn get_storage_info(&self) -> EngineResult<Option<StorageInfo>> {
        Ok(None)
    }
    async fn append_history_entry(&self, _: &HistoryEntry) -> EngineResult<()> {
        Ok(())
    }
    async fn query_history(&self, _: HistoryQuery) -> EngineResult<Vec<HistoryEntry>> {
        Ok(vec![])
    }
    async fn save_completed_instance(&self, _: &ProcessInstance) -> EngineResult<()> {
        Ok(())
    }
    async fn query_completed_instances(
        &self,
        _: CompletedInstanceQuery,
    ) -> EngineResult<Vec<ProcessInstance>> {
        Ok(vec![])
    }
    async fn get_completed_instance(&self, _: &str) -> EngineResult<Option<ProcessInstance>> {
        Ok(None)
    }
    async fn get_bucket_entries(
        &self,
        _: &str,
        _: usize,
        _: usize,
    ) -> EngineResult<Vec<BucketEntry>> {
        Ok(vec![])
    }
    async fn get_bucket_entry_detail(&self, _: &str, _: &str) -> EngineResult<BucketEntryDetail> {
        Err(EngineError::PersistenceError("not implemented".into()))
    }
}

fn isolated_config() -> AppBuildConfig {
    AppBuildConfig {
        require_nats: Some(false),
        api_key: Some(None),
        cors_origins: Some(engine_server::default_cors_origins()),
        ..Default::default()
    }
}

async fn serve(app: axum::Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind failed");
    let addr = listener.local_addr().expect("addr failed");
    let base = format!("http://{addr}");
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    base
}

async fn start_without_persistence() -> String {
    serve(engine_server::build_app_with_options(isolated_config())).await
}

async fn start_with_failing_xml_persistence() -> String {
    let persistence: Arc<dyn WorkflowPersistence> = Arc::new(FailSaveXmlPersistence);
    let engine = engine_core::WorkflowEngine::new().with_persistence(persistence.clone());
    let app = engine_server::build_app_with_config(
        Arc::new(engine),
        Some(persistence),
        HashMap::new(),
        None,
        Arc::new(engine_server::LogBuffer::new()),
        isolated_config(),
    );
    serve(app).await
}

async fn deploy(base: &str) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{base}/api/deploy"))
        .json(&serde_json::json!({
            "xml": MINIMAL_BPMN_XML,
            "name": "PersistFailProcess"
        }))
        .send()
        .await
        .expect("deploy request failed")
}

async fn list_definitions(base: &str) -> Vec<Value> {
    let res = reqwest::get(format!("{base}/api/definitions"))
        .await
        .expect("list definitions failed");
    assert_eq!(res.status(), 200, "GET /api/definitions must stay 200");
    res.json::<Vec<Value>>()
        .await
        .expect("parse definitions list")
}

#[tokio::test]
async fn deploy_without_persistence_is_200_and_listed() {
    let base = start_without_persistence().await;
    let res = deploy(&base).await;
    assert_eq!(
        res.status(),
        200,
        "in-memory deploy without persistence must stay 200"
    );

    let body: Value = res.json().await.expect("parse deploy response");
    let def_key = body["definition_key"]
        .as_str()
        .expect("definition_key missing");

    let defs = list_definitions(&base).await;
    assert!(
        defs.iter().any(|d| d["key"].as_str() == Some(def_key)),
        "deployed definition must appear in GET /api/definitions: {defs:?}"
    );
}

#[tokio::test]
async fn deploy_save_bpmn_xml_failure_is_5xx_and_not_listed() {
    let base = start_with_failing_xml_persistence().await;
    let res = deploy(&base).await;
    let status = res.status();
    assert!(
        status.is_server_error(),
        "save_bpmn_xml failure must be HTTP 5xx, got {status}"
    );
    assert_eq!(
        status.as_u16(),
        500,
        "PersistenceError is mapped to 500 in AppError"
    );

    let body: Value = res.json().await.expect("parse error body");
    let error = body["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("Persistence error") || error.contains("xml save"),
        "error body should mention persistence failure, got {body}"
    );

    let defs = list_definitions(&base).await;
    assert!(
        defs.is_empty(),
        "failed deploy must roll back the in-memory definition, got {defs:?}"
    );
    assert!(
        !defs
            .iter()
            .any(|d| d["bpmn_id"].as_str() == Some("PersistFailProcess")),
        "PersistFailProcess must not remain after XML save failure"
    );
}
