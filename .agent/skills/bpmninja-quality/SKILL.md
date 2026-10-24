---
name: bpmninja-quality
description: Cross-cutting skill for testing, CI/CD, documentation, and code quality across the entire BPMNinja workspace.
version: 2.0.0
tags: [testing, ci-cd, documentation, coverage, benchmarks, github-actions]
requires: [cargo]
---

# BPMNinja Quality Skill

## When to Activate
Activate for cross-cutting quality concerns:
- Writing or improving tests (unit, integration, E2E, fuzzing, property-based)
- Modifying CI/CD workflows (`.github/workflows/`)
- Updating documentation (README, architecture docs, API docs)
- Coverage analysis, mutation testing, benchmarks
- Release automation and versioning

## Scope & File Map

### Testing Infrastructure
```
engine-core/src/engine/tests/     # Engine integration tests
engine-core/src/domain/tests.rs   # Domain model tests
bpmn-parser/src/tests.rs          # Parser tests
persistence-nats/src/tests.rs     # Persistence tests (conditional on NATS)
```

### CI/CD Workflows
```
.github/workflows/
├── ci.yml                        # Rust, Client, TypeSpec, Desktop, cargo-deny
├── release.yml                   # Draft → Tauri-Bundles → Publish mit Prüfsummen
├── fuzzing.yml                   # cargo-fuzz
├── mutation-tests.yml            # cargo-mutants und Mutationstrend
├── update-quality-metrics.yml    # Wochenmetriken auf den metrics-Branch
├── docker-publish.yml            # Multi-Arch-Image
└── pages.yml                     # GitHub Pages
```

### Documentation
```
README.md                         # Project overview, badges, metrics
docs/
├── architecture.md               # Architecture overview (Mermaid diagrams)
└── (additional docs)
```

### Configuration
```
Cargo.toml                        # Workspace definition
devbox.json                       # Development environment
docker-compose.yaml               # Local NATS setup
```

## Domain Rules & Patterns

### Testing
1. **Determinism**: All tests must be deterministic. No sleep-based timing, no random seeds without control.
2. **Test Duration**: Individual tests should complete in < 30 seconds (benchmarks excluded).
3. **BPMN Compliance Tests**: Test BPMN execution semantics, not just data structures. Verify token flow, gateway routing, event triggers.
4. **Mutation Testing**: Use `cargo-mutants` to validate test effectiveness. Track mutation score.
5. **Conditional Tests**: NATS-dependent tests must be skipped when NATS is unavailable (use `#[ignore]` or runtime checks).

### CI/CD
1. **Workflow Stability**: All CI jobs must be deterministic and cache-optimized.
2. **Build Matrix**: Test on Linux, macOS, and Windows.
3. **Semantic Versioning**: Follow semver. Breaking changes require major version bump.
4. **Caching**: Use `Swatinem/rust-cache` for Cargo build caches in GitHub Actions.

### Documentation
1. **Accuracy**: All metrics in README must match actual test output. Never fabricate numbers.
2. **Architecture Diagrams**: Use Mermaid syntax. Keep diagrams in sync with actual module structure.
3. **API Documentation**: Use `cargo doc` comments (`///`) for all public types and functions.

## Verifikation

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Common Pitfalls
- Tests without meaningful assertions on BPMN semantics (just checking "no panic")
- Flaky tests caused by uncontrolled async timing
- Fabricated coverage or mutation scores in documentation
- CI workflows that work on Linux but fail on Windows (path separators, line endings)
- Adding test dependencies that bloat compile time

