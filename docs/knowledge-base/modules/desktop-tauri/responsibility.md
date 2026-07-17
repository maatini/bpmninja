# desktop-tauri — Responsibilities

## What desktop-tauri Owns

1. **@tag:ui-shell** — Tauri app shell with 7 tabs: Modeler, Instances, Definitions, Tasks, Overview, History, Monitoring.
2. **@tag:bpmn-js-modeler** — bpmn-js canvas with properties panel for BPMN diagram editing and deployment.
3. **@tag:instance-tracking** — Live instance list with token overlay on BPMN diagrams, variable editor with 6 types, history timeline, and token move.
4. **@tag:sse-client** — SSE subscription to `GET /api/events` via a Tauri background task that bridges SSE events to Tauri events for React components.
5. **@tag:monitoring-dashboard** — Storage info, KV bucket browser, engine statistics, live log stream with level/text filters, storage mode badge.
6. **@tag:incident-management** — Incident cards with quick retry, detail dialog with configurable retries, and resolve with variable editor.
7. **@tag:migration-dialog** — Instance migration UI with definition version selector and node mapping.
8. **@tag:tauri-commands** — Rust backend (Tauri commands) that relay HTTP requests to engine-server; thin proxy layer.
9. **@tag:ui-components** — shadcn/ui-based components (dialogs, tabs, accordions, toasts) using Tailwind CSS 4.

## Invariants

1. **Thin client**: All workflow logic lives in engine-server. Desktop app is pure UI + HTTP/SSE.
2. **No direct NATS access**: Desktop communicates only with engine-server via HTTP/SSE.
3. **Push-based updates**: No polling for core state views. SSE events trigger re-fetch via REST.
4. **Configurable API URL**: `ENGINE_API_URL` env var (default: `http://localhost:8081`).
5. **No local state persistence**: Everything is fetched from server; no local storage of engine state.

## Key Frontend Components

| Component | LoC | Purpose |
|-----------|-----|---------|
| `App.tsx` | ~180 | Main layout, tab navigation (7 tabs), timer-start detection |
| `ModelerPage.tsx` | ~350 | bpmn-js modeler with deploy, start & variable dialog |
| `InstancesPage.tsx` | ~245 | Instance list grouped by definition, suspend icon |
| `InstanceDetailDialog.tsx` | ~345 | Instance details with suspend/resume, timer cycle banner, auto-refresh |
| `InstanceViewer.tsx` | ~125 | Read-only BPMN viewer with active node highlighting |
| `HistoryTimeline.tsx` | ~225 | Event table with filters, detail dialog, diff display |
| `DeployedProcessesPage.tsx` | ~330 | Version grouping, accordion, cascade delete |
| `VariableEditor.tsx` | ~480 | Typed editor (6 types including file), upload/download |
| `MonitoringPage.tsx` | ~365 | Metric cards, NATS storage breakdown, KV browser |
| `PendingTasksPage.tsx` | ~290 | User & service task lists with completion dialogs |
| `IncidentsPage.tsx` | ~165 | Incident cards with quick retry, auto-refresh |
| `SettingsPage.tsx` | ~165 | API URL config + connection verify |
| `lib/tauri.ts` | ~170 | Typed Tauri command wrappers (API client layer) |
