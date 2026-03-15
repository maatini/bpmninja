use thiserror::Error;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Engine errors — thiserror-derived for zero boilerplate
// ---------------------------------------------------------------------------

/// All errors that can occur within the BPMN engine.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    /// The process definition is structurally invalid.
    #[error("Invalid definition: {0}")]
    InvalidDefinition(String),

    /// A referenced node does not exist in the definition.
    #[error("No such node: {0}")]
    NoSuchNode(String),

    /// No process definition found for the given ID.
    #[error("No such definition: {0}")]
    NoSuchDefinition(String),

    /// No process instance found for the given ID.
    #[error("No such instance: {0}")]
    NoSuchInstance(Uuid),

    /// Tried to complete a user task that is not currently pending.
    #[error("Task '{task_id}' is not pending (current state: {actual_state})")]
    TaskNotPending {
        task_id: Uuid,
        actual_state: String,
    },

    /// The process instance has already completed.
    #[error("Process instance has already completed")]
    #[allow(dead_code)]
    AlreadyCompleted,

    /// The timer duration does not match the start event's configuration.
    #[error("Timer mismatch: expected {expected}s, got {provided}s")]
    TimerMismatch { expected: u64, provided: u64 },

    /// A required service handler is not registered.
    #[error("No service handler registered for '{0}'")]
    HandlerNotFound(String),

    #[error("Persistence error: {0}")]
    PersistenceError(String),
}

/// Convenience alias used throughout the engine.
pub type EngineResult<T> = Result<T, EngineError>;
