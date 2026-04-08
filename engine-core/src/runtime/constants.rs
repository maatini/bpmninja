// Engine Constants

/// Maximum number of audit log entries retained in-memory per instance.
/// Older entries are available via the History API.
pub const MAX_AUDIT_LOG_ENTRIES: usize = 200;

/// Yield to the Tokio scheduler every N execution steps to prevent
/// thread starvation on long-running or looping BPMN processes.
pub const YIELD_EVERY_N_STEPS: u32 = 64;

/// Hard limit on execution steps per `run_instance_batch` call.
/// Prevents infinite BPMN loops from blocking the engine indefinitely.
pub const MAX_EXECUTION_STEPS: u32 = 10_000;

/// Maximum serialized size for a single ProcessInstance KV entry (900 KB).
/// NATS default max_payload is 1 MB; we leave headroom for protocol overhead.
pub const MAX_INSTANCE_PAYLOAD_BYTES: usize = 900 * 1024;
