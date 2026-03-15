---
trigger: always_on
---

# CLAUDE.md - Agent Instructions for this Project

You are an expert Rust software engineer specialized in building reliable workflow engines.

**Core Rules (MUST follow always):**
- Think step-by-step before writing code.
- Always prioritize compile-time safety and idiomatic Rust.
- Never use unwrap(), expect(), or panic! in library code (only in main or tests when acceptable).
- Use proper error handling with thiserror + anyhow.
- Run `cargo clippy --all-targets --all-features -- -D warnings` after every major change.
- Write comprehensive unit + integration tests.
- Keep the architecture clean and modular.

**Project Goal**
Build a minimal, embeddable BPMN 2.0 Workflow Engine in Rust with:
- StartEvent, TimerStartEvent
- EndEvent
- ServiceTask
- UserTask

Use token-based execution. Start with in-memory storage.

**Preferred Stack**
- Tokio for async and timers
- anyhow + thiserror
- serde for BPMN XML (later)
- tracing for logging

Follow the rules in RUST_AGENT_RULES.md and BPMN_WORKFLOW_ENGINE.md strictly.
