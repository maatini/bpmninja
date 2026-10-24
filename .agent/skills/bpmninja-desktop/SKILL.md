---
name: bpmninja-desktop
description: Skill for the desktop-tauri crate covering Tauri backend (Rust), React/TypeScript frontend, bpmn-js integration, and auto-updates.
version: 2.0.0
tags: [rust, tauri, react, typescript, bpmn-js, desktop]
requires: [cargo, node]
---

# BPMNinja Desktop Skill

## When to Activate
Activate whenever you work on the `desktop-tauri` crate:
- Tauri Rust backend (commands, state management, IPC)
- React/TypeScript frontend (components, hooks, features)
- bpmn-js canvas integration and performance
- Auto-update mechanism (Tauri Updater)
- Build configuration (tauri.conf.json, vite.config)

## Scope & File Map

### Tauri Backend (Rust)
```
desktop-tauri/src-tauri/src/
├── main.rs              # Tauri app bootstrap
├── state.rs             # Tauri managed state
├── api_helpers.rs       # API communication helpers
└── commands/            # Tauri invoke commands (Rust → JS bridge)
```

### React Frontend (TypeScript)
```
desktop-tauri/src/
├── main.tsx             # React entry point
├── App.tsx              # Root component, routing
├── index.css            # Global styles
├── vite-env.d.ts        # Vite type declarations
├── components/          # Wiederverwendbare UI-Components (inkl. shadcn/ui)
├── features/            # Feature-Module (Modeler, Instances, History, Tasks …)
├── hooks/               # App-weite Hooks (z. B. use-toast)
├── lib/                 # Utility libraries
└── shared/              # Shared types, components, hooks und tauri.ts
```

### Key Configuration Files
```
desktop-tauri/
├── src-tauri/
│   ├── tauri.conf.json  # Tauri app configuration
│   ├── Cargo.toml       # Rust dependencies
│   └── build.rs         # Tauri build script
├── package.json         # Node dependencies
├── tsconfig.json        # TypeScript configuration
└── vite.config.ts       # Vite bundler configuration
```

## Domain Rules & Patterns

### Tauri Backend
1. **Commands**: All Tauri commands in `commands/` use `#[tauri::command]` macro. Keep them thin – delegate to engine-core.
2. **State**: Use `tauri::State` for shared engine references. Never clone the engine – pass `Arc`.
3. **Error Handling**: Tauri commands must return `Result<T, String>` for the JS bridge. Convert engine errors to descriptive strings.

### React Frontend
1. **Component Memoization**: Wrap components with `React.memo()` when they receive stable props. Essential for bpmn-js performance.
2. **bpmn-js Integration**: The canvas must not re-render on every state change. Use `useRef` for the canvas container, `useCallback` for event handlers.
3. **Tauri Invoke**: Use `@tauri-apps/api/core` for `invoke()`. Type all command inputs/outputs.
4. **CSS**: Tailwind 4 gemäß `TAURI_AGENT.md`; Theme-Farben als HSL-CSS-Variablen. Keine parallele vanilla-CSS-Lösung.
5. **ESLint**: Zero ESLint errors. Run with strict mode.

### Cross-Stack
- Cross-Crate-Reihenfolge aus `CLAUDE.md`: Backend-Rust-Änderungen vor Frontend-React-Änderungen.
- After Rust backend changes: `cd desktop-tauri/src-tauri && cargo clippy --all-targets -- -D warnings`
- After React frontend changes: `npm run build`, bei UI-Flows `npx playwright test`

## Verifikation

```bash
cd desktop-tauri && npm run build
# bei UI-Flows zusätzlich:
npx playwright test
```

## Common Pitfalls
- bpmn-js canvas re-rendering on every React state change (must use refs)
- Tauri commands without proper error conversion (JS gets generic errors)
- Platform-specific path separators or file system assumptions
- Missing `React.memo()` on components that receive callback props
- Forgetting clippy und `npm run build` bei Full-Stack-Änderungen

