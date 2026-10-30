//! Secondary due-time index for pending timers.
//!
//! SSOT remains `pending_timers`. Insert into the map first, then the index.
//! `process_timers` walks due IDs instead of scanning the whole DashMap.
//! The scheduler sleeps until the next future due time or a schedule change,
//! instead of polling on a fixed interval.

use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use uuid::Uuid;

use super::WorkflowEngine;
use crate::runtime::PendingTimer;

/// Monotonic clock anchored to UTC at engine construction.
///
/// Production due checks use wall time (`Utc::now`). Tests enable the virtual
/// clock so the same checks follow `tokio::time`, which `pause` / `advance`
/// control. The anchor is fixed; virtual time does not track later wall-clock steps.
pub(crate) struct EngineClock {
    base_utc: DateTime<Utc>,
    base_instant: tokio::time::Instant,
    use_monotonic: AtomicBool,
}

impl EngineClock {
    pub(crate) fn new() -> Self {
        Self {
            base_utc: Utc::now(),
            base_instant: tokio::time::Instant::now(),
            use_monotonic: AtomicBool::new(false),
        }
    }

    #[cfg(test)]
    pub(crate) fn enable_virtual(&self) {
        self.use_monotonic.store(true, Ordering::Release);
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        if self.use_monotonic.load(Ordering::Acquire) {
            self.monotonic_now()
        } else {
            Utc::now()
        }
    }

    fn monotonic_now(&self) -> DateTime<Utc> {
        let Ok(delta) = chrono::Duration::from_std(self.base_instant.elapsed()) else {
            return self.base_utc;
        };
        self.base_utc
            .checked_add_signed(delta)
            .unwrap_or(self.base_utc)
    }
}

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

    /// Earliest `expires_at` strictly after `now`.
    ///
    /// Overdue timers are excluded. They are either processed immediately or,
    /// when they stay overdue without firing (suspended instances), must not
    /// schedule a zero-length sleep.
    pub(crate) fn next_due_after(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let after = (now, Uuid::from_u128(u128::MAX));
        self.lock()
            .range((std::ops::Bound::Excluded(after), std::ops::Bound::Unbounded))
            .next()
            .map(|((due, _), _)| *due)
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
    pub(crate) fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Switches due checks and scheduler sleeps onto the monotonic clock.
    ///
    /// Call at the start of a `tokio::time::pause` test, before arming timers
    /// whose deadlines are expressed with [`Self::now`].
    #[cfg(test)]
    pub(crate) fn enable_virtual_clock(&self) {
        self.clock.enable_virtual();
    }

    /// Wakes whoever is inside [`Self::wait_for_timer_work`].
    ///
    /// `notify_waiters` does not store a permit. Callers must register the
    /// waiter before reading the index, so a change between the read and the
    /// sleep is observed, and a change that landed earlier is visible in the index.
    pub(crate) fn notify_timer_schedule_changed(&self) {
        self.timer_notify.notify_waiters();
    }

    pub(crate) fn insert_pending_timer(&self, timer: PendingTimer) {
        let id = timer.id;
        let expires_at = timer.expires_at;
        self.pending_timers.insert(id, timer);
        self.timer_due_index.insert(id, expires_at);
        self.notify_timer_schedule_changed();
    }

    pub(crate) fn remove_pending_timer(&self, timer_id: &Uuid) -> Option<(Uuid, PendingTimer)> {
        let removed = self.pending_timers.remove(timer_id);
        if let Some((_, ref timer)) = removed {
            self.timer_due_index.remove(timer.id, timer.expires_at);
            self.notify_timer_schedule_changed();
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
        let schedule_changed = !dropped.is_empty();
        for (id, expires_at) in dropped {
            self.timer_due_index.remove(id, expires_at);
        }
        if schedule_changed {
            self.notify_timer_schedule_changed();
        }
    }

    /// Waits until a due timer can be processed, the next timer comes due, the
    /// schedule changes, or `max_idle` elapses — whichever comes first.
    ///
    /// The caller should invoke `process_timers` afterwards and call this again.
    /// Dropping the future (shutdown `select`) cancels the sleep and the waiter.
    ///
    /// Overdue timers that cannot fire (suspended instances) do not produce a
    /// zero wait: the sleep runs until the next future deadline or `max_idle`.
    pub async fn wait_for_timer_work(&self, max_idle: Duration) {
        // Register before reading the index so an insert in between is not lost.
        let notified = self.timer_notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();

        let now = self.now();
        if self.has_actionable_due_timer(now).await {
            return;
        }

        let deadline = sleep_deadline(self.timer_due_index.next_due_after(now), now, max_idle);
        tokio::select! {
            _ = &mut notified => {}
            _ = tokio::time::sleep_until(deadline) => {}
        }
    }

    /// `true` when `process_timers` would take at least one timer.
    ///
    /// Suspended instances are skipped there and stay overdue; treating them as
    /// actionable would make the scheduler loop without sleeping.
    async fn has_actionable_due_timer(&self, now: DateTime<Utc>) -> bool {
        for tid in self.timer_due_index.due_ids(now) {
            let instance_id = {
                let Some(timer) = self.pending_timers.get(&tid) else {
                    continue;
                };
                if timer.expires_at > now {
                    continue;
                }
                timer.instance_id
            };
            let suspended = if let Some(inst) = self.instances.get(&instance_id).await {
                let inst = inst.read().await;
                matches!(inst.state, crate::InstanceState::Suspended { .. })
            } else {
                false
            };
            if !suspended {
                return true;
            }
        }
        false
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
        self.notify_timer_schedule_changed();
    }
}

/// Tokio deadline for the next future timer, never later than `max_idle`.
///
/// A non-positive or unrepresentable delta falls back to `max_idle` when the
/// due time is still in the future, so a far-future timestamp cannot spin.
fn sleep_deadline(
    due: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    max_idle: Duration,
) -> tokio::time::Instant {
    let origin = tokio::time::Instant::now();
    let idle_at = origin.checked_add(max_idle).unwrap_or(origin);
    let Some(due) = due else {
        return idle_at;
    };
    if due <= now {
        return origin;
    }
    let Ok(wait) = due.signed_duration_since(now).to_std() else {
        return idle_at;
    };
    origin.checked_add(wait.min(max_idle)).unwrap_or(idle_at)
}
