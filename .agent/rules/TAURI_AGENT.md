---
trigger: file_match
file_patterns: ["desktop-tauri/**"]
---

# Desktop-Regeln (Tauri-Backend + React-Frontend)

- **Backend:** `desktop-tauri/src-tauri/` — Brücke zwischen React und `engine-core`, exponiert Tauri-Commands.
- **Frontend:** `desktop-tauri/src/` — React + TypeScript, Tailwind 4, shadcn/ui, bpmn-js.

## Stack
- Tauri 2.12, React 19, bpmn-js 18 (NavigatedViewer lesend, BpmnModeler editierend).
- Tailwind 4 über `@import "tailwindcss"; @config "../tailwind.config.js";`; Theme-Farben als HSL-CSS-Variablen.
- shadcn/ui (Radix) in `components/ui/` — diese Dateien nicht ändern. Icons: `lucide-react`, keine Emojis.
- Font: `Geist Variable` über `@fontsource-variable/geist`.
- TypeScript strict (`useUnknownInCatchVariables`): `catch { }` oder `catch (e: any)`, nie bare `catch (e)`.
  Externe Libs ohne Typen (bpmn-js) mit `@ts-ignore` und `any` typisieren.

## Backend
`AppState { engine: Arc<WorkflowEngine> }`; Engine nutzt intern DashMap, kein externer Mutex.
Start mit optionalem NATS-Auto-Connect, Fallback auf In-Memory, vollständige Wiederherstellung bei Verbindung.
Alle Commands sind async und nehmen `State<AppState>`; Fehler gehen als `Result<T, String>` ans Frontend.

| Command | Engine-Aufruf |
|---|---|
| `deploy_definition` | `deploy_definition()` + `save_bpmn_xml()` |
| `start_instance` | `start_instance()` |
| `list_instances` / `get_instance_details` / `delete_instance` | Instanz-Lifecycle |
| `update_instance_variables` | `update_instance_variables()` |
| `get_pending_tasks` / `complete_task` | User-Tasks |
| `get_pending_service_tasks` / `complete_service_task` | Service-Tasks |
| `list_definitions` / `get_definition_xml` / `delete_definition` | Definitionen und XML |
| `get_backend_info` / `switch_backend` | Persistenz-Status |
| `get_monitoring_data` | `get_stats()` + NATS-Info |
| `get_instance_history` | `query_history()` |

Regeln: Logik bleibt in `engine-core`; neue Commands zusätzlich in `src/shared/lib/tauri.ts` wrappen.

## Frontend
| Datei | Zweck |
|---|---|
| `App.tsx` | Shell mit Sidebar-Navigation |
| `features/modeler/ModelerPage.tsx` | bpmn-js-Modeler mit Deploy/Start |
| `features/instances/` | Instanzliste, Detaildialog, Read-only-Diagramm, Token-Move und Migration |
| `features/history/HistoryPage.tsx` | History-Tabelle und typisierter Variablen-Editor |
| `features/definitions/` | Definitionen und Prozessdetail |
| `features/tasks/` / `features/incidents/` | User- und Service-Tasks, Incidents |
| `features/monitoring/` / `features/settings/` | Metriken, Log-Stream, API-URL und Theme |
| `components/ui/` / `shared/hooks/` | shadcn/ui und Hooks — nicht ändern |
| `shared/lib/tauri.ts` | typisierte Command-Wrapper |

## UI-Konventionen
- Styling über Tailwind-Utilities; bedingte Klassen mit `cn()` aus `lib/utils`.
- Dark Mode über `[data-theme="dark"]`, Default über `prefers-color-scheme`.
- Loading mit `<Skeleton>`, Empty States mit Icon + Titel + Beschreibung.
- Destruktive Aktionen über shadcn `AlertDialog`, nie `window.confirm()`/`window.prompt()`.
- Auto-Refresh: 3 s für Instances/Tasks, 5 s für Monitoring/Incidents.
- Eingabefelder für Code/Variablen mit `autoCapitalize="off"` und `spellCheck={false}`.

## Verifikation
- Nach Frontend-Änderungen: `npm run build` (fängt strict-TS-Fehler) und bei UI-Flows `npx playwright test`.
- Keine Business-Logik in TypeScript; Engine-Interaktion ausschließlich über Tauri-Commands.
