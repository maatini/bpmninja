//! Fuzz target: BPMN XML parser
//!
//! Feeds arbitrary byte sequences into `parse_bpmn_xml()` to discover panics,
//! buffer overflows, or undefined behavior in the XML deserialization path.
//! The parser must gracefully return `Err` for all malformed inputs — never panic.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Only feed valid UTF-8 to the parser (it expects &str)
    if let Ok(xml) = std::str::from_utf8(data) {
        // We don't care about the result — only that it doesn't panic or crash
        let _ = bpmn_parser::parse_bpmn_xml(xml);
    }
});
