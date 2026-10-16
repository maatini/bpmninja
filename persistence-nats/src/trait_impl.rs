use async_trait::async_trait;
use futures::StreamExt;
use uuid::Uuid;

use engine_core::error::{EngineError, EngineResult};
use engine_core::model::{ProcessDefinition, Token};
use engine_core::persistence::WorkflowPersistence;
use engine_core::{PendingServiceTask, PendingUserTask, ProcessInstance};

use crate::client::NatsPersistence;

#[async_trait]
impl WorkflowPersistence for NatsPersistence {
    async fn reserve_conditional_start(
        &self,
        receipt: &engine_core::ConditionalStartReceipt,
    ) -> EngineResult<engine_core::ConditionalStartReceipt> {
        let store = self.kv("conditional_starts").await?;
        let payload = serde_json::to_vec(receipt)
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
        match store
            .create(receipt.request.request_id.to_string(), payload.into())
            .await
        {
            Ok(_) => Ok(receipt.clone()),
            Err(error) => self
                .get_conditional_start(receipt.request.request_id)
                .await?
                .ok_or_else(|| {
                    EngineError::PersistenceError(format!("Receipt reservation failed: {error}"))
                }),
        }
    }
    async fn get_conditional_start(
        &self,
        id: Uuid,
    ) -> EngineResult<Option<engine_core::ConditionalStartReceipt>> {
        let store = self.kv("conditional_starts").await?;
        store
            .get(id.to_string())
            .await
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?
            .map(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|e| EngineError::PersistenceError(e.to_string()))
            })
            .transpose()
    }
    async fn list_conditional_starts(
        &self,
    ) -> EngineResult<Vec<engine_core::ConditionalStartReceipt>> {
        let store = self.kv("conditional_starts").await?;
        let mut keys = store
            .keys()
            .await
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
        let mut receipts = Vec::new();
        while let Some(key) = keys.next().await {
            let key = key.map_err(|e| EngineError::PersistenceError(e.to_string()))?;
            let id =
                Uuid::parse_str(&key).map_err(|e| EngineError::PersistenceError(e.to_string()))?;
            if let Some(receipt) = self.get_conditional_start(id).await? {
                receipts.push(receipt);
            }
        }
        Ok(receipts)
    }
    async fn complete_conditional_start(
        &self,
        receipt: &engine_core::ConditionalStartReceipt,
    ) -> EngineResult<()> {
        let store = self.kv("conditional_starts").await?;
        let key = receipt.request.request_id.to_string();
        let entry = store
            .entry(&key)
            .await
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?
            .ok_or_else(|| {
                EngineError::PersistenceError("Missing conditional-start receipt".into())
            })?;
        let mut stored: engine_core::ConditionalStartReceipt = serde_json::from_slice(&entry.value)
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
        if stored.request != receipt.request || stored.result != receipt.result {
            return Err(EngineError::ConditionalStartConflict(
                receipt.request.request_id,
            ));
        }
        if stored.dispatched {
            return Ok(());
        }
        stored.dispatched = true;
        let payload = serde_json::to_vec(&stored)
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
        store
            .update(key, payload.into(), entry.revision)
            .await
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
        Ok(())
    }

    async fn save_token(&self, instance_id: Uuid, token: &Token) -> EngineResult<()> {
        let store = self.kv("tokens").await?;
        let key = format!("token-{instance_id}-{}", token.id);
        let payload = serde_json::to_vec(token).map_err(|e| {
            EngineError::PersistenceError(format!("Failed to serialize token: {}", e))
        })?;
        store.put(key, payload.into()).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to put token to KV: {}", e))
        })?;
        Ok(())
    }

    async fn load_tokens(&self, instance_id: Uuid) -> EngineResult<Vec<Token>> {
        let store = self.kv("tokens").await?;
        let prefix = format!("token-{instance_id}-");
        let mut keys = store.keys().await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to list token keys: {}", e))
        })?;
        let mut tokens = Vec::new();
        while let Some(key_result) = keys.next().await {
            match key_result {
                Ok(key) if key.starts_with(&prefix) => match store.get(&key).await {
                    Ok(Some(data)) => match serde_json::from_slice::<Token>(&data) {
                        Ok(token) => tokens.push(token),
                        Err(e) => {
                            tracing::warn!("Token '{}' deserialisieren fehlgeschlagen: {}", key, e)
                        }
                    },
                    Ok(None) => {}
                    Err(e) => tracing::warn!("Token '{}' lesen fehlgeschlagen: {}", key, e),
                },
                Ok(_) => {}
                Err(e) => tracing::warn!("Token-Key lesen fehlgeschlagen: {}", e),
            }
        }
        Ok(tokens)
    }

    async fn delete_token(&self, instance_id: Uuid, token_id: Uuid) -> EngineResult<()> {
        let store = self.kv("tokens").await?;
        let key = format!("token-{instance_id}-{token_id}");
        store.delete(key).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to delete token from KV: {}", e))
        })?;
        Ok(())
    }

    async fn save_instance(&self, instance: &ProcessInstance) -> EngineResult<()> {
        #[cfg(feature = "separate-audit-log")]
        let json = crate::codec::prepare_detached(instance)?;
        #[cfg(not(feature = "separate-audit-log"))]
        let json = crate::codec::encode_instance(instance)?;
        self.put_instance_json(instance.id, json).await
    }

    async fn save_instance_json(
        &self,
        instance: &ProcessInstance,
        json: &[u8],
    ) -> EngineResult<()> {
        let payload = self.prepare_instance_payload(instance, json.to_vec())?;
        self.save_instance_payload(instance, &payload).await
    }

    fn prepare_instance_payload(
        &self,
        instance: &ProcessInstance,
        json: Vec<u8>,
    ) -> EngineResult<Vec<u8>> {
        #[cfg(feature = "separate-audit-log")]
        {
            let _ = json;
            crate::codec::prepare_detached(instance)
        }
        #[cfg(all(not(feature = "separate-audit-log"), feature = "compact-instances"))]
        {
            let _ = json;
            crate::codec::encode_instance(instance)
        }
        #[cfg(not(any(feature = "separate-audit-log", feature = "compact-instances")))]
        {
            let _ = instance;
            Ok(json)
        }
    }

    async fn save_instance_payload(
        &self,
        instance: &ProcessInstance,
        payload: &[u8],
    ) -> EngineResult<()> {
        self.put_instance_json(instance.id, payload.to_vec()).await
    }

    async fn list_instances(&self) -> EngineResult<Vec<ProcessInstance>> {
        self.list_kv_entries("instances", "instance").await
    }

    async fn delete_instance(&self, id: &str) -> EngineResult<()> {
        let _audit_access = crate::audit::AUDIT_ACCESS.write().await;
        #[cfg(feature = "delta-instances")]
        self.snapshot_cache.lock().await.remove(&(
            "instances".to_string(),
            Uuid::parse_str(id).map_err(|e| EngineError::PersistenceError(e.to_string()))?,
        ));
        let store = self.kv("instances").await?;

        store.delete(id).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to delete instance from KV: {}", e))
        })?;

        // Completed archives retain their audit snapshots after active deletion.
        if self
            .kv("history_instances")
            .await?
            .get(id)
            .await
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?
            .is_none()
        {
            self.delete_instance_audits(id).await?;
            self.delete_instance_states(id).await?;
        }

        Ok(())
    }

    async fn save_definition(&self, definition: &ProcessDefinition) -> EngineResult<()> {
        let store = self.kv("definitions").await?;

        let json = serde_json::to_vec(definition).map_err(|e| {
            EngineError::PersistenceError(format!("Failed to serialize definition: {}", e))
        })?;

        store
            .put(definition.key.to_string(), json.into())
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to put definition to KV: {}", e))
            })?;

        Ok(())
    }

    async fn list_definitions(&self) -> EngineResult<Vec<ProcessDefinition>> {
        self.list_kv_entries("definitions", "definition").await
    }

    async fn delete_definition(&self, key: &str) -> EngineResult<()> {
        let store = self.kv("definitions").await?;

        store.delete(key).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to delete definition from KV: {}", e))
        })?;

        let obj_store = self.object_store("bpmn_xml").await?;

        let _ = obj_store.delete(key).await;

        Ok(())
    }

    async fn save_user_task(&self, task: &PendingUserTask) -> EngineResult<()> {
        let store = self.kv("user_tasks").await?;

        let json = serde_json::to_vec(task).map_err(|e| {
            EngineError::PersistenceError(format!("Failed to serialize user task: {}", e))
        })?;

        store
            .put(task.task_id.to_string(), json.into())
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to put user task to KV: {}", e))
            })?;

        Ok(())
    }

    async fn delete_user_task(&self, task_id: uuid::Uuid) -> EngineResult<()> {
        let store = self.kv("user_tasks").await?;

        store.delete(task_id.to_string()).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to delete user task from KV: {}", e))
        })?;

        Ok(())
    }

    async fn list_user_tasks(&self) -> EngineResult<Vec<PendingUserTask>> {
        self.list_kv_entries("user_tasks", "user task").await
    }

    async fn save_service_task(&self, task: &PendingServiceTask) -> EngineResult<()> {
        let store = self.kv("service_tasks").await?;

        let json = serde_json::to_vec(task).map_err(|e| {
            EngineError::PersistenceError(format!("Failed to serialize service task: {}", e))
        })?;

        store
            .put(task.id.to_string(), json.into())
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to put service task to KV: {}", e))
            })?;

        Ok(())
    }

    async fn delete_service_task(&self, task_id: uuid::Uuid) -> EngineResult<()> {
        let store = self.kv("service_tasks").await?;

        store.delete(task_id.to_string()).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to delete service task from KV: {}", e))
        })?;

        Ok(())
    }

    async fn list_service_tasks(&self) -> EngineResult<Vec<PendingServiceTask>> {
        self.list_kv_entries("service_tasks", "service task").await
    }

    async fn save_timer(&self, timer: &engine_core::PendingTimer) -> EngineResult<()> {
        let store = self.kv("timers").await?;

        let json = serde_json::to_vec(timer).map_err(|e| {
            EngineError::PersistenceError(format!("Failed to serialize timer: {}", e))
        })?;

        store
            .put(timer.id.to_string(), json.into())
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to put timer to KV: {}", e))
            })?;

        Ok(())
    }

    async fn delete_timer(&self, timer_id: uuid::Uuid) -> EngineResult<()> {
        let store = self.kv("timers").await?;

        store.delete(timer_id.to_string()).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to delete timer from KV: {}", e))
        })?;

        Ok(())
    }

    async fn list_timers(&self) -> EngineResult<Vec<engine_core::PendingTimer>> {
        self.list_kv_entries("timers", "timer").await
    }

    async fn save_message_catch(
        &self,
        catch: &engine_core::PendingMessageCatch,
    ) -> EngineResult<()> {
        let store = self.kv("messages").await?;

        let json = serde_json::to_vec(catch).map_err(|e| {
            EngineError::PersistenceError(format!("Failed to serialize message catch: {}", e))
        })?;

        store
            .put(catch.id.to_string(), json.into())
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to put message catch to KV: {}", e))
            })?;

        Ok(())
    }

    async fn delete_message_catch(&self, catch_id: uuid::Uuid) -> EngineResult<()> {
        let store = self.kv("messages").await?;

        store.delete(catch_id.to_string()).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to delete message catch from KV: {}", e))
        })?;

        Ok(())
    }

    async fn list_message_catches(&self) -> EngineResult<Vec<engine_core::PendingMessageCatch>> {
        self.list_kv_entries("messages", "message catch").await
    }

    async fn save_file(&self, object_key: &str, data: &[u8]) -> EngineResult<()> {
        const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50 MB
        if data.len() > MAX_FILE_SIZE {
            return Err(EngineError::InvalidDefinition(format!(
                "File exceeds maximum size of 50 MB ({} bytes)",
                data.len()
            )));
        }
        let store = self.object_store("instance_files").await?;
        let mut data_mut = data;
        store
            .put(object_key, &mut data_mut)
            .await
            .map_err(|e| EngineError::PersistenceError(format!("Failed to save file: {}", e)))?;
        Ok(())
    }

    async fn load_file(&self, object_key: &str) -> EngineResult<Vec<u8>> {
        use tokio::io::AsyncReadExt;
        let store = self.object_store("instance_files").await?;
        let mut result = store.get(object_key).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to load file '{}': {}", object_key, e))
        })?;
        let mut data = Vec::new();
        result.read_to_end(&mut data).await.map_err(|e| {
            EngineError::PersistenceError(format!("Error reading file data: {}", e))
        })?;
        Ok(data)
    }

    async fn delete_file(&self, object_key: &str) -> EngineResult<()> {
        let store = self.object_store("instance_files").await?;
        let _ = store.delete(object_key).await;
        Ok(())
    }

    async fn save_bpmn_xml(&self, definition_id: &str, xml: &str) -> EngineResult<()> {
        let store = self.object_store("bpmn_xml").await?;

        store
            .put(definition_id, &mut xml.as_bytes())
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to store BPMN XML: {}", e))
            })?;

        Ok(())
    }

    async fn load_bpmn_xml(&self, definition_id: &str) -> EngineResult<String> {
        use tokio::io::AsyncReadExt;

        let store = self.object_store("bpmn_xml").await?;

        let mut result = store.get(definition_id).await.map_err(|e| {
            EngineError::PersistenceError(format!(
                "Failed to load BPMN XML for '{}': {}",
                definition_id, e
            ))
        })?;

        let mut data = Vec::new();
        result
            .read_to_end(&mut data)
            .await
            .map_err(|e| EngineError::PersistenceError(format!("Error reading XML data: {}", e)))?;

        String::from_utf8(data).map_err(|e| {
            EngineError::PersistenceError(format!("BPMN XML is not valid UTF-8: {}", e))
        })
    }

    async fn list_bpmn_xml_ids(&self) -> EngineResult<Vec<String>> {
        let store = self.object_store("bpmn_xml").await?;

        let mut list = store.list().await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to list bpmn_xml objects: {}", e))
        })?;

        let mut ids = Vec::new();
        while let Some(info) = list.next().await {
            if let Ok(info) = info {
                ids.push(info.name);
            }
        }
        Ok(ids)
    }

    async fn get_storage_info(
        &self,
    ) -> EngineResult<Option<engine_core::persistence::StorageInfo>> {
        let si = self.client.server_info();
        let account = self.js.query_account().await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to query JetStream account: {}", e))
        })?;

        let mut buckets = Vec::new();

        // Collect KV bucket stats
        let kv_bucket_names = [
            "instances",
            "definitions",
            "user_tasks",
            "service_tasks",
            "timers",
            "messages",
            "history_instances",
            "instance_audit",
            "instance_states",
        ];
        for name in &kv_bucket_names {
            match self.js.get_key_value(*name).await {
                Ok(store) => match store.status().await {
                    Ok(status) => {
                        buckets.push(engine_core::persistence::BucketInfo {
                            name: name.to_string(),
                            bucket_type: "kv".to_string(),
                            entries: status.values(),
                            size_bytes: status.info.state.bytes,
                        });
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get status for KV bucket '{}': {}", name, e)
                    }
                },
                Err(e) => tracing::warn!("Failed to access KV bucket '{}': {}", name, e),
            }
        }

        // Collect ObjectStore bucket stats
        let obj_bucket_names = ["bpmn_xml", "instance_files"];
        for name in &obj_bucket_names {
            let stream_name = format!("OBJ_{}", name);
            match self.js.get_stream(&stream_name).await {
                Ok(mut stream) => match stream.info().await {
                    Ok(info) => {
                        buckets.push(engine_core::persistence::BucketInfo {
                            name: name.to_string(),
                            bucket_type: "object_store".to_string(),
                            entries: info.state.messages,
                            size_bytes: info.state.bytes,
                        });
                    }
                    Err(e) => tracing::warn!(
                        "Failed to get info for underlying ObjectStore stream '{}': {}",
                        name,
                        e
                    ),
                },
                Err(e) => tracing::warn!(
                    "Failed to access underlying ObjectStore stream '{}': {}",
                    name,
                    e
                ),
            }
        }

        // Collect Stream stats
        let stream_names = ["WORKFLOW_EVENTS", "WORKFLOW_HISTORY"];
        for name in &stream_names {
            match self.js.get_stream(*name).await {
                Ok(mut stream) => match stream.info().await {
                    Ok(info) => {
                        buckets.push(engine_core::persistence::BucketInfo {
                            name: name.to_string(),
                            bucket_type: "stream".to_string(),
                            entries: info.state.messages,
                            size_bytes: info.state.bytes,
                        });
                    }
                    Err(e) => tracing::warn!("Failed to get info for stream '{}': {}", name, e),
                },
                Err(e) => tracing::warn!("Failed to access stream '{}': {}", name, e),
            }
        }

        Ok(Some(engine_core::persistence::StorageInfo {
            backend_name: si.server_name.clone(),
            version: si.version.clone(),
            host: si.host.clone(),
            port: si.port,
            memory_bytes: account.memory,
            storage_bytes: account.storage,
            streams: account.streams,
            consumers: account.consumers,
            buckets,
        }))
    }

    async fn append_history_entry(
        &self,
        entry: &engine_core::history::HistoryEntry,
    ) -> EngineResult<()> {
        self.append_history_entries(std::slice::from_ref(entry))
            .await
    }

    /// Publishes all entries first, then awaits every JetStream ack: one
    /// round trip for the batch instead of one per entry. `Nats-Msg-Id` =
    /// entry id lets JetStream drop duplicates when a failed batch is retried.
    async fn append_history_entries(
        &self,
        entries: &[engine_core::history::HistoryEntry],
    ) -> EngineResult<()> {
        let mut acks = Vec::with_capacity(entries.len());
        for entry in entries {
            let subject = format!("history.instance.{}", entry.instance_id);
            let payload = serde_json::to_vec(entry).map_err(|e| {
                EngineError::PersistenceError(format!("Failed to serialize history entry: {}", e))
            })?;
            let mut headers = async_nats::HeaderMap::new();
            headers.insert("Nats-Msg-Id", entry.id.to_string().as_str());
            let ack = self
                .js
                .publish_with_headers(subject, headers, payload.into())
                .await
                .map_err(|e| {
                    EngineError::PersistenceError(format!(
                        "Failed to publish history entry to JetStream: {}",
                        e
                    ))
                })?;
            acks.push(ack);
        }
        for ack in acks {
            ack.await.map_err(|e| {
                EngineError::PersistenceError(format!(
                    "History entry not acknowledged by JetStream: {}",
                    e
                ))
            })?;
        }
        Ok(())
    }

    async fn query_history(
        &self,
        query: engine_core::persistence::HistoryQuery,
    ) -> EngineResult<Vec<engine_core::history::HistoryEntry>> {
        let stream = self.js.get_stream("WORKFLOW_HISTORY").await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to get WORKFLOW_HISTORY stream: {}", e))
        })?;

        let subject = format!("history.instance.{}", query.instance_id);

        let consumer = stream
            .create_consumer(async_nats::jetstream::consumer::pull::Config {
                deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
                ack_policy: async_nats::jetstream::consumer::AckPolicy::None,
                filter_subject: subject.clone(),
                ..Default::default()
            })
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to create history consumer: {}", e))
            })?;

        let mut messages = consumer
            .messages()
            .await
            .map_err(|e| EngineError::PersistenceError(format!("Message stream error: {}", e)))?;

        let mut entries = Vec::new();

        // Timeout to drain all existing messages
        while let Ok(Some(msg)) =
            tokio::time::timeout(std::time::Duration::from_millis(100), messages.next()).await
        {
            if let Ok(msg) = msg
                && let Ok(entry) =
                    serde_json::from_slice::<engine_core::history::HistoryEntry>(&msg.payload)
            {
                let mut matched = true;
                if let Some(types) = &query.event_types
                    && !types.contains(&entry.event_type)
                {
                    matched = false;
                }
                if let Some(nid) = &query.node_id
                    && entry.node_id.as_deref() != Some(nid)
                {
                    matched = false;
                }
                if let Some(aty) = &query.actor_type
                    && &entry.actor_type != aty
                {
                    matched = false;
                }
                if let Some(f) = query.from
                    && entry.timestamp < f
                {
                    matched = false;
                }
                if let Some(t) = query.to
                    && entry.timestamp > t
                {
                    matched = false;
                }

                if matched {
                    entries.push(entry);
                }
            }
        }

        // Ensure chronological order
        entries.sort_by_key(|e| e.timestamp);

        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(entries.len());

        Ok(entries.into_iter().skip(offset).take(limit).collect())
    }

    async fn get_bucket_entries(
        &self,
        bucket_name: &str,
        offset: usize,
        limit: usize,
    ) -> EngineResult<Vec<engine_core::persistence::BucketEntry>> {
        use engine_core::persistence::BucketEntry;
        let mut entries = Vec::new();

        let obj_bucket_names = ["bpmn_xml", "instance_files"];
        let kv_bucket_names = [
            "instances",
            "definitions",
            "user_tasks",
            "service_tasks",
            "timers",
            "messages",
            "history_instances",
            "instance_audit",
            "instance_states",
        ];

        if kv_bucket_names.contains(&bucket_name) {
            let store = self.js.get_key_value(bucket_name).await.map_err(|e| {
                EngineError::PersistenceError(format!("Failed to get {} KV: {}", bucket_name, e))
            })?;

            let mut keys = store.keys().await.map_err(|e| {
                EngineError::PersistenceError(format!(
                    "Failed to list KV keys for {}: {}",
                    bucket_name, e
                ))
            })?;

            let mut all_keys = Vec::new();
            while let Some(Ok(key)) = keys.next().await {
                all_keys.push(key);
            }
            // Sort to ensure deterministic pagination across multiple calls
            all_keys.sort();

            let page_keys = all_keys
                .into_iter()
                .skip(offset)
                .take(limit)
                .collect::<Vec<_>>();
            for key in page_keys {
                if let Ok(Some(entry)) = store.get(&key).await {
                    entries.push(BucketEntry {
                        key,
                        size_bytes: Some(entry.len() as u64),
                        created_at: Some(chrono::Utc::now()), // KV get doesn't expose metadata like created_at directly easily here
                    });
                } else {
                    entries.push(BucketEntry {
                        key,
                        size_bytes: None,
                        created_at: None,
                    });
                }
            }
        } else if obj_bucket_names.contains(&bucket_name) {
            let store = self.js.get_object_store(bucket_name).await.map_err(|e| {
                EngineError::PersistenceError(format!(
                    "Failed to get {} Object Store: {}",
                    bucket_name, e
                ))
            })?;

            let mut list = store.list().await.map_err(|e| {
                EngineError::PersistenceError(format!(
                    "Failed to list Object Store {}: {}",
                    bucket_name, e
                ))
            })?;

            let mut metadata_list = Vec::new();
            while let Some(Ok(info)) = list.next().await {
                metadata_list.push(info);
            }
            // Sort by name
            metadata_list.sort_by(|a, b| a.name.cmp(&b.name));

            for info in metadata_list.into_iter().skip(offset).take(limit) {
                let Some(dt) = info.modified else {
                    entries.push(BucketEntry {
                        key: info.name,
                        size_bytes: Some(info.size as u64),
                        created_at: None,
                    });
                    continue;
                };

                let nt = chrono::DateTime::from_timestamp(dt.unix_timestamp(), dt.nanosecond());
                entries.push(BucketEntry {
                    key: info.name,
                    size_bytes: Some(info.size as u64),
                    created_at: nt,
                });
            }
        } else if bucket_name == "WORKFLOW_HISTORY" || bucket_name == "WORKFLOW_EVENTS" {
            // For Streams, just dump latest messages sequence
            let mut stream = self.js.get_stream(bucket_name).await.map_err(|e| {
                EngineError::PersistenceError(format!(
                    "Failed to get stream {}: {}",
                    bucket_name, e
                ))
            })?;

            // To properly do this for streams genericly, we get the state
            let info = stream.info().await.map_err(|e| {
                EngineError::PersistenceError(format!("Failed to get stream info: {}", e))
            })?;

            // Just simulate keys based on first/last sequence
            let start = info.state.first_sequence.saturating_add(offset as u64);
            let last_seq = info.state.last_sequence;
            let _ = info; // Free any borrow

            let mut current = start;
            let mut count = 0;

            while count < limit && current <= last_seq {
                if let Ok(msg) = stream.get_raw_message(current).await {
                    entries.push(BucketEntry {
                        key: current.to_string(),
                        size_bytes: Some(msg.payload.len() as u64),
                        created_at: Some(
                            chrono::DateTime::from_timestamp(
                                msg.time.unix_timestamp(),
                                msg.time.nanosecond(),
                            )
                            .unwrap_or_else(chrono::Utc::now),
                        ),
                    });
                }
                current += 1;
                count += 1;
            }
        } else {
            return Err(EngineError::PersistenceError(format!(
                "Unknown bucket name: {}",
                bucket_name
            )));
        }

        Ok(entries)
    }

    async fn get_bucket_entry_detail(
        &self,
        bucket_name: &str,
        key: &str,
    ) -> EngineResult<engine_core::persistence::BucketEntryDetail> {
        let _audit_access = crate::audit::AUDIT_ACCESS.read().await;
        use engine_core::persistence::BucketEntryDetail;

        let obj_bucket_names = ["bpmn_xml", "instance_files"];
        let kv_bucket_names = [
            "instances",
            "definitions",
            "user_tasks",
            "service_tasks",
            "timers",
            "messages",
            "history_instances",
            "instance_audit",
            "instance_states",
        ];

        if kv_bucket_names.contains(&bucket_name) {
            let store = self.js.get_key_value(bucket_name).await.map_err(|e| {
                EngineError::PersistenceError(format!("Failed to get {} KV: {}", bucket_name, e))
            })?;

            if let Ok(Some(entry)) = store.get(key).await {
                if bucket_name == "instance_states" {
                    use base64::{Engine as _, engine::general_purpose::STANDARD};
                    return Ok(BucketEntryDetail {
                        key: key.to_string(),
                        data: STANDARD.encode(&entry),
                        encoding: "base64".into(),
                    });
                }
                let data = if crate::codec::detached(&entry) || crate::journal::is_head(&entry) {
                    serde_json::to_string(&self.restore_instance_record(&entry).await?)
                        .map_err(|e| EngineError::PersistenceError(e.to_string()))?
                } else {
                    crate::codec::monitoring_json(&entry)?
                };
                return Ok(BucketEntryDetail {
                    key: key.to_string(),
                    data,
                    encoding: "utf8".to_string(),
                });
            } else {
                return Err(EngineError::PersistenceError(format!(
                    "Key {} not found in KV {}",
                    key, bucket_name
                )));
            }
        } else if obj_bucket_names.contains(&bucket_name) {
            let store = self.js.get_object_store(bucket_name).await.map_err(|e| {
                EngineError::PersistenceError(format!(
                    "Failed to get {} Object Store: {}",
                    bucket_name, e
                ))
            })?;

            use tokio::io::AsyncReadExt;
            match store.get(key).await {
                Ok(mut result) => {
                    let mut data = Vec::new();
                    result.read_to_end(&mut data).await.map_err(|e| {
                        EngineError::PersistenceError(format!("Error reading object data: {}", e))
                    })?;

                    if bucket_name == "bpmn_xml" {
                        let xml = String::from_utf8(data)
                            .unwrap_or_else(|_| "Invalid UTF-8 XML".to_string());
                        Ok(BucketEntryDetail {
                            key: key.to_string(),
                            data: xml,
                            encoding: "utf8".to_string(),
                        })
                    } else {
                        // Return Base64 for raw files
                        use base64::{Engine as _, engine::general_purpose::STANDARD};
                        let b64 = STANDARD.encode(data);
                        Ok(BucketEntryDetail {
                            key: key.to_string(),
                            data: b64,
                            encoding: "base64".to_string(),
                        })
                    }
                }
                Err(e) => Err(EngineError::PersistenceError(format!(
                    "Key {} not found in Object Store {}: {}",
                    key, bucket_name, e
                ))),
            }
        } else if bucket_name == "WORKFLOW_HISTORY" || bucket_name == "WORKFLOW_EVENTS" {
            let stream = self.js.get_stream(bucket_name).await.map_err(|e| {
                EngineError::PersistenceError(format!(
                    "Failed to get stream {}: {}",
                    bucket_name, e
                ))
            })?;

            let seq: u64 = key.parse().map_err(|_| {
                EngineError::PersistenceError(format!("Invalid sequence ID {}", key))
            })?;

            match stream.get_raw_message(seq).await {
                Ok(msg) => {
                    let data = match String::from_utf8(msg.payload.to_vec()) {
                        Ok(s) => s,
                        Err(_) => "Binary Message".to_string(),
                    };
                    Ok(BucketEntryDetail {
                        key: key.to_string(),
                        data,
                        encoding: "utf8".to_string(),
                    })
                }
                Err(e) => Err(EngineError::PersistenceError(format!(
                    "Sequence {} not found in stream {}: {}",
                    key, bucket_name, e
                ))),
            }
        } else {
            Err(EngineError::PersistenceError(format!(
                "Unknown bucket name: {}",
                bucket_name
            )))
        }
    }

    async fn save_completed_instance(&self, instance: &ProcessInstance) -> EngineResult<()> {
        #[cfg(feature = "separate-audit-log")]
        let payload = crate::codec::prepare_detached(instance)?;
        #[cfg(not(feature = "separate-audit-log"))]
        let payload = crate::codec::encode_instance(instance)?;
        self.put_instance_record("history_instances", instance.id, payload)
            .await
    }

    async fn query_completed_instances(
        &self,
        query: engine_core::persistence::CompletedInstanceQuery,
    ) -> EngineResult<Vec<ProcessInstance>> {
        let mut all: Vec<ProcessInstance> = self
            .list_kv_entries("history_instances", "completed_instance")
            .await?;

        // Apply filters in-memory (same approach as query_history)
        all.retain(|inst| {
            if let Some(ref dk) = query.definition_key
                && inst.definition_key != *dk
            {
                return false;
            }
            if let Some(ref bk) = query.business_key
                && !inst.business_key.contains(bk.as_str())
            {
                return false;
            }
            if let Some(ref from) = query.from
                && let Some(ref completed) = inst.completed_at
                && completed < from
            {
                return false;
            }
            if let Some(ref to) = query.to
                && let Some(ref completed) = inst.completed_at
                && completed > to
            {
                return false;
            }
            if let Some(ref sf) = query.state_filter {
                let matches_state = match sf.as_str() {
                    "completed" => {
                        matches!(inst.state, engine_core::runtime::InstanceState::Completed)
                    }
                    "error" => matches!(
                        inst.state,
                        engine_core::runtime::InstanceState::CompletedWithError { .. }
                    ),
                    _ => true,
                };
                if !matches_state {
                    return false;
                }
            }
            true
        });

        // Sort by completed_at descending (newest first)
        all.sort_by_key(|inst| std::cmp::Reverse(inst.completed_at));

        // Pagination
        if let Some(offset) = query.offset {
            all = all.into_iter().skip(offset).collect();
        }
        if let Some(limit) = query.limit {
            all.truncate(limit);
        }

        Ok(all)
    }

    async fn get_completed_instance(&self, id: &str) -> EngineResult<Option<ProcessInstance>> {
        let _audit_access = crate::audit::AUDIT_ACCESS.read().await;
        let store = self.kv("history_instances").await?;

        match store.get(id).await {
            Ok(Some(data)) => {
                let inst = self.restore_instance_record(&data).await?;
                Ok(Some(inst))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(EngineError::PersistenceError(format!(
                "Failed to get completed instance {}: {}",
                id, e
            ))),
        }
    }
}

impl NatsPersistence {
    async fn put_instance_json(&self, id: Uuid, payload: Vec<u8>) -> EngineResult<()> {
        self.put_instance_record("instances", id, payload).await
    }

    async fn put_instance_record(
        &self,
        bucket: &str,
        id: Uuid,
        payload: Vec<u8>,
    ) -> EngineResult<()> {
        let _audit_access = crate::audit::AUDIT_ACCESS.read().await;
        let _snapshot_lock = crate::journal::lock_snapshot(id).await;
        let record = if crate::codec::is_prepared(&payload) {
            let (record, audit) = crate::codec::split_prepared(&payload)?;
            let (instance, key) = crate::codec::decode_instance_record(record)?;
            let key =
                key.ok_or_else(|| EngineError::PersistenceError("Missing audit reference".into()))?;
            if instance.id != id || key != crate::codec::audit_key(id, audit) {
                return Err(EngineError::PersistenceError(
                    "Audit snapshot does not match instance".into(),
                ));
            }
            self.ensure_instance_audit(&key, audit).await?;
            record
        } else {
            &payload
        };
        #[cfg(feature = "delta-instances")]
        {
            self.put_journal_snapshot(bucket, id, record).await
        }
        #[cfg(not(feature = "delta-instances"))]
        {
            self.kv(bucket)
                .await?
                .put(id.to_string(), record.to_vec().into())
                .await
                .map_err(|e| {
                    EngineError::PersistenceError(format!(
                        "Failed to put instance to {bucket}: {e}"
                    ))
                })?;
            Ok(())
        }
    }

    async fn ensure_instance_audit(&self, key: &str, bytes: &[u8]) -> EngineResult<()> {
        if self.audit_cache.lock().await.contains(key) {
            return Ok(());
        }
        let store = self.kv("instance_audit").await?;
        if let Err(error) = store.create(key, bytes.to_vec().into()).await {
            // Retry/another writer may have created the immutable snapshot.
            let existing = store
                .get(key)
                .await
                .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
            if existing.as_deref() != Some(bytes) {
                return Err(EngineError::PersistenceError(format!(
                    "Failed to create audit snapshot: {error}"
                )));
            }
        }
        let mut cache = self.audit_cache.lock().await;
        if cache.len() >= 1024 {
            cache.clear();
        }
        cache.insert(key.to_string());
        Ok(())
    }

    pub(crate) async fn restore_instance_record(
        &self,
        bytes: &[u8],
    ) -> EngineResult<ProcessInstance> {
        let record = self.resolve_snapshot(bytes).await?;
        let (mut instance, key) = crate::codec::decode_instance_record(&record)?;
        if let Some(key) = key {
            let audit = self
                .kv("instance_audit")
                .await?
                .get(&key)
                .await
                .map_err(|e| {
                    EngineError::PersistenceError(format!("Failed to load audit snapshot: {e}"))
                })?
                .ok_or_else(|| {
                    EngineError::PersistenceError(format!("Missing audit snapshot {key}"))
                })?;
            if crate::codec::audit_key(instance.id, &audit) != key {
                return Err(EngineError::PersistenceError(format!(
                    "Audit snapshot digest mismatch: {key}"
                )));
            }
            instance.audit_log = serde_json::from_slice(&audit).map_err(|e| {
                EngineError::PersistenceError(format!("Invalid audit snapshot: {e}"))
            })?;
        }
        Ok(instance)
    }

    async fn delete_instance_audits(&self, id: &str) -> EngineResult<()> {
        let prefix = format!("{id}.");
        self.audit_cache
            .lock()
            .await
            .retain(|key| !key.starts_with(&prefix));
        let store = self.kv("instance_audit").await?;
        let mut keys = store
            .keys()
            .await
            .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
        while let Some(key) = keys.next().await {
            let key = key.map_err(|e| EngineError::PersistenceError(e.to_string()))?;
            if key.starts_with(&prefix) {
                store
                    .purge(&key)
                    .await
                    .map_err(|e| EngineError::PersistenceError(e.to_string()))?;
            }
        }
        self.audit_cache
            .lock()
            .await
            .retain(|key| !key.starts_with(&prefix));
        Ok(())
    }
    async fn delete_instance_states(&self, id: &str) -> EngineResult<()> {
        let prefix = format!("{id}.");
        let store = self.kv("instance_states").await?;
        let mut keys = store.keys().await.map_err(crate::journal::failure)?;
        while let Some(key) = keys.next().await {
            let key = key.map_err(crate::journal::failure)?;
            if key.starts_with(&prefix) {
                store.purge(&key).await.map_err(crate::journal::failure)?;
            }
        }
        Ok(())
    }
}
