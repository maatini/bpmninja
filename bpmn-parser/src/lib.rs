mod collaboration;
mod data;
mod event_subprocess;
pub(crate) mod models;
pub mod parser;

#[cfg(test)]
mod tests;

pub use collaboration::{
    Collaboration, CollaborationDocument, MessageEndpoint, MessageFlow, Participant,
    parse_bpmn_collaboration,
};
pub use parser::{parse_bpmn_processes, parse_bpmn_xml};

#[cfg(test)]
mod data_tests;

#[cfg(test)]
mod event_subprocess_tests;

#[cfg(test)]
mod collaboration_tests;
