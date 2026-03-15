use async_trait::async_trait;

use crate::error::EngineResult;
use crate::model::Token;

/// A trait for persisting workflow engine state.
#[async_trait]
pub trait WorkflowPersistence: Send + Sync {
    /// Save a token's state for a given process instance.
    async fn save_token(&self, token: &Token) -> EngineResult<()>;
    /// Load all tokens for a given process instance.
    async fn load_tokens(&self, process_id: &str) -> EngineResult<Vec<Token>>;
}
