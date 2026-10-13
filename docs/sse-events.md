# SSE-Zustandsupdates

`GET /api/events` sendet weiterhin die Event-Namen `instance_changed`,
`task_changed` und `definition_changed`. Das `data`-Feld enthält jetzt JSON mit
Entity-IDs. Bestehende Clients, die nur den Event-Namen auswerten, bleiben
kompatibel.

```text
event: instance_changed
data: {"type":"instance_changed","instance_id":"…","definition_key":"…","removed":false,"delta":{"state":"Completed","current_node":"end","business_key":"","started_at":"…","completed_at":"…"}}

```

`delta` ist eine kompakte aktuelle **Listenprojektion**, kein vollständiges
Instanz-Dokument und kein JSON-Patch. Sie enthält Zustand, aktuellen Knoten,
Business-Key und Start-/Abschlusszeit. Clients können diese Felder auf einer
bekannten Zeile ersetzen. Bei Call-Activities enthält der Zustand nur die
Child-ID, keinen geparkten Token (auch nicht innerhalb eines Suspended-Zustands).
Variablen, Tokens und Audit-Logs werden nicht im
Broadcast kopiert; Details werden bei Bedarf über die Instanz-ID nachgeladen.
Für neue, lokal unbekannte Instanzen genügt ein einzelner Detail-Request.
`removed: true` entfernt die Zeile; das Delta entfällt dann. Ein erfolgreicher
Batch, administrative Variablen-/Statusänderungen, Migration und Löschung
erzeugen Instanz-Events. Fehlerzustände im Batch werden ebenfalls gemeldet.

`task_changed` enthält `instance_id`, `task_id` und gegebenenfalls
`definition_key`. Es invalidiert diesen Task beziehungsweise die Tasks dieser
Instanz; es ist kein vollständiger Task-Snapshot. Erstellung, erfolgreiche
Completion, Lock-/Retry-/Incident-Updates melden die betroffenen IDs.
Instanz-Löschung invalidiert auch deren Tasks. `definition_changed` enthält
`definition_key` und meldet Deployment/Löschung.

## Verlust und Wiederverbindung

Der Broadcast bleibt begrenzt und besitzt kein Replay. Ein langsamer Subscriber
erhält ausdrücklich:

```text
event: resync_required
data: {"type":"resync_required"}

```

Danach verwirft der Server den alten Broadcast-Rückstand und liefert neue
Events. Clients müssen ihre Daten erneut laden. Die Tauri-Bridge löst auch nach
jeder erfolgreichen neuen Verbindung einen Resync aus und leitet JSON-Payloads
erst nach dem vollständigen SSE-Nachrichtenblock weiter. Mehrzeilige `data:`-
Felder werden zusammengesetzt; ungültige Payloads führen zu Resync. Alte Server
mit leeren Daten bleiben unterstützt und lösen breite Invalidierungen aus.

## Desktop-Verhalten

- Die Instanzliste übernimmt Deltas für bekannte Zeilen direkt und lädt neue
  Instanzen einzeln. Löschungen entfernen die betroffene Zeile. Deltas, die
  während eines REST-Reloads eintreffen, gewinnen gegen dessen Snapshot.
- Ein geöffnetes Instanzdetail reagiert nur auf die passende Instanz-ID;
  lokale Variablen-Edits werden bei Hintergrundupdates bewahrt.
- Die Definitionsansicht ignoriert Events fremder Definitionen; Migrationen
  bereits angezeigter Instanzen bleiben berücksichtigt.
- Aggregate-/Task-Ansichten bündeln passende Events in festen 150-ms-Fenstern.
  Bei kontinuierlicher Last entstehen damit regelmäßige Updates statt eines
  endlos verschobenen Debounce. Resync umgeht Typ-/Entity-Filter.
- Das vorhandene 30-s-Polling und manuelle Aktualisierung bleiben Fallbacks.

Die SSE-Verbindung verwendet dieselbe API-Key-Authentifizierung wie die übrige
API. Es gibt keine zusätzliche Event-Historie und keine `Last-Event-ID`-Garantie.
