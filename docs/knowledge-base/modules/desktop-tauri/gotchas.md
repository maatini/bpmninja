# desktop-tauri — Gotchas

### ⚠️ Thin client — no local engine state

Desktop has NO workflow logic. If the server is down, the desktop app can't do anything meaningful. The settings page allows configuring the API URL, but there's no offline mode.

### ⚠️ API-Key headers (`BPMNINJA_API_KEY`)

When the engine requires `BPMNINJA_API_KEY`, the desktop app must send the same key on **all** HTTP calls (REST helpers, direct `reqwest` in commands, file up/download, SSE). Headers:

- `Authorization: Bearer <key>`
- `X-API-Key: <key>`

Empty/unset key → no auth headers (open engine). The key lives in `AppState.api_key` (`Mutex<Option<String>>`), is configured on the Settings page, and is **not** read from the process environment.

### ⚠️ SSE connection lifecycle

The Tauri background task (Rust) maintains the SSE connection. If it drops, it reconnects. React components react to Tauri events, not directly to SSE. There's a timing window between SSE reconnect and state sync — the UI briefly shows stale data.

Die SSE-Base-URL wird bei jedem Connect/Reconnect aus `AppState` gelesen (nicht die Spawn-Zeit-URL). Wechselt `set_api_url` die URL, bricht der Consumer die aktuelle Verbindung ab, setzt den Backoff zurück und verbindet mit der neuen URL. Der API-Key wird ebenfalls aus `AppState` gelesen; `set_api_key` startet den Consumer nicht neu — ein bereits offener Stream behält die Connect-Zeit-Header bis zum nächsten Reconnect (401-Backoff holt einen neu gespeicherten Key).

### ⚠️ bpmn-js requires Camunda moddle

The modeler uses `camunda-bpmn-moddle` to support Camunda-specific extensions in the properties panel (execution listeners, topic names, assignee, conditions). If this dep is removed, the custom properties panel breaks.

Custom providers write both desktop `data-*` attributes and Camunda attributes so the engine parser and Camunda Modeler XML stay interchangeable:

- User Task: `AssigneePropertiesProvider` writes `camunda:assignee` and `data-assignee`
- Service Task: `TopicPropertiesProvider` writes `data-topic` and `camunda:topic`

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

### ⚠️ Identifier, CSP, no dummy updater

Bundle identifier is `dev.bpmninja.desktop` (not the leftover `com.minibpm.dev`). Changing it creates a new macOS/Windows app-data path.

CSP is set in `tauri.conf.json`: `connect-src` allows `http:`/`https:`/`ws:`/`wss:` because the engine URL is user-configurable (LAN). bpmn-js needs `style-src 'unsafe-inline'`, `script-src 'unsafe-eval' 'wasm-unsafe-eval'`, and `worker-src blob:`.

The updater plugin is disabled until a real minisign key exists. The previous dummy pubkey (`A9274242…`) and `releases.minibpm.dev` endpoint are gone.

### ⚠️ Tauri plugin paths

File dialogs use `@tauri-apps/plugin-dialog` and `@tauri-apps/plugin-fs`. These are Tauri v2 plugins and require corresponding Rust crate registration in `src-tauri/Cargo.toml`.

### ⚠️ Rerunning the desktop app in dev mode

```bash
devbox run ui:dev
# Or manually:
cd desktop-tauri && npm install && npm run tauri dev
```

This requires a running backend (NATS + engine-server on localhost:8081).
