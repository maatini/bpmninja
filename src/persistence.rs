use async_nats::jetstream::{self, context::Context, stream::Config as StreamConfig};
use async_nats::Client;
use futures::StreamExt;
use std::collections::HashMap;

use crate::error::{EngineError, EngineResult};
use crate::model::Token;

/// A trait for persisting workflow engine state.
#[allow(async_fn_in_trait)]
pub trait WorkflowPersistence: Send + Sync {
    /// Save a token's state for a given process instance.
    async fn save_token(&self, token: &Token) -> EngineResult<()>;
    /// Load all tokens for a given process instance.
    async fn load_tokens(&self, process_id: &str) -> EngineResult<Vec<Token>>;
}

#[derive(Clone)]
pub struct NatsPersistence {
    #[allow(dead_code)]
    client: Client,
    js: Context,
    stream_name: String,
}

impl NatsPersistence {
    pub async fn connect(url: &str, stream_name: &str) -> EngineResult<Self> {
        let client = async_nats::connect(url).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to connect to NATS: {}", e))
        })?;
        
        let js = jetstream::new(client.clone());
        
        // Optional: Ensure the stream exists.
        // We ignore the error if it already exists.
        let _ = js
            .get_or_create_stream(StreamConfig {
                name: stream_name.to_string(),
                subjects: vec![format!("{}.*", stream_name)],
                ..Default::default()
            })
            .await;
            
        Ok(Self {
            client,
            js,
            stream_name: stream_name.to_string(),
        })
    }
}


impl WorkflowPersistence for NatsPersistence {
    async fn save_token(&self, token: &Token) -> EngineResult<()> {
        let subject = format!("{}.{}", self.stream_name, token.id);
        let payload = serde_json::to_vec(token).map_err(|e| {
            EngineError::PersistenceError(format!("Failed to serialize token: {}", e))
        })?;
        
        self.js
            .publish(subject, payload.into())
            .await
            .map_err(|e| {
                EngineError::PersistenceError(format!("Failed to publish to JetStream: {}", e))
            })?;
            
        Ok(())
    }

    async fn load_tokens(&self, _process_id: &str) -> EngineResult<Vec<Token>> {
        let stream = self.js.get_stream(&self.stream_name).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to get stream: {}", e))
        })?;
        
        let consumer = stream.create_consumer(async_nats::jetstream::consumer::pull::Config {
            deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
            ..Default::default()
        }).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to create consumer: {}", e))
        })?;
        
        let mut messages = consumer.messages().await.map_err(|e| {
            EngineError::PersistenceError(format!("Message stream error: {}", e))
        })?;
        
        let mut token_map = HashMap::new();
        
        while let Ok(Some(msg)) = tokio::time::timeout(std::time::Duration::from_millis(500), messages.next()).await {
            if let Ok(msg) = msg {
                let _ = msg.ack().await;
                if let Ok(token) = serde_json::from_slice::<Token>(&msg.payload) {
                    token_map.insert(token.id, token);
                }
            }
        }
        
        Ok(token_map.into_values().collect())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Arc;
    use uuid::Uuid;

    pub async fn setup_nats_test() -> Option<Arc<NatsPersistence>> {
        let url = "nats://localhost:4222";
        let stream = format!("TEST_STREAM_{}", Uuid::new_v4());
        
        match NatsPersistence::connect(url, &stream).await {
            Ok(persistence) => Some(Arc::new(persistence)),
            Err(e) => {
                log::warn!("Skipping NATS test, could not connect: {}", e);
                None
            }
        }
    }

    #[tokio::test]
    async fn test_save_and_load_token() {
        let persistence = match setup_nats_test().await {
            Some(p) => p,
            None => return, // Ignore if NATS container is not running
        };

        let mut token = Token::new("start_node");
        token.variables.insert("test_key".into(), serde_json::Value::String("test_value".into()));

        persistence.save_token(&token).await.unwrap();

        // Event-Sourcing Light Scenario
        token.current_node = "next_node".to_string();
        persistence.save_token(&token).await.unwrap();

        let loaded_tokens = persistence.load_tokens("some_process_id").await.unwrap();
        
        assert_eq!(loaded_tokens.len(), 1);
        let loaded_token = &loaded_tokens[0];
        
        assert_eq!(loaded_token.id, token.id);
        assert_eq!(loaded_token.current_node, "next_node");
        assert_eq!(loaded_token.variables.get("test_key").unwrap().as_str().unwrap(), "test_value");
    }
}
