// Re-export the app builder so integration tests can use it.
pub mod observability;
mod server;
pub use server::{build_app, build_app_with_engine};
