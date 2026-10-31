use std::collections::HashMap;
use std::sync::Arc;

use engine_core::EngineResult;
use engine_core::ProcessDefinition;
use engine_core::WorkflowEngine;
use engine_core::WorkflowPersistence;
use uuid::Uuid;

pub struct RestoreStats {
    pub definitions: usize,
    pub instances: usize,
    pub user_tasks: usize,
    pub service_tasks: usize,
    pub timers: usize,
    pub message_catches: usize,
    pub signal_catches: usize,
    pub conditional_catches: usize,
}

pub struct StartupCoordinator {
    persistence: Arc<dyn WorkflowPersistence>,
}

impl StartupCoordinator {
    pub fn new(persistence: Arc<dyn WorkflowPersistence>) -> Self {
        Self { persistence }
    }

    pub async fn restore(
        &self,
        engine: &mut WorkflowEngine,
        deployed_xml: &mut HashMap<String, String>,
    ) -> EngineResult<RestoreStats> {
        // Ein korrupter Collection-Datensatz bricht den Start ab. Ein Fallback
        // auf den ersten Prozess würde den Batch unvollständig wiederherstellen.
        let collections = self.persistence.list_definition_collections().await?;
        let mut definitions = self.restore_definitions(engine, deployed_xml).await;
        for collection in collections {
            engine.restore_definition_collection(&collection)?;
            for definition in &collection.definitions {
                if !collection.deleted.contains(&definition.key) {
                    deployed_xml.insert(definition.key.to_string(), collection.xml.clone());
                    definitions += 1;
                }
            }
        }
        let instances = self.restore_instances(engine).await;
        let user_tasks = self.restore_user_tasks(engine).await;
        let service_tasks = self.restore_service_tasks(engine).await;
        let timers = self.restore_timers(engine).await;
        let message_catches = self.restore_message_catches(engine).await;
        let signal_catches = self.restore_signal_catches(engine).await;
        let conditional_catches = self.restore_conditional_catches(engine).await;

        Ok(RestoreStats {
            definitions,
            instances,
            user_tasks,
            service_tasks,
            timers,
            message_catches,
            signal_catches,
            conditional_catches,
        })
    }

    async fn restore_definitions(
        &self,
        engine: &mut WorkflowEngine,
        deployed_xml: &mut HashMap<String, String>,
    ) -> usize {
        match self.persistence.list_definitions().await {
            Ok(defs) if !defs.is_empty() => {
                self.restore_definitions_from_json(engine, deployed_xml, defs)
                    .await
            }
            Ok(_) => {
                tracing::info!("Definitions-KV leer — Fallback auf XML-Parse für Restore.");
                self.restore_definitions_from_xml(engine, deployed_xml)
                    .await
            }
            Err(e) => {
                tracing::error!(
                    "Definitionen aus Definitions-KV laden fehlgeschlagen: {:?}. Versuche XML-Fallback.",
                    e
                );
                self.restore_definitions_from_xml(engine, deployed_xml)
                    .await
            }
        }
    }

    async fn restore_definitions_from_json(
        &self,
        engine: &mut WorkflowEngine,
        deployed_xml: &mut HashMap<String, String>,
        mut defs: Vec<ProcessDefinition>,
    ) -> usize {
        defs.sort_by(|a, b| a.id.cmp(&b.id).then(a.version.cmp(&b.version)));
        let mut count = 0;
        for def in defs {
            let xml_key = def.key.to_string();
            let key = def.key;
            engine.restore_definitions(vec![def]);
            match self.persistence.load_bpmn_xml(&xml_key).await {
                Ok(xml) => {
                    deployed_xml.insert(key.to_string(), xml);
                }
                Err(e) => {
                    tracing::warn!(
                        "BPMN-XML für Definition '{}' fehlt oder nicht ladbar: {:?} — Definition trotzdem restored.",
                        key,
                        e
                    );
                }
            }
            tracing::info!("Definition wiederhergestellt (key: {}, JSON)", key);
            count += 1;
        }
        tracing::info!("Restore abgeschlossen: {count} Definition(en) aus Definitions-KV.");
        count
    }

    async fn restore_definitions_from_xml(
        &self,
        engine: &mut WorkflowEngine,
        deployed_xml: &mut HashMap<String, String>,
    ) -> usize {
        let ids = match self.persistence.list_bpmn_xml_ids().await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::error!("Definitionen aus NATS-XML laden fehlgeschlagen: {:?}", e);
                return 0;
            }
        };
        let mut count = 0;
        for nats_key in &ids {
            match self.persistence.load_bpmn_xml(nats_key).await {
                Ok(xml) => match bpmn_parser::parse_bpmn_xml(&xml) {
                    Ok(mut def) => {
                        if let Ok(old_uuid) = Uuid::parse_str(nats_key) {
                            def.key = old_uuid;
                        }
                        let (key, _) = engine.deploy_definition(def).await;
                        deployed_xml.insert(key.to_string(), xml);
                        tracing::info!("Definition wiederhergestellt (key: {}, XML-Fallback)", key);
                        count += 1;
                    }
                    Err(e) => tracing::error!("BPMN '{}' parsen fehlgeschlagen: {:?}", nats_key, e),
                },
                Err(e) => tracing::error!("XML für '{}' laden fehlgeschlagen: {:?}", nats_key, e),
            }
        }
        tracing::info!("Restore abgeschlossen: {count} Definition(en) via XML-Fallback.");
        count
    }

    async fn restore_instances(&self, engine: &mut WorkflowEngine) -> usize {
        match self.persistence.list_instances().await {
            Ok(instances) => {
                let num = instances.len();
                for inst in instances {
                    engine.restore_instance(inst).await;
                }
                tracing::info!("{num} Prozessinstanz(en) wiederhergestellt.");
                num
            }
            Err(e) => {
                tracing::error!("Instanzen aus NATS laden fehlgeschlagen: {:?}", e);
                0
            }
        }
    }

    async fn restore_user_tasks(&self, engine: &mut WorkflowEngine) -> usize {
        match self.persistence.list_user_tasks().await {
            Ok(tasks) => {
                let num = tasks.len();
                for task in tasks {
                    engine.restore_user_task(task);
                }
                tracing::info!("{num} User-Task(s) wiederhergestellt.");
                num
            }
            Err(e) => {
                tracing::error!("User-Tasks aus NATS laden fehlgeschlagen: {:?}", e);
                0
            }
        }
    }

    async fn restore_service_tasks(&self, engine: &mut WorkflowEngine) -> usize {
        match self.persistence.list_service_tasks().await {
            Ok(tasks) => {
                let num = tasks.len();
                for task in tasks {
                    engine.restore_service_task(task);
                }
                tracing::info!("{num} Service-Task(s) wiederhergestellt.");
                num
            }
            Err(e) => {
                tracing::error!("Service-Tasks aus NATS laden fehlgeschlagen: {:?}", e);
                0
            }
        }
    }

    async fn restore_timers(&self, engine: &mut WorkflowEngine) -> usize {
        match self.persistence.list_timers().await {
            Ok(timers) => {
                let num = timers.len();
                for timer in timers {
                    engine.restore_timer(timer);
                }
                tracing::info!("{num} Timer wiederhergestellt.");
                num
            }
            Err(e) => {
                tracing::error!("Timer aus NATS laden fehlgeschlagen: {:?}", e);
                0
            }
        }
    }

    async fn restore_message_catches(&self, engine: &mut WorkflowEngine) -> usize {
        match self.persistence.list_message_catches().await {
            Ok(catches) => {
                let num = catches.len();
                for catch in catches {
                    engine.restore_message_catch(catch);
                }
                tracing::info!("{num} Message-Catch(es) wiederhergestellt.");
                num
            }
            Err(e) => {
                tracing::error!("Message-Catches aus NATS laden fehlgeschlagen: {:?}", e);
                0
            }
        }
    }

    async fn restore_signal_catches(&self, engine: &mut WorkflowEngine) -> usize {
        match self.persistence.list_signal_catches().await {
            Ok(catches) => {
                let num = catches.len();
                for catch in catches {
                    engine.restore_signal_catch(catch);
                }
                tracing::info!("{num} Signal-Catch(es) wiederhergestellt.");
                num
            }
            Err(e) => {
                tracing::error!("Signal-Catches aus NATS laden fehlgeschlagen: {:?}", e);
                0
            }
        }
    }

    async fn restore_conditional_catches(&self, engine: &mut WorkflowEngine) -> usize {
        match self.persistence.list_conditional_catches().await {
            Ok(catches) => {
                let num = catches.len();
                for catch in catches {
                    engine.restore_conditional_catch(catch);
                }
                tracing::info!("{num} Conditional-Catch(es) wiederhergestellt.");
                num
            }
            Err(e) => {
                tracing::error!("Conditional-Catches aus NATS laden fehlgeschlagen: {:?}", e);
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use engine_core::adapter::InMemoryPersistence;
    use engine_core::domain::{BpmnElement, ProcessDefinitionBuilder};
    use engine_core::runtime::{
        InstanceState, PendingConditionalCatch, PendingMessageCatch, PendingServiceTask,
        PendingSignalCatch, PendingTimer, PendingUserTask, ProcessInstance,
    };
    use std::collections::HashMap;

    const MINIMAL_BPMN_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions id="Definitions_1" xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="StartupProcess">
    <startEvent id="Start_1" />
    <endEvent id="End_1" />
    <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="End_1" />
  </process>
</definitions>"#;

    fn definition() -> ProcessDefinition {
        ProcessDefinitionBuilder::new("startup_proc")
            .node("start", BpmnElement::StartEvent)
            .node("end", BpmnElement::EndEvent)
            .flow("start", "end")
            .build()
            .expect("valid definition")
    }

    fn instance(definition_key: Uuid) -> ProcessInstance {
        ProcessInstance {
            id: Uuid::new_v4(),
            definition_key,
            business_key: "startup-restore".into(),
            parent_instance_id: None,
            state: InstanceState::Running,
            current_node: "Wait_1".into(),
            audit_log: vec!["started".into()],
            variables: HashMap::new(),
            tokens: HashMap::new(),
            active_tokens: vec![],
            join_barriers: HashMap::new(),
            multi_instance_state: HashMap::new(),
            compensation_log: vec![],
            outstanding_calls: HashMap::new(),
            incidents: Vec::new(),
            started_at: Some(chrono::Utc::now()),
            completed_at: None,
        }
    }

    #[tokio::test]
    async fn restore_loads_all_persisted_state() {
        let persistence: Arc<dyn WorkflowPersistence> = Arc::new(InMemoryPersistence::new());
        let definition = definition();
        let definition_key = definition.key;
        let key = definition_key.to_string();
        persistence.save_definition(&definition).await.unwrap();
        persistence
            .save_bpmn_xml(&key, MINIMAL_BPMN_XML)
            .await
            .unwrap();

        let instance = instance(definition_key);
        persistence.save_instance(&instance).await.unwrap();

        let user_task = PendingUserTask {
            task_id: Uuid::new_v4(),
            instance_id: instance.id,
            node_id: "User_1".into(),
            assignee: "alice".into(),
            token_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            business_key: Some("startup-restore".into()),
        };
        let service_task = PendingServiceTask {
            id: Uuid::new_v4(),
            instance_id: instance.id,
            definition_key,
            node_id: "Service_1".into(),
            topic: "startup".into(),
            token_id: Uuid::new_v4(),
            business_key: None,
            variables_snapshot: HashMap::new(),
            created_at: chrono::Utc::now(),
            worker_id: None,
            lock_expiration: None,
            retries: 3,
            error_message: None,
            error_details: None,
        };
        let timer = PendingTimer {
            id: Uuid::new_v4(),
            instance_id: instance.id,
            node_id: "Timer_1".into(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            token_id: Uuid::new_v4(),
            timer_def: None,
            remaining_repetitions: None,
        };
        let message = PendingMessageCatch {
            id: Uuid::new_v4(),
            instance_id: instance.id,
            node_id: "Message_1".into(),
            message_name: "OrderPaid".into(),
            token_id: Uuid::new_v4(),
        };
        let signal = PendingSignalCatch {
            id: Uuid::new_v4(),
            instance_id: instance.id,
            node_id: "Signal_1".into(),
            signal_name: "OrderReady".into(),
            token_id: Uuid::new_v4(),
        };
        let conditional = PendingConditionalCatch {
            id: Uuid::new_v4(),
            instance_id: instance.id,
            node_id: "Condition_1".into(),
            condition: "amount > 100".into(),
            token_id: Uuid::new_v4(),
            last_result: true,
        };

        persistence.save_user_task(&user_task).await.unwrap();
        persistence.save_service_task(&service_task).await.unwrap();
        persistence.save_timer(&timer).await.unwrap();
        persistence.save_message_catch(&message).await.unwrap();
        persistence.save_signal_catch(&signal).await.unwrap();
        persistence
            .save_conditional_catch(&conditional)
            .await
            .unwrap();

        let mut engine = WorkflowEngine::new();
        let mut deployed_xml = HashMap::new();
        let stats = StartupCoordinator::new(persistence)
            .restore(&mut engine, &mut deployed_xml)
            .await
            .expect("restore");

        assert_eq!(stats.definitions, 1);
        assert_eq!(stats.instances, 1);
        assert_eq!(stats.user_tasks, 1);
        assert_eq!(stats.service_tasks, 1);
        assert_eq!(stats.timers, 1);
        assert_eq!(stats.message_catches, 1);
        assert_eq!(stats.signal_catches, 1);
        assert_eq!(stats.conditional_catches, 1);

        assert!(engine.get_definition(&definition_key).await.is_some());
        assert_eq!(deployed_xml.get(&key), Some(&MINIMAL_BPMN_XML.to_string()));

        let engine_stats = engine.get_stats().await;
        assert_eq!(engine_stats.definitions_count, 1);
        assert_eq!(engine_stats.instances_total, 1);
        assert_eq!(engine_stats.pending_user_tasks, 1);
        assert_eq!(engine_stats.pending_service_tasks, 1);
        assert_eq!(engine_stats.pending_timers, 1);
        assert_eq!(engine_stats.pending_message_catches, 1);
        assert_eq!(engine_stats.pending_signal_catches, 1);
        assert_eq!(engine_stats.pending_conditional_catches, 1);
    }

    #[tokio::test]
    async fn restore_falls_back_to_bpmn_xml_without_definition_json() {
        let persistence: Arc<dyn WorkflowPersistence> = Arc::new(InMemoryPersistence::new());
        let key = Uuid::new_v4();
        persistence
            .save_bpmn_xml(&key.to_string(), MINIMAL_BPMN_XML)
            .await
            .unwrap();

        let mut engine = WorkflowEngine::new();
        let mut deployed_xml = HashMap::new();
        let stats = StartupCoordinator::new(persistence)
            .restore(&mut engine, &mut deployed_xml)
            .await
            .expect("restore");

        assert_eq!(stats.definitions, 1);
        assert_eq!(engine.get_stats().await.definitions_count, 1);
        assert!(engine.get_definition(&key).await.is_some());
        assert_eq!(
            deployed_xml.get(&key.to_string()),
            Some(&MINIMAL_BPMN_XML.to_string())
        );
    }

    #[tokio::test]
    async fn restore_keeps_definitions_when_xml_is_missing() {
        let persistence: Arc<dyn WorkflowPersistence> = Arc::new(InMemoryPersistence::new());
        let definition = definition();
        let definition_key = definition.key;
        persistence.save_definition(&definition).await.unwrap();

        let mut engine = WorkflowEngine::new();
        let mut deployed_xml = HashMap::new();
        let stats = StartupCoordinator::new(persistence)
            .restore(&mut engine, &mut deployed_xml)
            .await
            .expect("restore");

        assert_eq!(stats.definitions, 1);
        assert!(engine.get_definition(&definition_key).await.is_some());
        assert!(deployed_xml.is_empty());
    }
}
