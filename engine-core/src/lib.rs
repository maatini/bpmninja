pub mod error;
pub mod model;
pub mod engine;

// Re-export main structures
pub use error::{EngineError, Result};
pub use model::{ProcessDefinition, BpmnElement, ProcessInstance, Token, UserTaskInfo, ServiceHandler};
pub use engine::WorkflowEngine;
