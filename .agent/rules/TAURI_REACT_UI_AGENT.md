---
trigger: file_match
file_patterns: ["desktop-tauri/src/**"]
---

# UI/Desktop Agent (Frontend)
- **Domain:** `desktop-tauri/src/` (React + TypeScript frontend)
- **Role:** Modern desktop UI with **Vanilla CSS** and `bpmn-js` for BPMN diagram rendering.

## Tech Stack (NEVER deviate)
- **Styling:** Vanilla CSS in `index.css` — NO Tailwind, NO shadcn/ui, NO CSS-in-JS
- **Components:** Plain React functional components with hooks
- **Icons:** `lucide-react`
- **BPMN Rendering:** `bpmn-js` (NavigatedViewer for read-only, BpmnModeler for editing)
- **TypeScript:** Strict mode enabled (`useUnknownInCatchVariables: true`)
  - Use `catch { }` or `catch (e: any)` — never bare `catch (e)` 
  - External libs without types (bpmn-js) must be `@ts-ignore`'d and typed as `any`

## Key Files
| File | Purpose |
|---|---|
| `App.tsx` | Main app with sidebar navigation and page routing |
| `Modeler.tsx` | BPMN Modeler (bpmn-js) with deploy/start actions |
| `Instances.tsx` | Instance list + detail view with variable editor |
| `InstanceViewer.tsx` | Read-only BPMN diagram viewer with active node highlighting |
| `HistoryTimeline.tsx` | Compact tabular event history with detail dialog |
| `VariableEditor.tsx` | Reusable typed variable editor (Name/Type/Value table) |
| `DeployedProcesses.tsx` | Definition management (list, delete, view XML) |
| `Monitoring.tsx` | Engine metrics dashboard |
| `Settings.tsx` | Backend switching (in-memory / NATS) |
| `ConditionPropertiesProvider.ts` | bpmn-js properties panel: condition expressions |
| `ScriptPropertiesProvider.ts` | bpmn-js properties panel: Rhai execution listeners |
| `TopicPropertiesProvider.ts` | bpmn-js properties panel: service task topics |
| `ErrorBoundary.tsx` | React error boundary wrapper |
| `lib/tauri.ts` | All Tauri command wrappers (typed API layer) |
| `index.css` | All styles — single stylesheet, no CSS modules |

## CSS Conventions
- Use CSS class selectors (`.card`, `.button`, `.variables-table`)
- CSS custom properties in `:root` for theming (`--primary-color`, `--bg-color`, etc.)
- Inline styles only for dynamic, one-off positioning
- NO `className="flex items-center"` style Tailwind patterns

## Rules
- Do NOT implement business logic in TypeScript — keep it in Rust
- Use Tauri Commands (via `lib/tauri.ts`) for all engine interactions
- Always run `npm run build` (or `/verify-ui`) after changes to catch strict TS errors
- Input fields for code/variables must set `autoCapitalize="off"` and `spellCheck={false}`
