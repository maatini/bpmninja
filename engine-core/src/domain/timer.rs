use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Represents a BPMN 2.0 timer definition.
///
/// Supports all three BPMN timer types:
/// - `timeDuration` → Duration variant
/// - `timeDate` → AbsoluteDate variant  
/// - `timeCycle` → CronCycle or RepeatingInterval variant
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimerDefinition {
    /// ISO 8601 Duration (e.g. PT30S, P1DT2H, P1M).
    /// Converted to std::time::Duration at parse time.
    Duration(Duration),

    /// ISO 8601 absolute date-time (e.g. 2026-04-06T14:30:00Z).
    AbsoluteDate(DateTime<Utc>),

    /// Cron expression for recurring execution (e.g. "0 9 * * MON-FRI").
    CronCycle {
        expression: String,
        /// Optional maximum number of repetitions (None = infinite).
        max_repetitions: Option<u32>,
    },

    /// ISO 8601 repeating interval (e.g. R3/PT10M = repeat 3x every 10min).
    RepeatingInterval {
        /// Number of repetitions (None = infinite for R/...).
        repetitions: Option<u32>,
        /// Interval duration per cycle.
        interval: Duration,
    },
}

impl TimerDefinition {
    /// Calculates the next expiry DateTime from `now`.
    ///
    /// - Duration: now + duration
    /// - AbsoluteDate: the fixed date itself
    /// - CronCycle: next occurrence from cron schedule
    /// - RepeatingInterval: now + interval
    pub fn next_expiry(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            TimerDefinition::Duration(dur) => {
                let chrono_dur = chrono::Duration::from_std(*dur).ok()?;
                Some(now + chrono_dur)
            }
            TimerDefinition::AbsoluteDate(dt) => {
                if *dt > now {
                    Some(*dt)
                } else {
                    None
                }
            }
            TimerDefinition::CronCycle { expression, .. } => {
                // Uses croner for next occurrence
                let cron = expression.parse::<croner::Cron>().ok()?;
                cron.find_next_occurrence(&now, false).ok()
            }
            TimerDefinition::RepeatingInterval { interval, .. } => {
                let chrono_dur = chrono::Duration::from_std(*interval).ok()?;
                Some(now + chrono_dur)
            }
        }
    }

    /// Returns true if this timer can fire more than once.
    pub fn is_recurring(&self) -> bool {
        matches!(
            self,
            TimerDefinition::CronCycle { .. } | TimerDefinition::RepeatingInterval { .. }
        )
    }

    /// Remaining repetitions when this timer is first scheduled (`None` = infinite / not repeating).
    pub fn initial_remaining_repetitions(&self) -> Option<u32> {
        match self {
            TimerDefinition::RepeatingInterval { repetitions, .. } => *repetitions,
            TimerDefinition::CronCycle {
                max_repetitions, ..
            } => *max_repetitions,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn next_expiry_duration_adds_to_now() {
        let now = Utc::now();
        let td = TimerDefinition::Duration(Duration::from_secs(60));
        let expiry = td.next_expiry(now).unwrap();
        // Must be now + 60s, not now - 60s (catches replace + with -)
        assert!(expiry > now);
        assert!((expiry - now).num_seconds() == 60);
    }

    #[test]
    fn next_expiry_absolute_date_future_returns_some() {
        let now = Utc::now();
        let future = now + chrono::Duration::hours(1);
        let td = TimerDefinition::AbsoluteDate(future);
        assert_eq!(td.next_expiry(now), Some(future));
    }

    #[test]
    fn next_expiry_absolute_date_past_returns_none() {
        let now = Utc::now();
        let past = now - chrono::Duration::hours(1);
        let td = TimerDefinition::AbsoluteDate(past);
        // Past dates must return None (catches replace > with <, >=, ==)
        assert_eq!(td.next_expiry(now), None);
    }

    #[test]
    fn next_expiry_absolute_date_exact_now_returns_none() {
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let td = TimerDefinition::AbsoluteDate(now);
        // dt == now → *dt > now is false → None (catches > with >=)
        assert_eq!(td.next_expiry(now), None);
    }

    #[test]
    fn next_expiry_repeating_interval() {
        let now = Utc::now();
        let td = TimerDefinition::RepeatingInterval {
            repetitions: Some(3),
            interval: Duration::from_secs(300),
        };
        let expiry = td.next_expiry(now).unwrap();
        assert!(expiry > now);
        assert!((expiry - now).num_seconds() == 300);
    }

    #[test]
    fn is_recurring_duration_false() {
        let td = TimerDefinition::Duration(Duration::from_secs(10));
        assert!(!td.is_recurring());
    }

    #[test]
    fn is_recurring_absolute_false() {
        let td = TimerDefinition::AbsoluteDate(Utc::now());
        assert!(!td.is_recurring());
    }

    #[test]
    fn is_recurring_cron_true() {
        let td = TimerDefinition::CronCycle {
            expression: "0 9 * * *".into(),
            max_repetitions: None,
        };
        // Catches: replace is_recurring -> bool with false
        assert!(td.is_recurring());
    }

    #[test]
    fn is_recurring_repeating_interval_true() {
        let td = TimerDefinition::RepeatingInterval {
            repetitions: Some(5),
            interval: Duration::from_secs(60),
        };
        assert!(td.is_recurring());
    }

    #[test]
    fn initial_remaining_repeating_interval_some() {
        let td = TimerDefinition::RepeatingInterval {
            repetitions: Some(3),
            interval: Duration::from_secs(1),
        };
        assert_eq!(td.initial_remaining_repetitions(), Some(3));
    }

    #[test]
    fn initial_remaining_repeating_interval_infinite() {
        let td = TimerDefinition::RepeatingInterval {
            repetitions: None,
            interval: Duration::from_secs(1),
        };
        assert_eq!(td.initial_remaining_repetitions(), None);
    }

    #[test]
    fn initial_remaining_cron_max() {
        let td = TimerDefinition::CronCycle {
            expression: "0 9 * * *".into(),
            max_repetitions: Some(5),
        };
        assert_eq!(td.initial_remaining_repetitions(), Some(5));
    }

    #[test]
    fn initial_remaining_cron_infinite() {
        let td = TimerDefinition::CronCycle {
            expression: "0 9 * * *".into(),
            max_repetitions: None,
        };
        assert_eq!(td.initial_remaining_repetitions(), None);
    }

    #[test]
    fn initial_remaining_duration_none() {
        let td = TimerDefinition::Duration(Duration::from_secs(10));
        assert_eq!(td.initial_remaining_repetitions(), None);
    }

    #[test]
    fn initial_remaining_absolute_date_none() {
        let td = TimerDefinition::AbsoluteDate(Utc::now());
        assert_eq!(td.initial_remaining_repetitions(), None);
    }
}
