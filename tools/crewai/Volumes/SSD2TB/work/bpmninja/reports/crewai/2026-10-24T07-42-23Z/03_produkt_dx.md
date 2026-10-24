# DX- & Produktoberflächen-Review BPMNinja (Fokus „all“, v0.7.24)

Geprüft: `engine-server/src/server/*`, `docs/openapi.yaml`, `bpmn-ninja-external-task-client/src/*`, `desktop-tauri/src/**`, `README.md`, `docs/knowledge-base/**`. Acht priorisierte Funde, jeweils mit Oberfläche, Evidenz, Wirkung und inkrementellem Pfad.

---

## 1 — `/api/start/latest` widerspricht der eigenen Spezifikation (hoch)

**Oberfläche:** `POST /api/start/latest` — REST, OpenAPI, README.
**Evidenz:** Handler erwartet `bpmn_id` (`engine-server/src/server/instances.rs:42-47`, `StartLatestRequest { pub bpmn_id: String }`, kein `serde(rename)`), OpenAPI fordert `required: [bpmn_process_id]` (`docs/openapi.yaml:189-193`), README ebenso (`README.md:302`). Zusätzlich ist `business_key` beim Start nirgends dokumentiert — der Handler liest ihn aus den *Variablen* (`engine-core/src/engine/process_start.rs:72-75`), und die Desktop-UI nutzt genau diesen Trick (`StartInstanceDialog.tsx:39-41`).
**Wirkung:** Wer der Spec folgt (Redoc-Beispiel, generierter Client, Postman-Collection), bekommt statt eines Starts eine Axum-Deserialisierungs-Rejection. Der Business-Key-Start funktioniert nur, wenn man den UI-Code liest — er ist Teil des öffentlichen Vertrags, aber unsichtbar.
**Vorschlag:** Handler auf `bpmn_process_id` umstellen (oder `#[serde(alias)]` für beide) und `business_key` als optionales Top-Level-Feld in `StartRequest`/`StartLatestRequest` aufnehmen, mit Rückwärtskompatibilität über die Variable. Zusätzlich ein Contract-Test in `engine-server/tests/`, der Spec-Beispiele wörtlich gegen den Router schickt.

## 2 — Erfolgs-Statuscodes: Spec sagt 200, Server liefert 204 (hoch)

**Oberfläche:** OpenAPI/README vs. sechs Handler.
**Evidenz:** Dokumentiert `200`: `/api/complete/{id}` (`openapi.yaml:404`), `/api/service-task/{id}/complete` (`:464`), `/failure` (`:485`), `/extendLock` (`:501`), `/bpmnError` (`:520`), `/api/instances/{id}/variables` (`:570`), `DELETE /api/instances/{id}` (`:287`). Tatsächlich `StatusCode::NO_CONTENT`: `tasks.rs:44,130,160,224,246`, `instances.rs:151,239`.
**Wirkung:** Generierte SDKs und handgeschriebene Clients, die auf `status === 200` prüfen (häufiges Muster), werfen nach erfolgreicher Aktion. Redoc-Beispiele sind falsch. Der Fehler ist systematisch, nicht punktuell.
**Vorschlag:** Spezifikation auf `204` korrigieren (einmalige Doku-Änderung, kein Code-Risiko) und einen Test ergänzen, der für jede Route Status + Content-Type gegen eine kleine Erwartungstabelle prüft — damit Drift künftig im CI auffällt.

## 3 — Fehlerobjekt ist nicht uniform, obwohl README es zusichert (hoch)

**Oberfläche:** alle JSON-Routen; `AppError` deckt nur den Handler-Pfad ab.
**Evidenz:** `README.md:378-391` verspricht `{ "error": … }` für alle Fehler und listet nur 400/404/409/500. Tatsächlich laufen Body-/Pfad-/Query-Fehler an `AppError` vorbei: Handler nehmen `Json(payload): Json<T>` direkt (`tasks.rs:65`, `instances.rs:28`), Axum liefert `JsonRejection` als Plaintext mit 400/415/422; `DefaultBodyLimit::max` (`mod.rs:258`) liefert Plaintext-413, das in `openapi.yaml:54-80` gar nicht existiert.
**Wirkung:** Der dokumentierte Vertrag bricht genau dann, wenn Integratoren ihn am dringendsten brauchen (kaputter Request). Clients, die `body.error` lesen (`desktop-tauri/src-tauri/src/api_helpers.rs:37-40`), fallen auf Rohtext zurück und zeigen dem Nutzer „Request failed: 422 …“ statt einer Ursache.
**Vorschlag:** `AppError` um `Json(JsonRejection)`/`LengthLimit` erweitern, per `From`-Impl automatisch in `{error, code}` mappen; 413 und 422 in README und OpenAPI ergänzen. Kein Redesign, nur ein `impl From`.

## 4 — History-Filter der Desktop-UI filtern nichts (hoch)

**Oberfläche:** Instanz-Detail → Tab „History“, Filter „Lifecycle Only“ / „Variable Changes“ / „Service Worker“.
**Evidenz:** UI sendet `actor_types` (`HistoryTimeline.tsx:36`, `shared/lib/tauri.ts:74`) und baut den Query in `desktop-tauri/src-tauri/src/commands/instances.rs:63-77`; der Server liest aber `actor_type` (Singular, `engine-server/src/server/history.rs:14`) — unbekannte Query-Felder werden von Serde still verworfen. Selbst mit richtigem Namen scheitern die Werte: UI bietet `engine`/`serviceworker`/`api` (`HistoryTimeline.tsx:112-115`), das Enum kennt `Engine`/`ServiceWorker`/`Listener` (`engine-core/src/history/mod.rs:9-15`), und `history.rs:34-36` verwirft Parse-Fehler per `.ok()`. Die Event-Filter `InstanceFailed` und `VariablesChanged` (`HistoryTimeline.tsx:101,103`) existieren nicht — das Enum hat `Error` und `VariableUpdated` (`history/mod.rs:18-39`).
**Wirkung:** Der Nutzer wählt einen Filter, die Liste bleibt vollständig — ohne Hinweis. Das ist der Unterschied zwischen „Feature kaputt“ und „Feature unsichtbar kaputt“, und er kostet Vertrauen in die gesamte History-Ansicht.
**Vorschlag:** UI-Strings an die Enum-Namen binden (aus der OpenAPI-Enum-Liste generiert oder als gemeinsame Konstante), Parameternamen auf `actor_type` angleichen; serverseitig ungültige Filterwerte mit 400 + `{error}` ablehnen statt `.ok()`. Ein e2e-Test in `engine-server/tests/e2e_history.rs` mit gesetztem Filter würde den Fall abdecken.

## 5 — `GET /api/definitions/{id}/xml` liefert ein anderes Format als dokumentiert (mittel)

**Oberfläche:** Definition ansehen/herunterladen (Desktop), XML-Abruf per API.
**Evidenz:** Handler gibt `Result<String, AppError>` zurück → `text/plain` (`deploy.rs:110-119`), OpenAPI deklariert `application/json` mit `{xml: string}` (`openapi.yaml:144-152`). Fehlt das XML, kommt 400 „No XML found …“ (`deploy.rs:127-129`), dokumentiert ist 404 (`:153`). Der Tauri-Client liest `.text()` — funktioniert, verwirft aber die Servermeldung und meldet nur `Get definition XML failed: 400 Bad Request` (`desktop-tauri/src-tauri/src/commands/deploy.rs:46-48`), obwohl der Nachbarpfad `api_get` den `error`-Text extrahiert (`api_helpers.rs:37-40`).
**Wirkung:** Generierte Clients erwarten JSON und scheitern am Parsen. Der Nutzer sieht bei fehlendem XML keine Ursache; `InstanceDetailDialog.tsx:88-91` schluckt den Fehler komplett (`catch { /* XML optional */ }`) — der Viewer bleibt leer ohne Erklärung.
**Vorschlag:** Auf `{xml}`-JSON umstellen (oder Spec auf `text/plain` korrigieren und 404 liefern), Fehlerpfad analog `api_get` behandeln und im Dialog mindestens einen Hinweis-Badge „XML nicht verfügbar“ rendern.

## 6 — Per-Topic-`lockDuration` im TS-Client wird vom Server still ignoriert (mittel)

**Oberfläche:** `@bpmninja/external-task-client` → `subscribe(topic, handler, { lockDuration })`.
**Evidenz:** Client sendet die Dauer pro Topic korrekt (`ExternalTaskClient.ts:286-289`), README wirbt mit dem Override (`bpmn-ninja-external-task-client/README.md:94-97`). Der Server nimmt jedoch nur `payload.topics.first().lock_duration` (`engine-server/src/server/tasks.rs:73-78`) und wendet sie auf alle Tasks an; `PendingServiceTask` trägt die Dauer nicht, die Abweichung ist im Ergebnis unsichtbar. Zusatz: `autoExtendLock` nutzt immer `this.config.lockDuration` (`ExternalTaskClient.ts:349`), nicht den Topic-Wert.
**Wirkung:** Ein Worker mit `{schnell: 5s, lang: 300s}` bekommt für den kurzen Topic 300 s Lock — verspätete Retries, tote Instanzen, und niemand kann die Ursache im Task-Objekt sehen.
**Vorschlag:** `lock_duration` je Topic durchreichen (Signatur `&[(String, i64)]`), im Client beim Extend `sub.options.lockDuration` verwenden. Beides klein und rückwärtskompatibel.

## 7 — Poll-Loop des TS-Clients hat keinen Backoff und keine Fehlerklassifikation (mittel)

**Oberfläche:** Worker-Betrieb, Onboarding.
**Evidenz:** `fetchAndLock` wirft bei jedem Nicht-2xx eine Exception mit rohem Bodytext (`ExternalTaskClient.ts:305-308`); der Loop loggt und schläft exakt `pollingInterval` (Default 300 ms, `:41`), ohne Unterscheidung von 401/404/429/5xx (`:256-271`).
**Wirkung:** Ein falscher `apiKey` oder eine falsche `baseUrl` erzeugt dauerhaft ~3 Requests/s plus 3 Logzeilen/s, ohne dass der Prozess je deutlich „deine Konfiguration ist falsch“ sagt. Ein 429 wird identisch behandelt wie ein transienter Netzfehler.
**Vorschlag:** Statusklassen mappen (`401/403` → fatal, Client stoppt mit klarer Meldung; `429/5xx` → exponentieller Backoff bis 30 s, `calculateBackoff` existiert bereits in `utils/retry.ts:27`). Zusätzlich `{error}`-Feld aus der Antwort extrahieren, statt den Body zu loggen.

## 8 — Service-Task-/Incident-Sicht ist ungefiltert und unpaginiert (mittel)

**Oberfläche:** `GET /api/service-tasks`, Desktop „Incidents“, „Deployed Processes“.
**Evidenz:** `get_service_tasks` gibt die vollständige Liste zurück (`tasks.rs:25-31`) — ohne `limit`/`offset`, obwohl `/api/instances` beides hat (`instances.rs:91-119`). Die UI lädt deshalb alle Service-Tasks und filtert Incidents im Browser (`IncidentsPage.tsx:26`: `all.filter(t => t.retries <= 0)`), alle 30 s (`:33`). `DeployedProcessesPage` hat weder `usePolling` noch `useEngineEvents` (`:49-65`) — im Gegensatz zu `InstancesPage.tsx:46-47` — und zeigt fremde Deployments (CI, API) erst nach manuellem Refresh.
**Wirkung:** Bei vielen offenen Service-Tasks wird jeder Incident-Poll zum Volltransfer, und die Deployment-Liste widerspricht dem eigenen „Push-Updates“-Versprechen der Nachbarseiten. Beides sind Zustandskonsistenzen, die Nutzer als „die UI hängt“ interpretieren.
**Vorschlag:** `GET /api/service-tasks?state=incident&limit&offset` mit `X-Total-Count` (Muster aus `list_instances` kopieren); in `DeployedProcessesPage` dieselben zwei Hooks wie in `InstancesPage` ergänzen.

---

**Nicht erneut gemeldet:** Shutdown-Flush, NATS-Bootstrap, Timer-Round-Abbruch, `query_history`-Consumer, Fetch-and-Lock-Scan, `async`-`InstanceStore`, Timer-Mutex — bereits im Architektur-Audit belegt und dort besser verortet. Keine Datei geändert.