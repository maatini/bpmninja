# fuzz — Responsibilities

## What It Owns

9 fuzz targets using cargo-fuzz (libFuzzer) with AddressSanitizer enabled:

| Target | What it fuzzes |
|--------|---------------|
| `fuzz_bpmn_parser` | bpmn-parser with arbitrary UTF-8 strings |
| `fuzz_condition` | `evaluate_condition` with wild expression/variable combos |
| `fuzz_rhai_script` | Rhai engine memory limits and sandboxing |
| `fuzz_iso8601_duration` | ISO 8601 duration parsing |
| `fuzz_token_deserialize` | Deserializing arbitrary JSON as Token, ProcessInstance, HistoryEntry, FileReference |
| `fuzz_cron_expression` | Cron expression parsing via `croner` |
| `fuzz_deploy_roundtrip` | End-to-end: XML → Parse → Deploy → Start Instance |
| `fuzz_history_diff` | History diff calculation with arbitrary structured input |
| `fuzz_server_payloads` | Deserialize arbitrary bytes against all 18 REST API DTOs |

**Invariants:**
- All 9 targets must pass CI (green checkmark)
- AddressSanitizer catches memory leaks, panics, and undefined behavior
- Crash artifacts are uploaded as CI artifacts

**CI:** `.github/workflows/fuzzing.yml` — runs daily and on PRs involving parser, core, or server code.
