# Explizite Conditional-Starts (X2c)

Ein ausführbarer Prozess darf genau einen Top-Level-Start mit
`conditionalEventDefinition` haben. Die Condition verwendet dieselbe strikte
Inline-Grammatik wie andere Conditional-Events: eine Variable oder ein einfacher
Vergleich. Gemischte/mehrere Starts, eingehende Flows, Filter, externe Ressourcen,
Expression-Sprachen und Conditional-End-/Throw-Events werden abgelehnt.
Conditional-Starts in eingebetteten Sub-Prozessen bleiben ausgeschlossen;
prozessweite Event-Sub-Prozess-Starts behalten ihren bisherigen Vertrag.

## Aufruf

`POST /api/conditional-starts/evaluate`

```json
{
  "request_id": "b942a402-907e-494f-9989-0aa8cf95d221",
  "process_id": "approval",
  "business_key": "order-42",
  "variables": {"amount": 150}
}
```

Genau einen Selektor angeben: `definition_key` (UUID) oder `process_id` (neueste
Version). `request_id` muss eine nichtleere UUID sein; `variables` ist ein
JSON-Objekt. Quittung und vollständiger Initialsnapshot werden vor Reservierung
gegen das Engine-Payloadlimit (900 KiB, einschließlich Runtime-Overhead) geprüft. `business_key` ist optional und gehört außerhalb von `variables`.
Ein Default für `business_key` wird wie beim normalen Start als Metadatum
entnommen. Defaults der Definition werden zuerst geladen, anschließend mit den
übergebenen Prozessvariablen überschrieben. Task-lokale Werte existieren zu
diesem Zeitpunkt nicht.

Bei False enthält die Antwort `request_id`, die festgelegte `definition_key` und
`matched: false`; es entstehen weder Instanz noch Instanz-Lifecycle-Ereignisse.
Bei True kommt `matched: true` mit `instance_id` zurück; die Instanz läuft ab dem
Conditional-Start entlang ihrer normalen Flows. Native manuelle Start-APIs und
interne Call-Activity-Starts dürfen diesen Weg nicht umgehen.

## Wiederholung und Versionsbindung

Die Request-ID gilt global im konfigurierten Persistenzspeicher. Identischer
Selektor, Business-Key und strukturell identische Variablen liefern dasselbe
Ergebnis, auch nach Abschluss der Instanz. Die Reihenfolge von JSON-Objektfeldern
ist unerheblich. Geänderte Eingaben mit derselben ID liefern HTTP 409. Eine neue
Auswertung oder absichtlich zusätzliche Instanz benötigt eine neue Request-ID.
Auch False-Ergebnisse werden dauerhaft gespeichert und beim Retry nicht erneut
ausgewertet. Es gibt keinen False/True-Latch über verschiedene Requests.

`process_id` wird nur beim ersten akzeptierten Request aufgelöst. Nach einem
Redeploy bleibt dessen Retry an die ursprüngliche Version und das gespeicherte
Ergebnis gebunden. Neue Requests verwenden die tatsächlich neueste Definition;
hat sie keinen Conditional-Start, gibt es einen Validierungsfehler statt eines
Rückgriffs auf eine ältere Version. Deployments, Variablenupdates bestehender
Instanzen, Timer, Signale und Startup starten diese Prozesse nicht implizit.

## Speicherung und Wiederanlauf

Memory und NATS reservieren die Quittung atomar vor der Ausführung. Das eigene
NATS-KV-Bucket heißt `conditional_starts`; Quittungen haben ein explizites
Schemaformat (`version: 1`), Ergebnis, ursprüngliche Eingaben, effektive
Initialvariablen, reservierte Instanz-ID und Dispatch-Marker. Es gibt keine TTL
und keine automatische Löschung. Die Aufbewahrung ist unbegrenzt; Speicherbedarf
und Datenschutz-Löschung müssen beim Betrieb berücksichtigt werden.

Reihenfolge: Quittung reservieren → Initialsnapshot mit Token und bereits
beanspruchtem Start speichern → Listener und gespeicherte Fortsetzung ausführen
→ erste Arbeit veröffentlichen → Dispatch quittieren. Speicherfehler an den
Start-Barrieren werden an den Aufrufer zurückgegeben. Ein Retry verwendet die
reservierte ID und den vorhandenen Tokenzustand. Vorhandene aktive und archivierte
Instanzen werden abgeglichen; abgeschlossene Instanzen werden nicht neu gestartet.
Startup lädt Definitionen, Instanzen und Arbeit vor dem Abgleich und nimmt bei
fehlgeschlagenem Recovery keinen Verkehr an.

Die vorhandenen Occurrence-/Generation-Fences und Listener-Marker sichern
bereits gespeicherte Start-/End-Effekte und Fortsetzungen gegen doppelte
Ausführung. Garantien beziehen sich auf committed Engine-Zustand; beliebige
externe Seiteneffekte erhalten dadurch keine Exactly-once-Garantie. Die bestehende
Beschränkung auf einen schreibenden Engine-Prozess (C6) bleibt bestehen.

History und SSE (`conditional_start_changed`) verbinden Request-ID, Instanz und
Definition, ohne Eingabevariablen in Lifecycle-Metadaten zu kopieren. Es gibt in
diesem Inkrement keine Camunda-Evaluation-API. Grenzen für Call-/Loop-/Container-
Boundaries und verschachtelte Runtime-Scopes bleiben bestehen.
