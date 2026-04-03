---
trigger: file_match
file_patterns: ["bpmn-parser/**"]
---

# Parser Agent
- **Domain:** `bpmn-parser/`
- **Role:** Takes raw XML string/bytes and returns `engine-core::ProcessDefinition`.
- **Rules:** Use `quick-xml` and `serde`. Validate BPMN logic before returning Ok().
