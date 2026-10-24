---
trigger: file_match
file_patterns: ["engine-server/**", "agent-orchestrator/**"]
---

# Server- und Orchestrator-Regeln

- **Domain:** `engine-server/` (Axum-REST) und `agent-orchestrator/` (Demo-Worker).
- **Rolle:** `engine-server` ist ein dünner HTTP-Adapter; Geschäftslogik bleibt in `engine-core`.
- **State:** `axum::State<Arc<WorkflowEngine>>`; Serialisierung über `serde_json`.

## API-Konventionen
- Routen unter `/api/…` (kein Versionspräfix); Handler nach Ressource gruppiert.
- Erfolg liefert die Ressource direkt, Fehler `{ "error": "<message>" }`.
- Status: 200, 201, 400, 404, 409, 500; `EngineError`-Varianten werden darauf gemappt.
- Pfad-IDs über `axum::extract::Path`, Bodies über `axum::extract::Json`.
- Paginierung: `GET /api/instances?limit&offset` liefert `X-Total-Count`; ohne Query weiterhin das volle Array.
- Externe Task-Endpunkte folgen Camunda: `fetchAndLock`, complete, failure, extendLock, BPMN-Error.
  `lockDuration`/`newDuration` sind Millisekunden im HTTP-Vertrag und werden serverseitig auf Sekunden aufgerundet.

## Betrieb
- CORS über `CORS_ORIGINS` (Kommaliste; unset = lokale Dev-Origins). `*` bedeutet `Any`.
- `BIND_ADDR`/`BIND_ADDRESS` (Default `0.0.0.0`); Nicht-Loopback ohne API-Key ist fail-closed.
- Optionaler Key `BPMNINJA_API_KEY` als `Authorization: Bearer` oder `X-API-Key`; `/api/health` und `/api/ready` bleiben offen.
- Body-Limit `MAX_XML_BYTES` = 10 MiB; Multipart-Upload `MAX_UPLOAD_BYTES` = 5 MiB.
- Logging über `tower_http::trace::TraceLayer`; keine Credentials loggen.
- Keine Hintergrund-Worker oder Timer im Server; die gehören in `engine-core`.

## Orchestrator
`agent-orchestrator` ist ein Stub/Demo-Worker: Tokio-Start, NATS-Verbindung, Parser-Anbindung.
Graceful Shutdown und Env-Konfiguration beachten; keine Business-Logik duplizieren.

## Tests
- Ein Handler pro Endpunkt, Integrationstests ohne Mock-Datenbank.
- NATS-Tests nur mit laufendem Server ausführen oder sauber überspringen.
