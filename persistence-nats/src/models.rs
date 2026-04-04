use serde::Serialize;

/// Information about the connected NATS server and JetStream account.
#[derive(Debug, Clone, Serialize)]
pub struct NatsInfo {
    pub server_name: String,
    pub version: String,
    pub host: String,
    pub port: u16,
    pub max_payload: usize,
    /// JetStream memory usage in bytes.
    pub js_memory_bytes: u64,
    /// JetStream file storage usage in bytes.
    pub js_storage_bytes: u64,
    /// Number of active JetStream streams.
    pub js_streams: usize,
    /// Number of active JetStream consumers.
    pub js_consumers: usize,
}
