---
title: Kompatibilität mit BPMN 2.0 und Camunda 7
description: Eine Referenzseite für alle Abweichungen von BPMN 2.0 und Camunda 7 — unterstützte Attribute, geänderte Laufzeitsemantik, nicht unterstützte Elemente, API-Unterschiede und eine Migrationscheckliste für Modelle und Worker.
audience:
  - Prozessmodellierer
  - Entwickler und Integratoren
  - Betriebsteams und Plattform-Administratoren
prerequisites:
  - Erfahrung mit BPMN 2.0 oder Camunda 7
  - Kenntnis der BPMNinja-Grundbegriffe (Token, Service Task, Incident)
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/bpmn-elemente.md
  - reference/service-task-api.md
  - reference/rest-api.md
  - explanation/konzepte.md
  - how-to/external-task-worker-schreiben.md
---

# Kompatibilität mit BPMN 2.0 und Camunda 7

Diese Seite sammelt **alle** bekannten Abweichungen an einer Stelle. Sie ist die Antwort auf die Frage „Verhält sich das wie in Camunda?" — mit der Richtung je Punkt: **gleich**, **abweichend** oder **nicht unterstützt**.

**Beleglage:** Quelltext (`bpmn-parser/`, `engine-core/`, `engine-server/`, Version 0.7.24) und `README.md`. Wo eine Aussage nur aus der Dokumentation abgeleitet ist und nicht durch Code oder Test belegt, steht sie unter **Offen**.

## 1. Kurzfassung

| Bereich | Richtung | Kernaussage |
|---|---|---|
| BPMN-2.0-Kern (Events, Tasks, Gateways, Flows) | gleich | Die Grundmenge wird unterstützt und ausgeführt. |
| Service Tasks | abweichend | Ausschließlich External-Task-Pattern; kein eingebetteter Java-Delegate, kein Connector. |
| User Tasks | abweichend | Keine Formulare, keine Tasklist, keine Zuweisungs-Engine; Abschluss über REST. |
| Script Tasks | abweichend | Rhai statt JavaScript, Groovy oder JUEL. |
| Expression Language | abweichend | Vergleichsoperatoren und Truthy-Prüfungen; kein vollständiges JUEL/FEEL. |
| Embedded Sub-Processes | abweichend | Werden beim Parsen geflattet; keine Laufzeit-Verschachtelung. |
| Multi-Instance | abweichend | Nur parallele Multi-Instance an `serviceTask`. |
| DMN / Decision Tasks | nicht unterstützt | Keine Decision-Engine; `businessRuleTask` ist ein externer Task. |
| Signal-, Cancel- und Link-Events | nicht unterstützt | Werden nicht ausgelöst. |
| Data Objects / Data Stores | nicht unterstützt | Werden ignoriert; Datenaustausch über JSON-Variablen. |
| Camunda-REST-API | abweichend | Eigene Endpunkte, eigene Payloads; keine Drop-in-Kompatibilität. |
| Persistenz | abweichend | NATS JetStream statt relationaler Datenbank. |
| History | abweichend | Event-Sourcing in NATS; eigenes Abfragemodell. |

---

## 2. Service Tasks und Worker

| Aspekt | Camunda 7 | BPMNinja |
|---|---|---|
| Ausführung | Eingebetteter Java-Delegate, Connector oder External Task | **Nur** External Task (Fetch-and-Lock) |
| Topic-Attribut | `camunda:topic` | `data-topic` → `data-handler` → `camunda:topic` → Node-ID |
| Lock-Dauer | Millisekunden | `fetchAndLock`: Millisekunden, gültig 1 000 bis 3 600 000, intern `(ms + 999) / 1000`. `extendLock`: keine Validierung, Klemmung der Sekunden auf 1..=3600. |
| Fehler melden | `bpmnError`, `failure` | `bpmnError`, `failure`; Statuszuordnung siehe [`reference/service-task-api.md`](service-task-api.md) |
| Wiederholungen | `retries` pro Task | `retries` pro Task, Startwert `3`; `failure(..., 0)` erzeugt sofort einen Incident |
| Client | Camunda External Task Client (Java, JS, …) | TypeScript-Client im Repository-Paket `bpmn-ninja-external-task-client` |
| Task-Listen | Camunda Tasklist | Keine; User Tasks werden über die REST-API abgeschlossen |

**Worker-Migration:** Ein Camunda-External-Task-Client spricht ein anderes Protokoll. Der Wechsel erfordert einen neuen Worker auf Basis von [`how-to/external-task-worker-schreiben.md`](../how-to/external-task-worker-schreiben.md). Die Handler-Logik kann übernommen werden, die Client-Aufrufe nicht.

---

## 3. User Tasks

| Aspekt | Camunda 7 | BPMNinja |
|---|---|---|
| Formulare | Camunda Forms, Embedded Forms | Keine Formular-Engine |
| Tasklist | Camunda Tasklist | Keine |
| Zuweisung | `assignee`, `candidateUsers`, `candidateGroups` | `data-assignee` → `camunda:assignee` → `unassigned`; keine Kandidatenauflösung |
| Abschluss | Tasklist oder REST | `POST /api/complete/{id}` |
| Fälligkeit, Priorität | Unterstützt | Nicht als Vertrag dokumentiert |

Ein Modell, das auf Kandidatengruppen oder Formulare setzt, lässt sich deployen, aber die Zuweisungslogik muss außerhalb der Engine abgebildet werden.

---

## 4. Scripting und Ausdrücke

| Aspekt | Camunda 7 | BPMNinja |
|---|---|---|
| Skriptsprache | JavaScript, Groovy, JUEL | **Rhai** |
| Skript-Task | `scriptTask` mit `scriptFormat` | `scriptTask` mit inline `<script>` oder `data-script`; `scriptFormat` wird nicht ausgewertet |
| Leerer Skript-Task | Zulässig, wenn extern befüllt | **Ablehnung** (`has an empty script`) |
| Listener | Execution Listener (Java, Script) | Execution Listener als Rhai (`event="start"`/`"end"`) |
| Bedingungen | JUEL/FEEL | `==`, `!=`, `>`, `>=`, `<`, `<=`, Truthy |
| Ressourcenlimits | JVM | 50 000 Operationen, 2 MiB, 1 000 ms (konfigurierbar) |

**Konsequenz für die Migration:** Jeder Script Task und jeder Execution Listener muss in Rhai neu geschrieben werden. Variablenzugriffe und Rückgabewerte folgen der Rhai-Syntax, nicht der Camunda-Skript-API.

---

## 5. Sub-Prozesse und Call Activities

| Aspekt | Camunda 7 | BPMNinja |
|---|---|---|
| Embedded Sub-Process | Eigener Scope mit eigener Instanz | Beim Parsen **geflattet**; zur Laufzeit nur Knoten im Hauptgraphen |
| Variablen-Scope | Hierarchisch | Ein Variablen-Scope; `SubProcessEndEvent` kehrt in denselben Scope zurück |
| Call Activity | `calledElement`, Versionierung, Binding | `calledElement` auf eine deployte Definition; Variablen werden propagiert |
| Transaction Sub-Process | Unterstützt | Nicht unterstützt |
| Ad-Hoc Sub-Process | Unterstützt | Nicht unterstützt |
| Event Sub-Process | Unterstützt | Scope-Event-Listener für Timer, Nachricht und Fehler; unterbrechend oder nicht-unterbrechend |

**Wichtig für Modellierer:** Ein Boundary Event an einem geflatteten Sub-Prozess verhält sich anders als in Camunda, weil es zur Laufzeit keine verschachtelte Aktivität gibt. Prüfe solche Modelle vor der Migration.

---

## 6. Multi-Instance

| Variante | Camunda 7 | BPMNinja |
|---|---|---|
| Parallel, `collection` | Unterstützt | Unterstützt, nur `serviceTask` |
| Parallel, `loopCardinality` | Unterstützt | Unterstützt, nur `serviceTask` |
| Sequenziell (`isSequential="true"`) | Unterstützt | **Nicht unterstützt** (`InvalidDefinition`) |
| Multi-Instance an User Task | Unterstützt | **Nicht unterstützt** |
| Multi-Instance an Script/Send/Receive Task | Teilweise | **Nicht unterstützt** |

Details und die genauen Ablehnungsmeldungen stehen in [`reference/bpmn-elemente.md`](bpmn-elemente.md#5-multi-instance).

---

## 7. Ereignisse und Fehler

| Aspekt | Camunda 7 | BPMNinja |
|---|---|---|
| Error Boundary Event | `errorRef` auf `<bpmn:error>` | `errorRef` wird über `<bpmn:error id>` aufgelöst; **ohne Treffer Catch-all** |
| Error End Event | `errorRef`/`errorCode` | Aufgelöster Code; ohne Treffer `generic_error` |
| Eskalation | `escalationRef` | Aufgelöst; ohne Treffer `generic_escalation` |
| Compensation | Boundary + Throw/End | Unterstützt, LIFO oder `activityRef` |
| Signal Events | Unterstützt | **Nicht unterstützt** |
| Cancel Events | Unterstützt | **Nicht unterstützt** |
| Link Events | Unterstützt | **Nicht unterstützt** |
| Message Correlation | `messageName` + `businessKey` | `messageName` + optional `businessKey` |
| Timer | Duration, Date, Cycle | Duration, Date, Cron, Wiederholungsintervall |

Die Catch-all-Semantik des Error Boundary Events ist die gefährlichste Abweichung: Ein Modell mit `errorRef="INVALID_ADDRESS"` ohne passendes `<bpmn:error>` fängt in BPMNinja **jeden** Fehler an dieser Aktivität. Camunda würde den Deploy ablehnen. Deshalb immer ein `<bpmn:error>` deklarieren.

---

## 8. Variablen und Datenaustausch

| Aspekt | Camunda 7 | BPMNinja |
|---|---|---|
| Variablenmodell | Typisierte Variablen (String, Number, Boolean, Object, …) | `HashMap<String, serde_json::Value>` |
| Serialisierung | Java-Serialisierung oder JSON | JSON |
| Datei-Variablen | Über Spin/Connectors | NATS Object Store `instance_files` |
| Data Objects | Modelliert und ausgewertet | Ignoriert; Austausch ausschließlich über Variablen |
| Input/Output-Mapping | `camunda:inputOutput` | Nicht als Vertrag dokumentiert |

---

## 9. API und Betrieb

| Aspekt | Camunda 7 | BPMNinja |
|---|---|---|
| REST-Basis | `/engine-rest` | `/api` |
| Auth | Basic, Session, Filter | `X-API-Key` mit Admin- und Worker-Rolle |
| External-Task-Endpunkte | `/external-task/...` | `/api/service-task/...` |
| Deploy | Multipart, mehrere Ressourcen | `POST /api/deploy` mit JSON `{name, xml}`; **ein Prozess pro Datei** |
| Persistenz | Relationale Datenbank | NATS JetStream (KV, Object Store, Streams) |
| Skalierung | Mehrere Engine-Knoten mit gemeinsamer DB | **Single Writer**; Skalierung über Worker |
| History | Act-History-Tabellen | Event-Sourcing, `history.instance.*` |
| Monitoring | Metrics, Cockpit | Prometheus, `/metrics`, SSE, Desktop-UI |

Die Endpunkt- und Payload-Referenz steht in [`reference/rest-api.md`](rest-api.md); die Rollenmatrix in [`reference/konfiguration.md`](konfiguration.md).

---

## 10. Migrationscheckliste

Arbeite die Punkte vor dem Deploy eines bestehenden Camunda-Modells ab:

1. **Service Tasks:** Jeden Task auf External-Task-Pattern prüfen. Java-Delegates und Connectoren entfernen, Topic vergeben.
2. **User Tasks:** Formulare und Kandidatengruppen außerhalb der Engine abbilden; Assignee über `data-assignee` setzen.
3. **Script Tasks und Listener:** In Rhai übersetzen; leere Skripte entfernen oder befüllen.
4. **Bedingungen:** Auf die unterstützten Operatoren reduzieren; JUEL-Funktionen ersetzen.
5. **Multi-Instance:** Sequenzielle Loops und Multi-Instance an Nicht-Service-Tasks umbauen.
6. **Fehler:** Für jeden `errorRef` ein `<bpmn:error>` deklarieren, sonst entsteht ein Catch-all.
7. **Sub-Prozesse:** Geflattete Sub-Prozesse und ihre Boundary Events prüfen.
8. **Nicht unterstützte Elemente:** Signal-, Cancel- und Link-Events sowie Data Objects entfernen oder ersetzen.
9. **Prozesse pro Datei:** Pro Deploy-Datei genau einen ausführbaren Prozess belassen.
10. **Worker:** Auf den TypeScript-Client umstellen; Idempotenz und Lock-Dauer prüfen.

**Abnahmekriterium:** Das Modell deployt ohne `InvalidDefinition`, jeder Service Task wird von einem Worker bearbeitet, und jeder fachliche Fehler läuft in den vorgesehenen Boundary-Pfad.

---

## Offen

- **Camunda-8-Kompatibilität:** Diese Seite vergleicht mit Camunda 7. Eine Aussage zu Zeebe/Camunda 8 ist nicht belegt und fehlt bewusst.
- **Input/Output-Mapping:** Ob `camunda:inputOutput` teilweise ausgewertet wird, ist nicht als Vertrag dokumentiert. **Offen:** Parser-Prüfung.
- **User-Task-Felder:** Fälligkeit, Priorität und Kandidaten werden nicht aufgelöst; eine vollständige Liste der ignorierten Attribute fehlt.
- **Bedingungssprache:** Die genaue Grammatik der Bedingungsauswertung ist nicht als Referenz dokumentiert; die Beispiele stammen aus dem Parser.
- **Incident-Art bei ungefangenem Error End Event:** In Abschnitt 7 ist nur die Code-Auflösung belegt. Ob ein ungefangenes Error End Event einen Incident erzeugt und welcher Art, ist **nicht belegt** (siehe [`reference/bpmn-elemente.md`](bpmn-elemente.md) §Offen).
- **Versionsstand:** Die Seite gilt für BPMNinja 0.7.24. Ein „gültig ab"-Feld fehlt, weil die Versionierung der Dokumentation noch nicht eingeführt ist.
