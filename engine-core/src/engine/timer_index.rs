//! Secondary due-time index for pending timers.
//!
//! SSOT remains `pending_timers`. Insert into the map first, then the index.
//! `process_timers` walks due IDs instead of scanning the whole DashMap.

use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::sync::Mutex;
use uuid::Uuid;

use super::WorkflowEngine;
use crate::runtime::PendingTimer;

pub(crate) struct TimerDueIndex {
    inner: Mutex<BTreeMap<(DateTime<Utc>, Uuid), ()>>,
}

impl TimerDueIndex {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(BTreeMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<(DateTime<Utc>, Uuid), ()>> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn insert(&self, id: Uuid, expires_at: DateTime<Utc>) {
        self.lock().insert((expires_at, id), ());
    }

    pub(crate) fn remove(&self, id: Uuid, expires_at: DateTime<Utc>) {
        self.lock().remove(&(expires_at, id));
    }

    /// Timer IDs whose `expires_at` is at or before `now`.
    pub(crate) fn due_ids(&self, now: DateTime<Utc>) -> Vec<Uuid> {
        let end = (now, Uuid::from_u128(u128::MAX));
        self.lock().range(..=end).map(|((_, id), _)| *id).collect()
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, id: Uuid, expires_at: DateTime<Utc>) -> bool {
        self.lock().contains_key(&(expires_at, id))
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.lock().len()
    }
}

impl WorkflowEngine {
    pub(crate) fn insert_pending_timer(&self, timer: PendingTimer) {
        let id = timer.id;
        let expires_at = timer.expires_at;
        self.pending_timers.insert(id, timer);
        self.timer_due_index.insert(id, expires_at);
    }

    pub(crate) fn remove_pending_timer(&self, timer_id: &Uuid) -> Option<(Uuid, PendingTimer)> {
        let removed = self.pending_timers.remove(timer_id);
        if let Some((_, ref timer)) = removed {
            self.timer_due_index.remove(timer.id, timer.expires_at);
        }
        removed
    }

    pub(crate) fn retain_pending_timers(&self, mut keep: impl FnMut(&Uuid, &PendingTimer) -> bool) {
        let mut dropped = Vec::new();
        self.pending_timers.retain(|id, timer| {
            if keep(id, timer) {
                true
            } else {
                dropped.push((timer.id, timer.expires_at));
                false
            }
        });
        for (id, expires_at) in dropped {
            self.timer_due_index.remove(id, expires_at);
        }
    }

    /// Updates `expires_at` on both SSOT and due-index (tests fire timers immediately).
    #[cfg(test)]
    pub(crate) fn set_timer_expiry(&self, timer_id: Uuid, expires_at: DateTime<Utc>) {
        let old = self.pending_timers.get(&timer_id).map(|t| t.expires_at);
        let Some(old) = old else {
            return;
        };
        self.timer_due_index.remove(timer_id, old);
        if let Some(mut timer) = self.pending_timers.get_mut(&timer_id) {
            timer.expires_at = expires_at;
        }
        self.timer_due_index.insert(timer_id, expires_at);
    }
}
