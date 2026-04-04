---
name: tauri-frontend
description: Skill for the Tauri desktop application — React + Vanilla CSS frontend with bpmn-js modeler and dual-mode backend (embedded/HTTP).
version: 2.0
triggers: ["tauri", "desktop", "ui", "frontend", "react", "bpmn-js"]
author: Maatini
tags: [tauri, react, typescript, vanilla-css, bpmn-js]
---

# TAURI FRONTEND SKILL

## Crate: `desktop-tauri`
Tauri v1 desktop application with React + Vanilla CSS + bpmn-js.

## Dual Backend Modes
- **Embedded (default):** `WorkflowEngine` runs inside Tauri backend (`main.rs`)
- **HTTP mode:** `--features http-backend` connects to `engine-server` via REST API

## Architecture
```
desktop-tauri/
├── src-tauri/src/main.rs    # Tauri backend (all commands, state management)
├── src/                      # React frontend
│   ├── App.tsx               # Main app with sidebar navigation
│   ├── Modeler.tsx           # BPMN Modeler (bpmn-js) with deploy/start
│   ├── Instances.tsx         # Instance list + detail + variable editor
│   ├── InstanceViewer.tsx    # Read-only BPMN viewer with node highlighting
│   ├── HistoryTimeline.tsx   # Compact tabular history with detail dialog
│   ├── VariableEditor.tsx    # Reusable typed variable editor table
│   ├── DeployedProcesses.tsx # Definition management
│   ├── Monitoring.tsx        # Engine metrics dashboard
│   ├── Settings.tsx          # Backend switching
│   ├── lib/tauri.ts          # All Tauri command wrappers (typed API)
│   └── index.css             # All styles (Vanilla CSS, no Tailwind!)
├── tests/e2e/                # Playwright E2E tests
└── package.json
```

## Tauri Commands (exposed to frontend)
| Command | Purpose |
|---|---|
| `deploy_definition(xml, name)` | Deploy BPMN XML |
| `deploy_simple_process()` | Deploy hardcoded simple process |
| `start_instance(def_id, variables)` | Start new process instance |
| `list_instances()` | List all process instances |
| `get_instance_details(id)` | Get single instance details |
| `update_instance_variables(id, vars)` | Update instance variables |
| `delete_instance(id)` | Delete a process instance |
| `get_pending_tasks()` | List pending user tasks |
| `complete_task(task_id)` | Complete a user task |
| `list_definitions()` | List deployed definitions |
| `get_definition_xml(id)` | Get original BPMN XML |
| `delete_definition(id, cascade)` | Delete a definition |
| `get_backend_info()` | Get current backend type |
| `switch_backend(type, url)` | Switch between in-memory/NATS |
| `get_monitoring_data()` | Engine + NATS metrics |
| `read_bpmn_file(path)` | Read BPMN file from disk |
| `get_instance_history(id, query)` | Query instance history |
| `get_pending_service_tasks()` | List pending service tasks |
| `complete_service_task(id, worker, vars)` | Complete a service task |

## State Management
- `AppState` with `Arc<Mutex<WorkflowEngine>>` for embedded mode
- BPMN XML cached in-memory + NATS Object Store
- NATS auto-connect on startup (graceful fallback to in-memory)
- Full state restore from NATS on startup

## Styling
- **TailwindCSS 3** with `tailwind.config.js` for theme configuration
- **shadcn/ui** components based on Radix UI primitives (in `components/ui/`)
- CSS custom properties (HSL format) for light/dark theming in `src/index.css`
- Lucide React icons for all iconography
- `Geist Variable` font via `@fontsource-variable/geist`

## TypeScript Strict Mode
- `useUnknownInCatchVariables: true` — never use bare `catch (e)`
- Use `catch { }` for ignored errors or `catch (e: any)` when error is used
- External libs without types (bpmn-js) must use `@ts-ignore` + `any` typing

## Rules
- Do NOT implement business logic in TypeScript — keep it in Rust
- Use Tauri Commands for all engine interactions
- All `#[cfg(feature = "http-backend")]` variants must mirror embedded commands
