//! Fuzz target: ISO 8601 duration parser (via BPMN XML wrapper)
//!
//! The `parse_iso8601_duration()` function in `bpmn-parser` is `pub(crate)`,
//! so we exercise it indirectly by constructing minimal BPMN XML with a
//! `<timerEventDefinition><timeDuration>` element containing the fuzzed string.
//! This also tests the full timer-definition parsing pipeline.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(duration_str) = std::str::from_utf8(data) {
        // Skip inputs that would break the XML envelope itself
        if duration_str.contains('<') || duration_str.contains('>') || duration_str.contains('&') {
            return;
        }

        // Construct a minimal valid BPMN XML with the fuzzed duration string
        // embedded in a timer start event's timeDuration element.
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
  <process id="fuzz_proc" isExecutable="true">
    <startEvent id="start">
      <timerEventDefinition>
        <timeDuration>{}</timeDuration>
      </timerEventDefinition>
    </startEvent>
    <endEvent id="end"/>
    <sequenceFlow id="f1" sourceRef="start" targetRef="end"/>
  </process>
</definitions>"#,
            duration_str
        );

        // Must never panic — only return Ok or Err
        let _ = bpmn_parser::parse_bpmn_xml(&xml);
    }
});
