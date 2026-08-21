// Re-export the app builder so integration tests can use it.
pub mod log_buffer;
pub mod log_nats;
pub mod observability;
mod server;
pub mod startup;
pub use log_buffer::LogBuffer;
pub use log_nats::NatsLogSink;
pub use server::{
    AppBuildConfig, build_app, build_app_with_config, build_app_with_engine,
    build_app_with_options, require_nats_from_env,
};
pub use startup::StartupCoordinator;
