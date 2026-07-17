// Re-export the app builder so integration tests can use it.
pub mod log_buffer;
pub mod log_nats;
pub mod observability;
pub mod startup;
mod server;
pub use log_buffer::LogBuffer;
pub use log_nats::NatsLogSink;
pub use server::{
    AppBuildConfig, build_app, build_app_with_config, build_app_with_engine, build_app_with_options,
};
pub use startup::StartupCoordinator;
