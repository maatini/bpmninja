// Re-export the app builder so integration tests can use it.
pub mod log_buffer;
pub mod observability;
mod server;
pub use log_buffer::LogBuffer;
pub use server::{build_app, build_app_with_engine};
