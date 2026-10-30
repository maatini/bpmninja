//! Shared helpers for split unit tests (M5).

use super::super::*;
use crate::domain::ListenerEvent;
use crate::domain::ProcessDefinitionBuilder;
use serde_json::Value;
use std::collections::HashMap;

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

// ---------------------------------------------------------------------------
// Instrumented persistence (test double)
// ---------------------------------------------------------------------------

/// Ein `save_instance`, das im Backend angekommen ist und auf Freigabe wartet.
///
/// Der Test entscheidet, wann der Snapshot geschrieben wird, ohne mit
/// `sleep` zu synchronisieren.
pub(super) struct SaveArrival {
    pub(super) instance_id: Uuid,
    pub(super) variables: HashMap<String, Value>,
    release_tx: tokio::sync::oneshot::Sender<()>,
    done_rx: tokio::sync::oneshot::Receiver<()>,
}

impl SaveArrival {
    /// Gibt den blockierten Schreibvorgang frei und wartet, bis er committed ist.
    pub(super) async fn release_and_wait(self) {
        let _ = self.release_tx.send(());
        let _ = self.done_rx.await;
    }
}

/// Test-Persistenz: delegiert an `InMemoryPersistence` und kann `save_instance` anhalten.
///
/// Solange kein Gate gesetzt ist, werden Schreibvorgänge sofort committed und in
/// `committed` protokolliert. Mit [`InstrumentedPersistence::arm_save_gate`] meldet
/// jeder `save_instance`-Aufruf seine Ankunft und wartet auf [`SaveArrival::release_and_wait`].
pub(super) struct InstrumentedPersistence {
    pub(super) inner: crate::adapter::InMemoryPersistence,
    gate: tokio::sync::Mutex<Option<tokio::sync::mpsc::Sender<SaveArrival>>>,
    committed: tokio::sync::Mutex<Vec<HashMap<String, Value>>>,
}

impl InstrumentedPersistence {
    pub(super) fn new() -> Self {
        Self {
            inner: crate::adapter::InMemoryPersistence::new(),
            gate: tokio::sync::Mutex::new(None),
            committed: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub(super) async fn arm_save_gate(&self) -> tokio::sync::mpsc::Receiver<SaveArrival> {
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        *self.gate.lock().await = Some(tx);
        rx
    }

    pub(super) async fn committed_variables(&self) -> Vec<HashMap<String, Value>> {
        self.committed.lock().await.clone()
    }

    async fn commit_instance(&self, instance: &ProcessInstance) -> EngineResult<()> {
        self.inner.save_instance(instance).await?;
        self.committed.lock().await.push(instance.variables.clone());
        Ok(())
    }
}

#[async_trait::async_trait]
impl WorkflowPersistence for InstrumentedPersistence {
    async fn save_token(&self, instance_id: Uuid, token: &Token) -> EngineResult<()> {
        self.inner.save_token(instance_id, token).await
    }
    async fn load_tokens(&self, instance_id: Uuid) -> EngineResult<Vec<Token>> {
        self.inner.load_tokens(instance_id).await
    }
    async fn delete_token(&self, instance_id: Uuid, token_id: Uuid) -> EngineResult<()> {
        self.inner.delete_token(instance_id, token_id).await
    }
    async fn save_instance(&self, instance: &ProcessInstance) -> EngineResult<()> {
        let sender = self.gate.lock().await.clone();
        if let Some(sender) = sender {
            let (release_tx, release_rx) = tokio::sync::oneshot::channel();
            let (done_tx, done_rx) = tokio::sync::oneshot::channel();
            sender
                .send(SaveArrival {
                    instance_id: instance.id,
                    variables: instance.variables.clone(),
                    release_tx,
                    done_rx,
                })
                .await
                .map_err(|_| {
                    EngineError::PersistenceError(
                        "save gate closed before the snapshot was accepted".into(),
                    )
                })?;
            let _ = release_rx.await;
            self.commit_instance(instance).await?;
            let _ = done_tx.send(());
            return Ok(());
        }
        self.commit_instance(instance).await
    }
    async fn list_instances(&self) -> EngineResult<Vec<ProcessInstance>> {
        self.inner.list_instances().await
    }
    async fn delete_instance(&self, id: &str) -> EngineResult<()> {
        self.inner.delete_instance(id).await
    }
    async fn save_definition(&self, definition: &ProcessDefinition) -> EngineResult<()> {
        self.inner.save_definition(definition).await
    }
    async fn list_definitions(&self) -> EngineResult<Vec<ProcessDefinition>> {
        self.inner.list_definitions().await
    }
    async fn delete_definition(&self, key: &str) -> EngineResult<()> {
        self.inner.delete_definition(key).await
    }
    async fn save_definition_collection(
        &self,
        collection: &crate::DefinitionCollection,
    ) -> EngineResult<()> {
        self.inner.save_definition_collection(collection).await
    }
    async fn list_definition_collections(&self) -> EngineResult<Vec<crate::DefinitionCollection>> {
        self.inner.list_definition_collections().await
    }
    async fn save_user_task(&self, task: &PendingUserTask) -> EngineResult<()> {
        self.inner.save_user_task(task).await
    }
    async fn delete_user_task(&self, task_id: Uuid) -> EngineResult<()> {
        self.inner.delete_user_task(task_id).await
    }
    async fn list_user_tasks(&self) -> EngineResult<Vec<PendingUserTask>> {
        self.inner.list_user_tasks().await
    }
    async fn save_service_task(&self, task: &PendingServiceTask) -> EngineResult<()> {
        self.inner.save_service_task(task).await
    }
    async fn delete_service_task(&self, task_id: Uuid) -> EngineResult<()> {
        self.inner.delete_service_task(task_id).await
    }
    async fn list_service_tasks(&self) -> EngineResult<Vec<PendingServiceTask>> {
        self.inner.list_service_tasks().await
    }
    async fn save_timer(&self, timer: &PendingTimer) -> EngineResult<()> {
        self.inner.save_timer(timer).await
    }
    async fn delete_timer(&self, timer_id: Uuid) -> EngineResult<()> {
        self.inner.delete_timer(timer_id).await
    }
    async fn list_timers(&self) -> EngineResult<Vec<PendingTimer>> {
        self.inner.list_timers().await
    }
    async fn save_message_catch(&self, catch: &PendingMessageCatch) -> EngineResult<()> {
        self.inner.save_message_catch(catch).await
    }
    async fn delete_message_catch(&self, catch_id: Uuid) -> EngineResult<()> {
        self.inner.delete_message_catch(catch_id).await
    }
    async fn list_message_catches(&self) -> EngineResult<Vec<PendingMessageCatch>> {
        self.inner.list_message_catches().await
    }
    async fn save_signal_catch(&self, catch: &PendingSignalCatch) -> EngineResult<()> {
        self.inner.save_signal_catch(catch).await
    }
    async fn delete_signal_catch(&self, catch_id: Uuid) -> EngineResult<()> {
        self.inner.delete_signal_catch(catch_id).await
    }
    async fn list_signal_catches(&self) -> EngineResult<Vec<PendingSignalCatch>> {
        self.inner.list_signal_catches().await
    }
    async fn save_conditional_catch(&self, catch: &PendingConditionalCatch) -> EngineResult<()> {
        self.inner.save_conditional_catch(catch).await
    }
    async fn delete_conditional_catch(&self, catch_id: Uuid) -> EngineResult<()> {
        self.inner.delete_conditional_catch(catch_id).await
    }
    async fn list_conditional_catches(&self) -> EngineResult<Vec<PendingConditionalCatch>> {
        self.inner.list_conditional_catches().await
    }
    async fn save_file(&self, object_key: &str, data: &[u8]) -> EngineResult<()> {
        self.inner.save_file(object_key, data).await
    }
    async fn load_file(&self, object_key: &str) -> EngineResult<Vec<u8>> {
        self.inner.load_file(object_key).await
    }
    async fn delete_file(&self, object_key: &str) -> EngineResult<()> {
        self.inner.delete_file(object_key).await
    }
    async fn save_bpmn_xml(&self, definition_key: &str, xml: &str) -> EngineResult<()> {
        self.inner.save_bpmn_xml(definition_key, xml).await
    }
    async fn load_bpmn_xml(&self, definition_key: &str) -> EngineResult<String> {
        self.inner.load_bpmn_xml(definition_key).await
    }
    async fn list_bpmn_xml_ids(&self) -> EngineResult<Vec<String>> {
        self.inner.list_bpmn_xml_ids().await
    }
    async fn get_storage_info(&self) -> EngineResult<Option<crate::persistence::StorageInfo>> {
        self.inner.get_storage_info().await
    }
    async fn append_history_entry(&self, entry: &crate::history::HistoryEntry) -> EngineResult<()> {
        self.inner.append_history_entry(entry).await
    }
    async fn query_history(
        &self,
        query: crate::persistence::HistoryQuery,
    ) -> EngineResult<Vec<crate::history::HistoryEntry>> {
        self.inner.query_history(query).await
    }
    async fn save_completed_instance(&self, instance: &ProcessInstance) -> EngineResult<()> {
        self.inner.save_completed_instance(instance).await
    }
    async fn query_completed_instances(
        &self,
        query: crate::persistence::CompletedInstanceQuery,
    ) -> EngineResult<Vec<ProcessInstance>> {
        self.inner.query_completed_instances(query).await
    }
    async fn get_completed_instance(&self, id: &str) -> EngineResult<Option<ProcessInstance>> {
        self.inner.get_completed_instance(id).await
    }
    async fn get_bucket_entries(
        &self,
        bucket_name: &str,
        offset: usize,
        limit: usize,
    ) -> EngineResult<Vec<crate::persistence::BucketEntry>> {
        self.inner
            .get_bucket_entries(bucket_name, offset, limit)
            .await
    }
    async fn get_bucket_entry_detail(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> EngineResult<crate::persistence::BucketEntryDetail> {
        self.inner.get_bucket_entry_detail(bucket_name, key).await
    }
}
