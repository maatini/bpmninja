use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EngineStats {
    pub definitions_count: usize,
    pub instances_total: usize,
    pub instances_running: usize,
    pub instances_completed: usize,
    pub instances_waiting_user: usize,
    pub instances_waiting_service: usize,
    pub pending_user_tasks: usize,
    pub pending_service_tasks: usize,
    pub pending_timers: usize,
    pub pending_message_catches: usize,
    /// Number of persistence write failures since engine start.
    pub persistence_errors: u64,
    /// Number of pending retry jobs in the background queue (0 = healthy).
    pub pending_retry_jobs: usize,
}
