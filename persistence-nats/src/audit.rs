//! Audit maintenance for single-process deployments.
use crate::NatsPersistence;
use engine_core::error::{EngineError, EngineResult};
use futures::StreamExt;
use std::collections::HashSet;
use tokio::sync::RwLock;
use uuid::Uuid;

// Shared across connections and clones, including connections made on restore.
// This is a process-local gate, not a distributed lease.
pub(crate) static AUDIT_ACCESS: RwLock<()> = RwLock::const_new(());

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditPruneReport {
    pub instance_id: Uuid,
    pub dry_run: bool,
    pub retained: Vec<String>,
    pub candidates: Vec<String>,
    pub purged: usize,
}

fn failure(error: impl std::fmt::Display) -> EngineError {
    EngineError::PersistenceError(format!("Audit cleanup: {error}"))
}

impl NatsPersistence {
    /// Plan or apply cleanup of unreferenced audit versions for one instance.
    ///
    /// Application readers/writers using this storage must be paused during
    /// application, and pending requests (including cancelled requests) settled.
    /// Keep clients paused after a failure until pending purges have settled. This method excludes local writes, deletes and reads
    /// through NatsPersistence, including its clones and other connections.
    /// It does not exclude raw JetStream access or external clients.
    /// Current active and archived references are retained and verified before
    /// deletion. Any malformed reference or incomplete scan aborts cleanup.
    /// A failed purge can leave a partially applied plan; rerunning is safe.
    pub async fn prune_instance_audits(
        &self,
        instance_id: Uuid,
        dry_run: bool,
    ) -> EngineResult<AuditPruneReport> {
        let _exclusive = AUDIT_ACCESS.write().await;
        let id = instance_id.to_string();
        let mut referenced = HashSet::new();
        for bucket in ["instances", "history_instances"] {
            if let Some(bytes) = self.kv(bucket).await?.get(&id).await.map_err(failure)? {
                let record = self.resolve_snapshot(&bytes).await?;
                let (instance, audit_key) = crate::codec::decode_instance_record(&record)?;
                if instance.id != instance_id {
                    return Err(failure("instance document ID mismatch"));
                }
                if let Some(key) = audit_key {
                    self.restore_instance_record(&record).await?;
                    referenced.insert(key);
                }
            }
        }
        let prefix = format!("{id}.");
        let store = self.kv("instance_audit").await?;
        let mut keys = store.keys().await.map_err(failure)?;
        let mut candidates = Vec::new();
        // Finish and validate the whole scan before deleting anything.
        while let Some(key) = keys.next().await {
            let key = key.map_err(failure)?;
            if key.starts_with(&prefix) && !referenced.contains(&key) {
                candidates.push(key);
            }
        }
        candidates.sort();
        let mut retained: Vec<_> = referenced.into_iter().collect();
        retained.sort();
        let mut report = AuditPruneReport {
            instance_id,
            dry_run,
            retained,
            candidates,
            purged: 0,
        };
        if !dry_run {
            for key in &report.candidates {
                // Invalidate before purge: a lost acknowledgement must not
                // leave a cached key whose underlying snapshot was removed.
                self.audit_cache.lock().await.remove(key);
                store.purge(key).await.map_err(failure)?;
                report.purged += 1;
            }
        }
        Ok(report)
    }
}
