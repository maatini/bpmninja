pub mod adapter;
pub mod condition;
pub mod domain;
pub mod engine;
pub mod history;
pub mod port;
pub mod runtime;
pub mod scripting;

pub use condition::evaluate_condition;

pub use domain::*;
pub use engine::WorkflowEngine;
pub use history::{HistoryDiff, HistoryEntry, HistoryEventType, VariableDiff};
pub use port::*;
pub use runtime::*;
pub use scripting::*;

/// Stabile Modulaliase für Downstream-Crates (`bpmn-parser`, `engine-server`, Persistenz).
pub use domain as model;
pub use domain::timer as timer_definition;
pub use port as persistence;
