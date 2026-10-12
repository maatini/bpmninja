//! P3: `persist_instance` must not hold the instance lock across the network save,
//! and concurrent saves of one instance must land in order.

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::{EngineResult, ProcessDefinitionBuilder, Token};
use crate::history::HistoryEntry;
use crate::port::{
    BucketEntry, BucketEntryDetail, CompletedInstanceQuery, HistoryQuery, StorageInfo,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

/// Delegates to `InMemoryPersistence`; `save_instance` sleeps while `slow` is set.
/// The first slow save sleeps `first_delay`, later ones `later_delay`.
struct SlowSavePersistence {
    inner: InMemoryPersistence,
    slow: AtomicBool,
    slow_saves: AtomicUsize,
    first_delay: Duration,
    later_delay: Duration,
}

impl SlowSavePersistence {
    fn new(first_delay: Duration, later_delay: Duration) -> Self {
        Self {
            inner: InMemoryPersistence::new(),
            slow: AtomicBool::new(false),
            slow_saves: AtomicUsize::new(0),
            first_delay,
            later_delay,
        }
    }
}

#[async_trait::async_trait]
impl WorkflowPersistence for SlowSavePersistence {
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

/// Engine with slow-save persistence and one instance parked at a user task.
async fn setup(
    first_delay: Duration,
    later_delay: Duration,
) -> (Arc<WorkflowEngine>, Arc<SlowSavePersistence>, Uuid) {
    let persistence = Arc::new(SlowSavePersistence::new(first_delay, later_delay));
    let engine = Arc::new(
        WorkflowEngine::new().with_persistence(persistence.clone() as Arc<dyn WorkflowPersistence>),
    );
    let def = ProcessDefinitionBuilder::new("persist_lock")
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("tester".into()))
        .node("end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .build()
        .unwrap();
    let (key, _) = engine.deploy_definition(def).await;
    let id = engine.start_instance(key).await.unwrap();
    persistence.slow.store(true, Ordering::SeqCst);
    (engine, persistence, id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn instance_stays_readable_while_save_is_in_flight() {
    let (engine, _p, id) = setup(Duration::from_millis(800), Duration::from_millis(800)).await;

    let writer = {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine
                .update_instance_variables(id, HashMap::from([("x".to_string(), json!(1))]))
                .await
        })
    };
    // Let the update reach the slow save.
    tokio::time::sleep(Duration::from_millis(150)).await;

    let read = tokio::time::timeout(Duration::from_millis(300), engine.get_instance_details(id))
        .await
        .expect("instance read blocked by an in-flight save");
    assert_eq!(read.unwrap().variables.get("x"), Some(&json!(1)));

    writer.await.unwrap().unwrap();
    engine.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_saves_persist_the_latest_state() {
    // First save is slow, the second fast: without ordering the older
    // snapshot would be written last.
    let (engine, persistence, id) =
        setup(Duration::from_millis(400), Duration::from_millis(5)).await;

    let first = {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine
                .update_instance_variables(id, HashMap::from([("x".to_string(), json!(1))]))
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    engine
        .update_instance_variables(id, HashMap::from([("x".to_string(), json!(2))]))
        .await
        .unwrap();
    first.await.unwrap().unwrap();

    let live = engine.get_instance_details(id).await.unwrap();
    let persisted = persistence
        .inner
        .list_instances()
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.id == id)
        .expect("instance was persisted");
    assert_eq!(live.variables.get("x"), Some(&json!(2)));
    assert_eq!(persisted.variables.get("x"), live.variables.get("x"));
    engine.shutdown().await;
}
