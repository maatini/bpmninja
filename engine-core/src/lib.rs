pub mod condition;
pub mod domain;
pub mod runtime;
pub mod port;
pub mod adapter;
pub mod engine;
pub mod history;
pub mod scripting;

pub use condition::evaluate_condition;

// Backward-compatible re-exports (existing code doesn't break)
pub use domain::*;
pub use runtime::*;
pub use port::*;
pub use adapter::*;
pub use engine::WorkflowEngine;
pub use history::{HistoryDiff, HistoryEntry, HistoryEventType, VariableDiff};
pub use scripting::*;

// Legacy module aliases for downstream crates
// TODO: Remove once all downstream crates are migrated to new paths
pub use domain as model;
pub use port as persistence;
pub use domain::timer as timer_definition;
