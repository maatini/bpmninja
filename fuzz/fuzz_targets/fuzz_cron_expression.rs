//! Fuzz target: Cron expression parsing via croner.
//!
//! BPMN timers accept user-provided cron expressions in `timeCycle` definitions.
//! The croner library must handle arbitrary strings gracefully without panics
//! or excessive CPU usage.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only process reasonable-length, valid UTF-8 strings
    let input = match std::str::from_utf8(data) {
        Ok(s) if !s.is_empty() && s.len() <= 1024 => s,
        _ => return,
    };

    // Parse the cron expression — must not panic
    if let Ok(cron) = input.parse::<croner::Cron>() {
        // If parsing succeeds, find_next_occurrence must also not panic
        let now = chrono::Utc::now();
        let _ = cron.find_next_occurrence(&now, false);

        // Also test is_time_matching — another potential panic source
        let _ = cron.is_time_matching(&now);
    }
});
