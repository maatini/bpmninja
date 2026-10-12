//! Shared helpers for split unit tests (M5).

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::ListenerEvent;
use crate::domain::ProcessDefinitionBuilder;
use crate::domain::{EngineResult, Token};
use crate::history::HistoryEntry;
use crate::port::{
    BucketEntry, BucketEntryDetail, CompletedInstanceQuery, HistoryQuery, StorageInfo,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

pub(super) async fn complete_all_service_tasks(
    engine: &WorkflowEngine,
    worker: &str,
    vars: HashMap<String, Value>,
) {
    let mut to_complete = Vec::new();
    for task in engine
        .pending_service_tasks
        .iter()
        .map(|r| r.value().clone())
    {
        to_complete.push((task.id, task.topic.clone()));
    }
    for (id, topic) in to_complete {
        let _ = engine
            .fetch_and_lock_service_tasks(worker, 10, std::slice::from_ref(&topic), 60000)
            .await;
        engine
            .complete_service_task(id, worker, vars.clone())
            .await
            .unwrap();
    }
}

pub(super) async fn setup_linear_engine() -> (WorkflowEngine, Uuid) {
    let engine = WorkflowEngine::new();

    // Register a simple service handler

    let def = ProcessDefinitionBuilder::new("linear")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "validate".into(),
                multi_instance: None,
            },
        )
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();

    let (def_key, _) = engine.deploy_definition(def).await;
    (engine, def_key)
}

// -----------------------------------------------------------------------
// E2E: XOR Gateway with 2 UserTasks (condition: x > 0 / default)
// -----------------------------------------------------------------------

/// Helper: builds a workflow with XOR gateway routing to two user tasks.
///
/// ```text
/// Start → XOR Gateway → (x > 0) → user-task-1 ("author")   → End
///                     → (default) → user-task-2 ("reviewer") → End
/// ```
pub(super) fn build_xor_user_task_definition() -> ProcessDefinition {
    ProcessDefinitionBuilder::new("xor_user_tasks")
        .node("start", BpmnElement::StartEvent)
        .node(
            "gw",
            BpmnElement::ExclusiveGateway {
                default: Some("user-task-2".into()),
            },
        )
        .node("user-task-1", BpmnElement::UserTask("author".into()))
        .node("user-task-2", BpmnElement::UserTask("reviewer".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "gw")
        .conditional_flow("gw", "user-task-1", "x > 0")
        .flow("gw", "user-task-2")
        .flow("user-task-1", "end")
        .flow("user-task-2", "end")
        .build()
        .unwrap()
}

pub(super) fn build_script_test_definition() -> ProcessDefinition {
    ProcessDefinitionBuilder::new("script_test")
        .node("start", BpmnElement::StartEvent)
        .node(
            "svc",
            BpmnElement::ServiceTask {
                topic: "calculate".into(),
                multi_instance: None,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .flow("start", "svc")
        .flow("svc", "end")
        .listener(
            "svc",
            ListenerEvent::Start,
            "x = x * 2; let result = \"small\"; if x > 10 { result = \"big\" }",
        )
        .build()
        .unwrap()
}

pub(super) fn build_child_error_process(code: &str) -> ProcessDefinition {
    ProcessDefinitionBuilder::new("child_proc")
        .node("start", BpmnElement::StartEvent)
        .node(
            "err_end",
            BpmnElement::ErrorEndEvent {
                error_code: String::from(code),
            },
        )
        .flow("start", "err_end")
        .build()
        .unwrap()
}

// ---------------------------------------------------------------------------
// migrate_instance tests
// ---------------------------------------------------------------------------

/// Hilfsfunktion: deployt zwei Versionen eines Prozesses mit gleicher bpmn_id.
/// v1 hat Nodes start→ut→end, v2 hat dieselben IDs (kein Mapping nötig).
pub(super) async fn setup_migration_engine_same_ids() -> (WorkflowEngine, Uuid, Uuid) {
    let engine = WorkflowEngine::new();

    let def_v1 = ProcessDefinitionBuilder::new("migratable")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();

    let def_v2 = ProcessDefinitionBuilder::new("migratable")
        .node("start", BpmnElement::StartEvent)
        .node("ut", BpmnElement::UserTask("alice".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "ut")
        .flow("ut", "end")
        .build()
        .unwrap();

    let (key_v1, _) = engine.deploy_definition(def_v1).await;
    let (key_v2, _) = engine.deploy_definition(def_v2).await;

    (engine, key_v1, key_v2)
}

/// Test persistence: delegates to `InMemoryPersistence` and adds probes.
///
/// - `save_instance` sleeps while `slow` is set (first slow save `first_delay`,
///   later ones `later_delay`).
/// - `append_history_entries` calls are counted in `history_batches`.
pub(super) struct InstrumentedPersistence {
    pub(super) inner: InMemoryPersistence,
    pub(super) slow: AtomicBool,
    slow_saves: AtomicUsize,
    first_delay: Duration,
    later_delay: Duration,
    pub(super) history_batches: AtomicUsize,
    pub(super) serialized_payloads: std::sync::Mutex<Vec<Vec<u8>>>,
    pub(super) save_failures: AtomicUsize,
}

impl InstrumentedPersistence {
    pub(super) fn new(first_delay: Duration, later_delay: Duration) -> Self {
        Self {
            inner: InMemoryPersistence::new(),
            slow: AtomicBool::new(false),
            slow_saves: AtomicUsize::new(0),
            first_delay,
            later_delay,
            history_batches: AtomicUsize::new(0),
            serialized_payloads: std::sync::Mutex::new(Vec::new()),
            save_failures: AtomicUsize::new(0),
        }
    }
}

#[async_trait::async_trait]
impl WorkflowPersistence for InstrumentedPersistence {
    async fn save_token(&self, i: Uuid, t: &Token) -> EngineResult<()> {
        self.inner.save_token(i, t).await
    }
    async fn load_tokens(&self, i: Uuid) -> EngineResult<Vec<Token>> {
        self.inner.load_tokens(i).await
    }
    async fn delete_token(&self, i: Uuid, t: Uuid) -> EngineResult<()> {
        self.inner.delete_token(i, t).await
    }
    async fn save_instance(&self, instance: &ProcessInstance) -> EngineResult<()> {
        if self.slow.load(Ordering::SeqCst) {
            let n = self.slow_saves.fetch_add(1, Ordering::SeqCst);
            let delay = if n == 0 {
                self.first_delay
            } else {
                self.later_delay
            };
            tokio::time::sleep(delay).await;
        }
        self.inner.save_instance(instance).await
    }
    async fn save_instance_json(
        &self,
        instance: &ProcessInstance,
        json: &[u8],
    ) -> EngineResult<()> {
        self.serialized_payloads.lock().unwrap().push(json.to_vec());
        if self
            .save_failures
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
        {
            return Err(crate::domain::EngineError::PersistenceError(
                "injected save failure".into(),
            ));
        }
        self.save_instance(instance).await
    }
    async fn list_instances(&self) -> EngineResult<Vec<ProcessInstance>> {
        self.inner.list_instances().await
    }
    async fn delete_instance(&self, id: &str) -> EngineResult<()> {
        self.inner.delete_instance(id).await
    }
    async fn save_definition(&self, d: &ProcessDefinition) -> EngineResult<()> {
        self.inner.save_definition(d).await
    }
    async fn list_definitions(&self) -> EngineResult<Vec<ProcessDefinition>> {
        self.inner.list_definitions().await
    }
    async fn delete_definition(&self, k: &str) -> EngineResult<()> {
        self.inner.delete_definition(k).await
    }
    async fn save_user_task(&self, t: &PendingUserTask) -> EngineResult<()> {
        self.inner.save_user_task(t).await
    }
    async fn delete_user_task(&self, id: Uuid) -> EngineResult<()> {
        self.inner.delete_user_task(id).await
    }
    async fn list_user_tasks(&self) -> EngineResult<Vec<PendingUserTask>> {
        self.inner.list_user_tasks().await
    }
    async fn save_service_task(&self, t: &PendingServiceTask) -> EngineResult<()> {
        self.inner.save_service_task(t).await
    }
    async fn delete_service_task(&self, id: Uuid) -> EngineResult<()> {
        self.inner.delete_service_task(id).await
    }
    async fn list_service_tasks(&self) -> EngineResult<Vec<PendingServiceTask>> {
        self.inner.list_service_tasks().await
    }
    async fn save_timer(&self, t: &PendingTimer) -> EngineResult<()> {
        self.inner.save_timer(t).await
    }
    async fn delete_timer(&self, id: Uuid) -> EngineResult<()> {
        self.inner.delete_timer(id).await
    }
    async fn list_timers(&self) -> EngineResult<Vec<PendingTimer>> {
        self.inner.list_timers().await
    }
    async fn save_message_catch(&self, c: &PendingMessageCatch) -> EngineResult<()> {
        self.inner.save_message_catch(c).await
    }
    async fn delete_message_catch(&self, id: Uuid) -> EngineResult<()> {
        self.inner.delete_message_catch(id).await
    }
    async fn list_message_catches(&self) -> EngineResult<Vec<PendingMessageCatch>> {
        self.inner.list_message_catches().await
    }
    async fn save_file(&self, k: &str, d: &[u8]) -> EngineResult<()> {
        self.inner.save_file(k, d).await
    }
    async fn load_file(&self, k: &str) -> EngineResult<Vec<u8>> {
        self.inner.load_file(k).await
    }
    async fn delete_file(&self, k: &str) -> EngineResult<()> {
        self.inner.delete_file(k).await
    }
    async fn save_bpmn_xml(&self, k: &str, x: &str) -> EngineResult<()> {
        self.inner.save_bpmn_xml(k, x).await
    }
    async fn load_bpmn_xml(&self, k: &str) -> EngineResult<String> {
        self.inner.load_bpmn_xml(k).await
    }
    async fn list_bpmn_xml_ids(&self) -> EngineResult<Vec<String>> {
        self.inner.list_bpmn_xml_ids().await
    }
    async fn get_storage_info(&self) -> EngineResult<Option<StorageInfo>> {
        self.inner.get_storage_info().await
    }
    async fn append_history_entry(&self, e: &HistoryEntry) -> EngineResult<()> {
        self.inner.append_history_entry(e).await
    }
    async fn append_history_entries(&self, entries: &[HistoryEntry]) -> EngineResult<()> {
        self.history_batches.fetch_add(1, Ordering::SeqCst);
        self.inner.append_history_entries(entries).await
    }
    async fn query_history(&self, q: HistoryQuery) -> EngineResult<Vec<HistoryEntry>> {
        self.inner.query_history(q).await
    }
    async fn save_completed_instance(&self, i: &ProcessInstance) -> EngineResult<()> {
        self.inner.save_completed_instance(i).await
    }
    async fn query_completed_instances(
        &self,
        q: CompletedInstanceQuery,
    ) -> EngineResult<Vec<ProcessInstance>> {
        self.inner.query_completed_instances(q).await
    }
    async fn get_completed_instance(&self, id: &str) -> EngineResult<Option<ProcessInstance>> {
        self.inner.get_completed_instance(id).await
    }
    async fn get_bucket_entries(
        &self,
        b: &str,
        o: usize,
        l: usize,
    ) -> EngineResult<Vec<BucketEntry>> {
        self.inner.get_bucket_entries(b, o, l).await
    }
    async fn get_bucket_entry_detail(&self, b: &str, k: &str) -> EngineResult<BucketEntryDetail> {
        self.inner.get_bucket_entry_detail(b, k).await
    }
}
