# BPMN-Element-Matrix

Stand der BPMN-2.0-Elemente in BPMNinja. Abgeleitet aus `bpmn-parser/src/parser.rs`
(`reject_unsupported_elements`, `SUPPORTED_EVENT_DEFINITIONS`) und `engine-core`.

| Symbol | Bedeutung |
|---|---|
| ✅ | Geparst **und ausgeführt** |
| 🟡 | Akzeptiert, **ohne Ausführungssemantik** (Darstellung, Dokumentation, Daten — der Fluss ändert sich nicht) |
| ❌ | **Beim Deploy abgelehnt** mit `InvalidDefinition`; die Meldung nennt Typ, Id und Zeile |
| ⚠️ | **Teilweise** — nur in der hier beschriebenen Form |

Beim Einzel-Deploy wird nur der deployte Prozess geprüft (erster mit `isExecutable="true"`,
sonst der erste). `parse_bpmn_processes` prüft jeden Pool mit `isExecutable="true"` und
ignoriert nicht ausführbare Pools. Leere oder doppelte Prozess-IDs lehnen das ganze
Dokument ab. Der Deploy meldet **alle** Treffer eines Prozesses auf einmal, nicht nur
den ersten. Es gibt keinen Schalter, der die Prüfung abschaltet.

## Ereignisse

| Element | Start | Intermediate Catch | Intermediate Throw | Boundary | End |
|---|---|---|---|---|---|
| None | ✅ | — | ❌ | — | ✅ |
| Message | ✅ | ✅ | ✅ (als Send) | ✅ unterbrechend / nicht unterbrechend | ❌ |
| Timer (Duration, Date, Cycle) | ✅ | ✅ | — | ✅ unterbrechend / nicht unterbrechend | — |
| Error | ❌ | — | — | ✅ | ✅ |
| Escalation | ❌ | — | ✅ | ✅ unterbrechend / nicht unterbrechend | ✅ |
| Compensation | ❌ | — | ✅ | ✅ | ✅ |
| Conditional | ❌ | ✅ | — | ✅ unterbrechend / nicht unterbrechend | — |
| Signal | ✅ | ✅ | ✅ | ✅ unterbrechend / nicht unterbrechend | ✅ |
| Link | — | ✅ (nicht materialisiert) | ✅ | — | — |
| Cancel | ❌ | ❌ | ❌ | ❌ | ❌ |
| Terminate | — | — | — | — | ✅ |
| Multiple / Parallel Multiple | ❌ | ❌ | ❌ | ❌ | ❌ |

Mehrere Event-Definitionen am selben Ereignis sind ❌.

Ein Start-Ereignis **innerhalb** eines eingebetteten Sub-Prozesses darf keine
Event-Definition tragen: der interne Start ist ein Durchreicher, der Trigger würde
verworfen. Timer-, Message- und Signal-Starts sind dort ❌, obwohl sie auf
Prozessebene ✅ sind.

Signal-, Error-, Escalation-, Compensation- und Terminate-Endereignisse **innerhalb**
eines eingebetteten Sub-Prozesses werden beim Flatten zu `SubProcessEndEvent` und
behalten ihre Sonderbedeutung nicht (kein Broadcast, kein Abbruch des Elternprozesses).
Das ist die dokumentierte Flattening-Abweichung, kein stilles Wegwerfen eines auf
Prozessebene unbekannten Elements.

## Aktivitäten

| Element | Status | Anmerkung |
|---|---|---|
| User Task | ✅ | Assignee aus `data-assignee` / `camunda:assignee` |
| Service Task | ✅ | External Task; Topic aus `data-topic` / `data-handler` / `camunda:topic` |
| Script Task | ✅ | Rhai; leeres Script ❌ |
| Send Task | ✅ | Wirft eine Nachricht und läuft weiter |
| Receive Task | ✅ | Wie Message-Catch. `instantiate="true"` auf Prozessebene ist ein Message-Start (Collaboration-Ziel). Im Sub-Prozess und mit eingehendem Sequenzfluss ❌ |
| Manual Task | ✅ | Wie User Task |
| Business Rule Task | ✅ / ❌ | Mit Topic → External Task; ohne Topic ❌ (kein DMN) |
| Task (abstrakt) | ❌ | Konkreten Task-Typ verwenden |
| Call Activity | ✅ | Ohne Mapping alle Variablen in beide Richtungen. Mit `camunda:in`/`camunda:out` nur die genannten Ziele; eine Richtung macht beide Seiten explizit. Error-Boundaries |
| Embedded Sub-Process | ✅ | Beim Parsen geflacht (ADR-003). Boundary-Events am Sub-Prozess sind nicht spezifikationsgenau |
| Event-Sub-Process | ❌ | `triggeredByEvent="true"` wird nicht ausgelöst |
| Transaction | ❌ | |
| Ad-hoc Sub-Process | ❌ | |
| Loop (`standardLoopCharacteristics`) | ❌ | |
| Multi-Instance parallel / sequentiell | ✅ / ❌ | Nur auf `serviceTask` (`collection` + `elementVariable` oder `loopCardinality`). Andere Task-Typen ❌. `completionCondition` wird nicht ausgewertet (die Instanz wartet auf alle Elemente) |
| Compensation-Aktivität (`isForCompensation`) | ⚠️ | Der Handler wird über einen **Sequenzfluss** am Compensation-Boundary gefunden. Die Standardform über eine **Association** (bpmn-js) wird noch nicht gelesen |

## Gateways

| Element | Status | Anmerkung |
|---|---|---|
| Exclusive (XOR) | ✅ | Bedingungen und Default-Flow |
| Parallel (AND) | ✅ | Fork / Join |
| Inclusive (OR) | ✅ | Join zählt die genommenen Zweige des passenden Splits |
| Complex | ✅ | `activationCondition` |
| Event-based (exclusive) | ✅ | Ziele: Message, Timer oder Signal. Conditional-Catch ❌ |
| Event-based instanziierend / parallel | ❌ | `instantiate="true"` oder `eventGatewayType="Parallel"` |

## Verbindende Objekte

| Element | Status | Anmerkung |
|---|---|---|
| Sequence Flow | ✅ | Bedingte und Default-Flows |
| Message Flow | ⚠️ | Gerichtete interne Route zwischen ausführbaren Pools derselben Collaboration. `plan_internal_messages` pinnt Quell- und Zielversion; es wird nichts automatisch zugestellt. Participant-Ebene, verschachtelte Scopes und Schleifen ❌ |
| Association | 🟡 | Compensation-Associations werden noch nicht ausgewertet |
| Data Association (Input / Output) | ⚠️ | An User, Service, Script, Send Task und Call Activity ausgeführt (`sourceRef`/`targetRef` oder Camunda `inputOutput`). Transformation, Skript, Interpolation und Collection werden abgelehnt |

## Swimlanes und Artefakte

| Element | Status | Anmerkung |
|---|---|---|
| Pool (Participant) | ⚠️ | `POST /api/deploy` bleibt ein Prozess. `POST /api/deploy/collection` deployed alle Pools mit `isExecutable="true"` atomar. Black-Box-Pools bleiben Metadaten |
| Lane / Lane Set | 🟡 | |
| Group | 🟡 | |
| Text Annotation | 🟡 | |

## Daten

| Element | Status | Anmerkung |
|---|---|---|
| Data Object / Data Object Reference | ⚠️ | Deklaration. `bpmninja:default` ist JSON und gilt nur beim Start; ohne Default entsteht kein Wert |
| Data Store Reference | 🟡 | |
| Data Input / Data Output / `ioSpecification` | ⚠️ | An unterstützten Aktivitäten Endpunkte von Assoziationen. Collection sowie Prozess- und Subprozess-IO werden abgelehnt |
| Property | ⚠️ | Prozess-Property wie ein Data Object. Task-Property nur als Assoziations-Endpunkt, kein eigener Default |
| Message, Error, Escalation, Signal (Wurzelelemente) | ✅ | Werden von Event-Definitionen referenziert |

## Pflege

Der folgende Index ist die maschinenlesbare Fassung der Listen in
`bpmn-parser/src/parser.rs`. `element_matrix_matches_parser_support_lists` bricht ab,
wenn Index und Code auseinanderlaufen. Ein neues unterstütztes Element wird dort
eingetragen, aus der Ablehnung genommen und hier nachgezogen.

```support-index
unsupported-elements: adHocSubProcess standardLoopCharacteristics task transaction
rejected-attributes: eventBasedGateway@eventGatewayType=Parallel eventBasedGateway@instantiate subProcess@triggeredByEvent
rejected-events: boundaryEvent/cancelEventDefinition endEvent/cancelEventDefinition startEvent/conditionalEventDefinition
boundaryEvent: compensateEventDefinition conditionalEventDefinition errorEventDefinition escalationEventDefinition messageEventDefinition signalEventDefinition timerEventDefinition
endEvent: compensateEventDefinition errorEventDefinition escalationEventDefinition signalEventDefinition terminateEventDefinition
intermediateCatchEvent: conditionalEventDefinition linkEventDefinition messageEventDefinition signalEventDefinition timerEventDefinition
intermediateThrowEvent: compensateEventDefinition escalationEventDefinition linkEventDefinition messageEventDefinition signalEventDefinition
startEvent: messageEventDefinition signalEventDefinition timerEventDefinition
```
