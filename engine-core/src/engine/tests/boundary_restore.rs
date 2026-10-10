//! Boundary timers attached to a waiting user task must be persisted and restored.

use super::super::*;
use crate::adapter::InMemoryPersistence;
use crate::domain::ProcessDefinitionBuilder;

fn user_task_with_boundary_timer(key: Option<Uuid>) -> ProcessDefinition {
    let mut builder = ProcessDefinitionBuilder::new("bound_restore");
    if let Some(key) = key {
        builder = builder.with_key(key);
    }
    builder
        .node("start", BpmnElement::StartEvent)
        .node("task", BpmnElement::UserTask("assignee".into()))
        .node(
            "bound_timer",
            BpmnElement::BoundaryTimerEvent {
                attached_to: "task".into(),
                timer: crate::domain::TimerDefinition::Duration(Duration::from_secs(60)),
                cancel_activity: true,
            },
        )
        .node("end", BpmnElement::EndEvent)
        .node("timeout_end", BpmnElement::EndEvent)
        .flow("start", "task")
        .flow("task", "end")
        .flow("bound_timer", "timeout_end")
        .build()
        .unwrap()
}

#[tokio::test]
async fn boundary_timer_persists_and_restores() {
    let pers = Arc::new(InMemoryPersistence::new());
    let engine = WorkflowEngine::new().with_persistence(pers.clone());

    let (def_key, _) = engine
        .deploy_definition(user_task_with_boundary_timer(None))
        .await;
    engine.start_instance(def_key).await.unwrap();

    assert!(
        !engine.pending_user_tasks.is_empty(),
        "user task should be pending"
    );
    let timers = pers.list_timers().await.unwrap();
    assert!(!timers.is_empty(), "boundary timer must be persisted");

    let engine2 = WorkflowEngine::new().with_persistence(pers.clone());
    engine2
        .deploy_definition(user_task_with_boundary_timer(Some(def_key)))
        .await;

    for instance in pers.list_instances().await.unwrap() {
        engine2.restore_instance(instance).await;
    }
    for timer in pers.list_timers().await.unwrap() {
        engine2.restore_timer(timer);
    }
    for task in pers.list_user_tasks().await.unwrap() {
        engine2.restore_user_task(task);
    }

    assert!(!engine2.pending_timers.is_empty());
}
