---
trigger: always_on
---

# RUST_AGENT_RULES.md - Rust Skills & Best Practices for AI Agents

## Must-Follow Rust Rules
- Use Rust 2024 edition.
- Prefer `Result<T, E>` + `?` operator.
- Custom errors with `thiserror`.
- Async: Use `tokio` with `#[tokio::main]`.
- For timers: `tokio::time::sleep` or `tokio::time::interval`.
- State machines: Use enums for task types and process states.
- Concurrency: Use channels (tokio::sync::mpsc) and Arc<Mutex<>> or RwLock carefully.
- Testing: Always write #[test] and #[tokio::test]. Aim for >85% coverage on core logic.
- Clippy: Fix all warnings + pedantic where reasonable.
- Naming: snake_case, descriptive. Modules in separate files.

## Anti-Patterns (NEVER do)
- No .unwrap() in production paths.
- No giant main.rs files – use proper module structure (src/engine/, src/model/, src/tasks/).
- No shared mutable state without synchronization.
- No ignoring compiler warnings.

Follow these for maximum agent performance.
