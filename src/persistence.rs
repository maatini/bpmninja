use async_nats::jetstream::{self, context::Context, stream::Config as StreamConfig};
use async_nats::Client;

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
        // A minimal implementation would use a KV store or a specialized consumer
        // to rebuild current token state. For now, we return empty to let the
        // engine start fresh. Full event sourcing requires rebuilding from the stream.
        Ok(vec![])
    }
}
