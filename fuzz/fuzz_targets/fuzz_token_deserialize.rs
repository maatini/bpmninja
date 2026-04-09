//! Fuzz target: Token + ProcessInstance + HistoryEntry deserialization.
//!
//! These structs are deserialized from external sources (NATS KV, HTTP API).
//! Malformed JSON payloads must never cause panics — only clean `Err` returns.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Guard: only process data that looks vaguely like JSON
    if data.is_empty() || data.len() > 65536 {
        return;
    }

    // Attempt to deserialize as Token — must not panic
    let _ = serde_json::from_slice::<engine_core::domain::Token>(data);

    // Attempt to deserialize as ProcessInstance — must not panic
    let _ = serde_json::from_slice::<engine_core::runtime::ProcessInstance>(data);

    // Attempt to deserialize as HistoryEntry — must not panic
    let _ = serde_json::from_slice::<engine_core::history::HistoryEntry>(data);

    // Attempt to deserialize as FileReference — must not panic
    let _ = serde_json::from_slice::<engine_core::domain::FileReference>(data);

    // Roundtrip: if parsing succeeds, re-serialization must also succeed
    if let Ok(token) = serde_json::from_slice::<engine_core::domain::Token>(data) {
        let _ = serde_json::to_string(&token).expect("re-serialization must not fail");
    }
});
