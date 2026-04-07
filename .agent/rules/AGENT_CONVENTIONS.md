---
trigger: always_on
---

# Global Agent Conventions for BPMNinja

1. **Single Responsibility:** Modify only your assigned crate. DO NOT touch other domains. See `PROJECT_CONTEXT.md` for crate assignments.
2. **Traits over Types:** Cross-crate communication only via Rust Traits (e.g., `WorkflowPersistence`). Never import concrete types from other crates.
3. **Zero Temp Files:** Do not use `tmp/` or `temp/` folders. All code must be in well-named modules or tested in-memory.
4. **Agent Handoff:** If a feature requires full-stack changes, follow `CROSS_CRATE_WORKFLOW.md` for the correct implementation order.
