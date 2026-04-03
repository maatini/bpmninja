---
trigger: always_on
---

# Cross-Crate Feature Workflow

When a feature spans multiple crates (e.g. a new API endpoint that touches engine-core, engine-server, and desktop-tauri), follow this strict order to avoid broken intermediate states.

## Implementation Order (dependencies first)

```
1. engine-core     → Trait/API/model changes (pure logic, no I/O)
2. bpmn-parser     → XML parsing changes (if new BPMN elements needed)
3. persistence-nats → Persistence changes (if new data to store)
4. engine-server   → REST endpoint changes (HTTP adapter)
5. desktop-tauri/src-tauri → Tauri Command changes (Rust backend)
6. desktop-tauri/src       → React UI changes (TypeScript frontend)
```

## Verification After Each Layer
- After steps 1–4: Run `/verify` (cargo build + clippy + test)
- After steps 5–6: Run `/verify-ui` (npm run build)
- After all steps: Run both `/verify` and `/verify-ui`

## Rules
- Never modify a downstream crate before its upstream dependency is stable.
- If a Rust trait signature changes in `engine-core`, update ALL implementors before proceeding.
- Document any new public API surface in the corresponding SKILL.md file.
