# Camunda-7-Worker-Fassade

K1 bietet die fünf External-Task-Operationen unter der Base-URL
`http://localhost:8081/engine-rest` an. Worker mit den unten beschriebenen
Variablentypen können diese URL statt der Camunda-Engine-URL verwenden.
Die vorhandene BPMNinja-API unter `/api` verwendet weiterhin rohe JSON-Werte.

Der Wire-Vertrag orientiert sich an der offiziellen
[Camunda-7-REST-Referenz](https://docs.camunda.org/rest/camunda-bpm-platform/7.22/)
und den [Camunda-Variablentypen](https://docs.camunda.org/javadoc/camunda-bpm-platform/7.24/org/camunda/bpm/engine/variable/type/ValueType.html).
Dies ist eine Worker-Fassade mit expliziten Grenzen, keine vollständige
Camunda-Engine-REST-Implementierung.

| Methode | Pfad relativ zur Base-URL | Ergebnis |
| --- | --- | --- |
| POST | `/external-task/fetchAndLock` | 200, Array mit camelCase-Task-Metadaten und typisierten Variablen |
| POST | `/external-task/{id}/complete` | 204 |
| POST | `/external-task/{id}/failure` | 204 |
| POST | `/external-task/{id}/extendLock` | 204 |
| POST | `/external-task/{id}/bpmnError` | 204 |

## Fetch und Completion

```json
{
  "workerId": "example-worker",
  "maxTasks": 10,
  "asyncResponseTimeout": 30000,
  "usePriority": true,
  "topics": [
    {
      "topicName": "charge-card",
      "lockDuration": 60000,
      "variables": ["amount", "customer"],
      "deserializeValues": true
    }
  ]
}
```

`lockDuration` gilt pro Topic in Millisekunden; verschiedene Topics dürfen
verschiedene Dauern haben. `variables` filtert die Antwort, verändert aber
keine Prozessvariablen. Ohne Auswahl werden aktuelle Instanzvariablen geliefert,
auch wenn sie nach Task-Erstellung verändert wurden. `asyncResponseTimeout`
ist auf 1.800.000 ms begrenzt. Long Polling wacht bei neuen Tasks auf; ablaufende
Locks/Retry-Deadlines werden spätestens beim nächsten 500-ms-Fallback geprüft.
Worker-ID muss 1–255 Zeichen lang sein, `maxTasks` positiv. Topics müssen eindeutig
und nicht leer sein, Lock-Dauern positiv und innerhalb des Datumsbereichs.
Suspendierte Instanzen werden nicht gefetcht.

Completion verwendet Camunda-Variablen-Envelopes:

```json
{
  "workerId": "example-worker",
  "variables": {
    "approved": { "type": "Boolean", "value": true },
    "amount": { "type": "Long", "value": 5 },
    "details": {
      "type": "Json",
      "value": "{\"status\":\"approved\"}",
      "valueInfo": { "serializationDataFormat": "application/json" }
    }
  }
}
```

Unterstützt sind `Null`, `Boolean`, `Short`, `Integer`, `Long`, `Double`, `String`,
`Date` und `Json`. Typ fehlt → Ableitung aus dem JSON-Wert. `Json` wird intern
als JSON gespeichert und für den Worker als serialisierter JSON-String geliefert;
typisiertes null bleibt null. Date-Werte akzeptieren RFC3339 und das Camunda-
Datumsformat mit Zeitzone ohne Doppelpunkt. Zahlenbereiche werden geprüft.
Integer/Long/Short/Date/Json-Typinformationen werden in `variable_types` am
Instanz-Snapshot gespeichert, bleiben bei JSON-/MessagePack-Restore erhalten
und fehlen in alten Snapshots als leerer Default. Native Variablen-Updates
verwerfen die betroffenen Hinweise; nach Script-Änderungen werden nur weiterhin
zum Wert passende Hinweise genutzt. Die Engine bleibt JSON-nativ.

`valueInfo` darf null/leer sein; außerdem werden `transient: false` und für Json
`serializationDataFormat: "application/json"` akzeptiert. Nicht repräsentierbare
Optionen und Typen werden vor der Completion mit 400 abgelehnt.

## Failure, Lock-Verlängerung und BPMN Error

Failure nimmt `workerId`, `retries`, `retryTimeout`, `errorMessage` und
`errorDetails` an. `retries: 0` erzeugt ein Incident; positive Werte halten den
Task erneut verfügbar. `retryTimeout` ist eine nichtnegative Millisekunden-Dauer
(Default 0), deren Deadline im bestehenden persistierten Lock-Ablauf-Feld
gespeichert wird. Der Worker wird freigegeben; vor der Deadline darf kein neuer
Worker den Task fetchen. Auch ein Restart verkürzt diese Wartezeit nicht.
Ohne `retries` gilt die bestehende BPMNinja-Policy: aktuellen Zähler vermindern.
Die Engine beginnt Service Tasks mit drei Retries.

`extendLock` nimmt `workerId` und `newDuration` in Millisekunden an und setzt den
Ablauf ab dem aktuellen Zeitpunkt. Alle vier Task-Aktionen prüfen den aktiven
Worker-Lock; falscher Worker, abgelaufener Lock oder suspendierte Instanz → 400.
Unbekannter Task → 404. Die Engine prüft Ownership bei der Mutation erneut.

`bpmnError` nimmt `workerId`, `errorCode`, optional `errorMessage` und typisierte
`variables` an. Werte stehen dem Boundary-Pfad vor dessen Ausführung zur
Verfügung; die Nachricht wird ins Audit-Log aufgenommen. Ein nicht gefangener
BPMN Error folgt der bestehenden BPMNinja-Policy und erzeugt ein Incident.

## Grenzen und Authentifizierung

Rust-Aufrufer, die `ProcessInstance` per Struct-Literal erstellen, müssen das
neue Feld `variable_types` initialisieren (z. B. mit `Default::default()`).
Bestehende persistierte Instanzen erhalten beim Lesen automatisch eine leere Map.

- Keine Java-Object-/XML-/Bytes-/File-Serialisierung und keine lokalen oder
  transienten Variablen-Scopes; nichtleere `localVariables` werden abgelehnt.
- Keine Tenant-/Definitions-/Business-Key-/Variablen-Filter im Fetch. Alle Tasks
  sind tenantlos; `withoutTenantId` passt deshalb auf alle Tasks.
- Alle Prioritäten sind 0. `usePriority` ist akzeptiert; eine Sortierungsoption
  oder Extension-Properties-Abfrage wird abgelehnt.
- `deserializeValues` ist akzeptiert; für die unterstützten Typen unterscheidet
  sich der Wire-Wert nicht zwischen true/false. Keine Java-Klassen werden geladen.
- Keine zusätzlichen Camunda-Operationen (Query, Unlock, SetRetries, Deployment,
  History usw.). Incidents können weiter über die native BPMNinja-API bearbeitet
  werden. Call-Activity-/Subprozess-Scope-Semantik bleibt die der Engine.

Bei gesetztem `BPMNINJA_API_KEY` den Schlüssel über Bearer oder `X-API-Key`
mitgeben, beispielsweise mit dem Header-Interceptor des Worker-SDKs. Camunda-
Basic-Auth wird nicht implementiert. Fehler auf den fünf Routen enthalten
`type`, `message` und `code`; eine fehlende/falsche API-Key-Authentifizierung
liefert 401. Body-Limits und CORS gelten wie für die native API.

Der unterstützte Vertrag steht auch in `api-spec/main.tsp` und der generierten
OpenAPI-Datei. Regressionstests:

```sh
cargo test --locked -p engine-core --lib camunda_compat
cargo test --locked -p engine-server --lib camunda
cargo test --locked -p persistence-nats --lib --all-features
```
