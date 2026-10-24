# DX- & Produkt-Review BPMNinja — Fokus „all"

Geprüft wurden REST-Handler (`engine-server/src/server/*`), `docs/openapi.yaml`, der TypeScript-Client (`bpmn-ninja-external-task-client/`) und die Desktop-UI (`desktop-tauri/src/**`). Die acht Funde aus der Repo-Analyse (F1–F8) sind bereits adressiert und werden hier nicht wiederholt; alle Vorschläge sind inkrementell.

---

### 1 — `POST /api/start/latest` widerspricht der eigenen Spezifikation — **hoch**
**Oberfläche:** REST-Route, Onboarding.
**Evidenz:** `docs/openapi.yaml:189-193` und `README.md:302` verlangen `bpmn_process_id`; der Handler liest `bpmn_id` (`engine-server/src/server/instances.rs:43-47`). Zudem dokumentiert die Spec `StartResponse` (`openapi.yaml:202`), der Handler liefert zusätzlich `definition_key` und `version` (`instances.rs:50-54,70-74`), und einen fehlenden Prozess quittiert er mit 400 (`:67`), nicht mit dem dokumentierten 404.
**Wirkung:** Der in README und Spec gezeigte erste Integrationsschritt endet in einem Axum-422 ohne erklärende Meldung. Generierte Clients erhalten das Feld `version` nicht.
**Vorschlag:** Handler auf `bpmn_process_id` umstellen (mit `#[serde(alias = "bpmn_id")]` für eine Übergangsversion), `StartLatestResponse` in der Spec ergänzen, 400/404 präzisieren. Aufwand: S.

### 2 — TypeScript-Client bleibt bei falscher Konfiguration stumm „running" — **hoch**
**Oberfläche:** `ExternalTaskClient`.
**Evidenz:** `ExternalTaskClient.ts:256-266` fängt jeden Poll-Fehler, loggt und schläft `pollingInterval` (Default 300 ms, `:41`) — ohne Backoff, ohne Zähler, ohne Callback. `start()` gibt `void` zurück (`:159-179`) und prüft nur, ob Subscriptions existieren (`:165-167`). Fehlertexte werden als `fetchAndLock failed: ${status} ${text}` geworfen (`:305-308`).
**Wirkung:** Falsche `baseUrl`, abgelaufener API-Key oder 401 erzeugen eine endlose, lautlose Fehlerschleife mit 3 Requests/s gegen den Server. Der Worker „läuft", verarbeitet aber nichts — der klassische Produktionsausfall ohne Alert.
**Vorschlag:** `requestTimeoutMs` (Default 30 s, `AbortSignal.timeout`), exponentieller Backoff bis 30 s, `onPollError`-Hook plus `lastPollError`/`isHealthy`, Fail-fast nach 3 aufeinanderfolgenden 401/403/404. Aufwand: S.

### 3 — Persistenzfehler werden als Client-Fehler (400) ausgegeben; Fehlerobjekt ohne Code — **hoch**
**Oberfläche:** REST + Desktop-Fehleranzeige.
**Evidenz:** `engine-server/src/server/history.rs:52,74,122` mappen jeden Speicherfehler auf `AppError::BadRequest` → 400. Ein nicht gefundener History-Eintrag liefert 400 (`:78-80`) statt der dokumentierten 404 (`openapi.yaml:814`); „No persistence configured" ebenfalls 400 (`:82-85`). Alle Fehler haben die Form `{"error": "<Text>"}` (`state.rs:89`).
**Wirkung:** Ein NATS-Ausfall erscheint im Monitoring als Client-Fehler und rutscht an 5xx-Alerts vorbei; UI und Integratoren können „retry" nicht von „dein Request ist falsch" unterscheiden. Der Desktop-Helper parst genau dieses eine Feld (`desktop-tauri/src-tauri/src/api_helpers.rs:34-43`).
**Vorschlag:** Persistenzfehler → 503, fehlender Eintrag → 404; Fehlerbody um `code` (z. B. `HISTORY_UNAVAILABLE`) ergänzen. Aufwand: S.

### 4 — Statuscodes und Pflicht-Bodies in der Spec weichen von den Handlern ab — **mittel**
**Oberfläche:** OpenAPI, generierte Clients, `curl`-Einstieg.
**Evidenz:** `complete`, `failure`, `extendLock`, `bpmnError` sind mit 200 dokumentiert (`openapi.yaml:464,485,501,520`), die Handler antworten mit 204 (`engine-server/src/server/tasks.rs:130,160,224,246`). `POST /api/service-task/{id}/retry` ist ohne `requestBody` beschrieben (`openapi.yaml:530-537`), der Handler erzwingt `Json<RetryIncidentRequest>` (`tasks.rs:169-177`) — ein bodyloser Aufruf scheitert mit 415. Fehlerantworten für 404/409 fehlen bei `failure`/`extendLock`/`bpmnError` ganz.
**Wirkung:** Der TypeScript-Client prüft nur `res.ok` und überlebt das; jeder generierte Client, jedes Contract-Test-Setup und jeder manuelle Aufruf nach Doku bricht.
**Vorschlag:** Handler auf 200 mit Body oder Spec auf 204 ziehen, `RetryIncidentRequest` dokumentieren (alternativ `#[serde(default)]` akzeptieren), 404/409 je Route nachtragen. Aufwand: S.

### 5 — Desktop zeigt `CompletedWithError` und `ParallelExecution` als harmlose Neutralzustände — **mittel-hoch**
**Oberfläche:** Instances-Liste, Instance-Header, Overview.
**Evidenz:** Die Engine kennt elf Zustände (`engine-core/src/runtime/instance.rs:63-97`), das UI-Typing nur sechs (`desktop-tauri/src/shared/types/engine.ts:23-28`). `stateLabel`/`stateBadgeClass` haben keinen Zweig für `CompletedWithError`; der Fallback erzeugt „Completed With Error" mit grauem Standard-Badge (`InstanceStateUtils.ts:14,31`). `ParallelExecution { active_token_count }` verliert die Tokenzahl.
**Wirkung:** Fehlgeschlagene Prozesse sehen in der Liste wie erledigt aus — der Nutzer findet sie nur über den separaten History-Filter `state=error` (`engine.ts:41`). Das ist der sichtbarste Zustandskonsistenz-Bruch der UI.
**Vorschlag:** `switch` über die vollständige Union mit `never`-Guard, `CompletedWithError` → `destructive`-Badge plus `error_code` als Tooltip, `ParallelExecution` mit Tokenzahl, `Suspended` mit `previous_state`. Aufwand: S.

### 6 — History-Filter werden serverseitig stillschweigend verworfen — **mittel**
**Oberfläche:** History-Tab, REST.
**Evidenz:** `history.rs:28-36` parst `event_types`/`actor_type` mit `filter_map(.. .ok())` bzw. `.ok()`; ungültige Werte werden zu `None`, also „kein Filter". Die UI sendet Kleinschreibung `engine`/`serviceworker` (`HistoryTimeline.tsx:112-115`), die Spec listet PascalCase `Engine`/`ServiceWorker` (`openapi.yaml:766`). `from`/`to`/`limit` bietet die UI gar nicht an (`HistoryTimeline.tsx:34-36`).
**Wirkung:** Der Nutzer filtert und bekommt entweder alles oder eine leere Liste mit „No history entries found for the current filters" (`:122-124`) — ohne Hinweis, dass sein Filterwert unbekannt war. Jede Filteränderung lädt zudem die komplette Historie neu (`:47-50`).
**Vorschlag:** Strikte Validierung → 400 mit Liste gültiger Werte, case-insensitive Parsing, UI um Zeitraum und `limit` erweitern. Aufwand: S.

### 7 — Pagination ist uneinheitlich und wird von der Desktop-App nicht genutzt — **mittel**
**Oberfläche:** `/api/instances`, `/api/tasks`, `/api/service-tasks`, Instances-Page.
**Evidenz:** `/api/instances` liefert ohne `limit`/`offset` ein unbegrenztes Array und `X-Total-Count` nur im paginierten Zweig (`instances.rs:102-119`); `limit` wird still auf 1000 geklemmt (`:110`). `/api/tasks` und `/api/service-tasks` haben gar keine Paging-Parameter (`tasks.rs:19-31`). Der Tauri-Befehl ruft `/api/instances` ohne Query (`desktop-tauri/src-tauri/src/commands/instances.rs:44-49`) und wird per `setInterval` gepollt (`shared/hooks/use-polling.ts:25-27`).
**Wirkung:** Bei einigen tausend Instanzen serialisiert und überträgt die UI diese Liste bei jedem Poll-Zyklus vollständig — Ruckeln und unnötige Last, mit wachsendem Effekt.
**Vorschlag:** Default-Limit 200 + `X-Total-Count` immer setzen, `limit`/`offset` auch für `tasks`/`service-tasks`, Tauri-Aufruf mit Limit und Nachladen, Liste über `useEngineEvents` deltäisch aktualisieren. Aufwand: M.

### 8 — Tauri-Fehlkanal ist ein untypisierter String, Toasts zeigen Pfad und Status — **niedrig-mittel**
**Oberfläche:** Alle Desktop-Seiten, z. B. Incidents.
**Evidenz:** `api_helpers.rs:41,57-60,81,101,121` bauen `Request failed: {status} {path}: {message}`; alle Commands geben `Result<_, String>` zurück (`commands/instances.rs:52-57`); die Seite hängt das an den Toast: `'Failed to load incidents: ' + e` (`IncidentsPage.tsx:28-30`).
**Wirkung:** Der Nutzer sieht interne API-Pfade statt einer Handlungsoption; 401 (Key falsch), 503 (Engine weg) und 404 sehen identisch aus. Vorhandene Bausteine wie `EngineOfflineBanner` und `EmptyState` werden nicht angesteuert.
**Vorschlag:** Serialisiertes `ApiError { kind, status, path, message, hint }`, gemeinsame Fehlerkomponente mit Next Action (401 → Settings/API-Key, 5xx → Retry, Netzwerk → Offline-Banner). Aufwand: S–M.

---

**Priorisierung für den nächsten Sprint:** Fund 1 und 4 (Contract-Wahrheit, je ~1 h), Fund 2 (Worker-Sichtbarkeit), Fund 3 (Fehler-Taxonomie) — danach 5 und 6 als reine UI-/Validierungsarbeiten.