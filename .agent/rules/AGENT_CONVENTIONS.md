# Global Agent Conventions for mini-bpm
1. **Single Responsibility:** Modify only your assigned crate. DO NOT touch other domains.
2. **Traits over Types:** Cross-crate communication only via Rust Traits (e.g., `PersistenceProvider`).
3. **Zero Temp Files:** Do not use `tmp/` or `temp/` folders. All code must be in well-named modules or tested in-memory.
4. **Agent Handoff:** If a feature requires full-stack changes, document current progress and call the next Agent via Orchestrator Rules.
