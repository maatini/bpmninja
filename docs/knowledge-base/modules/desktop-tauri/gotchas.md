# desktop-tauri — Gotchas

### ⚠️ Thin client — no local engine state

Desktop has NO workflow logic. If the server is down, the desktop app can't do anything meaningful. The settings page allows configuring the API URL, but there's no offline mode.

### ⚠️ SSE connection lifecycle

The Tauri background task (Rust) maintains the SSE connection. If it drops, it reconnects. React components react to Tauri events, not directly to SSE. There's a timing window between SSE reconnect and state sync — the UI briefly shows stale data.

### ⚠️ bpmn-js requires Camunda moddle

The modeler uses `camunda-bpmn-moddle` to support Camunda-specific extensions in the properties panel (execution listeners, topic names, conditions). If this dep is removed, the custom properties panel breaks.

### ⚠️ Custom properties (ConditionPropertiesProvider)

The `ConditionPropertiesProvider.ts` handles flow condition editing with expression and script modes. It specifically suppresses conditions for `Flow_Default` (default flows never have conditions per Camunda 7 compatibility).

### ⚠️ Variable editor supports 6 types

- String, Number, Boolean, JSON, File (upload/download), null
- File variables need NATS Object Store backing — won't work in in-memory mode
- Large JSON values can slow down the editor (no lazy rendering)

### ⚠️ Playwright tests use specific selectors

E2E tests target `.instance-list-item` CSS classes (div-based), NOT `<table>` elements. The `InstancesPage` renders cards, not a table. Changing the component layout breaks tests.

### ⚠️ Dialog accessibility (Radix UI)

Dialog components require `DialogDescription` for accessibility. Some dialogs use `<DialogDescription className="sr-only">` (screen-reader-only). Missing descriptions cause Radix console warnings.

### ⚠️ Tailwind CSS 4 (not 3)

The project uses Tailwind v4 with the `@tailwindcss/vite` plugin. Configuration is in `index.css` via `@theme` directive, NOT in a `tailwind.config.js` file. The `tailwind.config.js` in the repo is legacy/no longer used.

### ⚠️ Tauri plugin paths

File dialogs use `@tauri-apps/plugin-dialog` and `@tauri-apps/plugin-fs`. These are Tauri v2 plugins and require corresponding Rust crate registration in `src-tauri/Cargo.toml`.

### ⚠️ Rerunning the desktop app in dev mode

```bash
devbox run ui:dev
# Or manually:
cd desktop-tauri && npm install && npm run tauri dev
```

This requires a running backend (NATS + engine-server on localhost:8081).
