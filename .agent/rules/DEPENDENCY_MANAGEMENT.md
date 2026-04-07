---
trigger: file_match
file_patterns: ["**/Cargo.toml"]
---

# Dependency Management

## Workspace Dependencies (mandatory)
All shared dependencies MUST be declared in the root `Cargo.toml` under `[workspace.dependencies]`. Individual crates reference them via `dependency.workspace = true`.

```toml
# Root Cargo.toml
[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }

# Crate Cargo.toml
[dependencies]
tokio = { workspace = true }
```

## Adding New Dependencies
1. Add the dependency to `[workspace.dependencies]` in root `Cargo.toml` first.
2. Reference it in the crate's `Cargo.toml` with `workspace = true`.
3. Run `/verify` to ensure it compiles across all crates.
4. Exception: Crate-specific dependencies (used by only one crate) may be declared directly, but prefer workspace-level for consistency.

## Feature Flags
- `in-memory`: Used in `persistence-nats` to enable in-memory fallback for tests.
- Keep feature flags minimal — only add them when there is a concrete compile-time switching need.

## Forbidden
- No vendored dependencies.
- No Git dependencies in production (only for temporary prototyping, must be replaced before release).
- No pinning to exact versions (`=1.2.3`) unless there is a documented compatibility issue.
