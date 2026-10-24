---
title: BPMN-Elemente und ihre Abbildung
description: Referenz der BPMN-2.0-Elemente, die BPMNinja beim Deploy akzeptiert — Abbildung auf Laufzeittypen, Auflösungsreihenfolgen für Topic, Assignee, Nachrichten und Fehlercodes, unterstützte Multi-Instance-Varianten sowie die Elemente, die den Deploy mit InvalidDefinition ablehnen.
audience:
  - Prozessmodellierer
  - Entwickler und Integratoren
  - Beitragende und Maintainer
prerequisites:
  - Grundkenntnis von BPMN 2.0
  - Ein deploybares Prozessmodell (XML)
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/kompatibilitaet.md
  - reference/rest-api.md
  - how-to/prozess-deployen.md
  - tutorials/erster-prozess.md
  - how-to/external-task-worker-schreiben.md
  - explanation/konzepte.md
---

# BPMN-Elemente und ihre Abbildung

Diese Seite beantwortet zwei Fragen vor dem Deploy: **Wird mein Element akzeptiert?** und **welches Laufzeitverhalten entsteht daraus?** Sie ist gegen den Parser geprüft (`bpmn-parser/src/parser.rs`, `bpmn-parser/src/models.rs`, `engine-core/src/domain/element.rs`, Version 0.7.24). Abweichungen gegenüber BPMN 2.0 und Camunda 7 stehen gesammelt in [`reference/kompatibilitaet.md`](kompatibilitaet.md); der Ablauf eines Deploys steht in [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md).

## Deploy-Regeln, die für alle Elemente gelten

| Regel | Verhalten | Beleg |
|---|---|---|
| Ein Prozess pro Datei | Nur der erste `<process isExecutable="true">` wird deployt. Ist keiner als executable markiert, wird der erste `<process>` verwendet. Weitere Prozesse derselben Datei werden ignoriert. | `bpmn-parser/src/parser.rs` |
| Unbekannte Elemente | Werden ignoriert, sofern sie nicht zu einem expliziten Ablehnungsfall gehören (Abschnitt 7). | `BpmnDefinitions` mit `IgnoredElement` |
| Mehrfach-Multi-Instance | Nur `serviceTask` unterstützt Multi-Instance (Abschnitt 5). Alle anderen Elemente lehnen `multiInstanceLoopCharacteristics` mit `InvalidDefinition` ab. | `parse_multi_instance`, `reject_multi_instance` |
| Fehlerformat bei Ablehnung | `400` mit `{ "error": "<Begründung>" }`; die Meldung nennt Element-ID und Regel. | `reference/rest-api.md` |

Ein Deploy ist atomar: Schlägt das Parsen fehl, wird weder eine Definition noch das XML gespeichert.

---

## 1. Ereignisse

### 1.1 Start-Ereignisse

| BPMN-XML | Laufzeittyp | Verhalten | Einschränkung |
|---|---|---|---|
| `<startEvent>` ohne Definition | `StartEvent` | Instanz startet sofort. | — |
| `<startEvent><timerEventDefinition>` | `TimerStartEvent` | Startet nach ISO-8601-Dauer (`PT30S`), absolutem Datum, Cron-Ausdruck oder Wiederholungsintervall. | Timer wird vom Scheduler verarbeitet (`TIMER_INTERVAL_MS`, Default 1000 ms). |
| `<startEvent><messageEventDefinition>` | `MessageStartEvent` | Startet bei Korrelation über `messageName`. | `messageRef` wird über `<message id name>` aufgelöst; fehlt die Auflösung, entsteht `generic_message`. |

### 1.2 End-Ereignisse

| BPMN-XML | Laufzeittyp | Verhalten | Einschränkung |
|---|---|---|---|
| `<endEvent>` | `EndEvent` | Beendet den Pfad; die Instanz ist abgeschlossen, wenn kein Token mehr läuft. | — |
| `<endEvent><terminateEventDefinition>` | `TerminateEndEvent` | Beendet sofort alle aktiven Tokens der Instanz. | — |
| `<endEvent><errorEventDefinition errorRef="…">` | `ErrorEndEvent` | Wirft einen BPMN-Fehler mit aufgelöstem Code. | Auflösung siehe Abschnitt 6.4; ohne Treffer wird `generic_error` verwendet. Ob ein ungefangenes Error End Event einen Incident erzeugt, steht unter **Offen**. |
| `<endEvent><escalationEventDefinition>` | `EscalationEndEvent` | Wirft eine Eskalation; ungefangen bleibt sie nicht fatal und der Token läuft weiter. | `generic_escalation` ohne Treffer. |
| `<endEvent><compensateEventDefinition>` | `CompensationEndEvent` | Führt registrierte Kompensationshandler aus (optional `activityRef`, sonst LIFO) und schließt den Token ab. | — |

### 1.3 Intermediate Catch- und Throw-Ereignisse

| BPMN-XML | Laufzeittyp | Verhalten | Einschränkung |
|---|---|---|---|
| `<intermediateCatchEvent><timerEventDefinition>` | `TimerCatchEvent` | Pausiert, bis der Timer abläuft. | — |
| `<intermediateCatchEvent><messageEventDefinition>` | `MessageCatchEvent` | Pausiert, bis `POST /api/message` mit passendem `messageName` korreliert. | Ohne auflösbaren `messageRef`: `generic_message`. |
| `<intermediateThrowEvent><messageEventDefinition>` | `SendTask` | Publiziert die Nachricht und läuft sofort weiter. | Ohne auflösbaren `messageRef`: `generic_throw`. |
| `<intermediateThrowEvent><escalationEventDefinition>` | `EscalationThrowEvent` | Wirft eine Eskalation an den umgebenden Scope. | — |
| `<intermediateThrowEvent><compensateEventDefinition>` | `CompensationThrowEvent` | Führt Kompensationshandler aus (`activityRef` oder alle LIFO) und läuft weiter. | — |

Ein `intermediateCatchEvent` **ohne** Timer- oder Message-Definition wird abgelehnt:

```text
intermediateCatchEvent '<id>' has no timerEventDefinition or messageEventDefinition
```

### 1.4 Boundary-Ereignisse

| BPMN-XML | Laufzeittyp | Verhalten | Einschränkung |
|---|---|---|---|
| `<boundaryEvent><timerEventDefinition>` | `BoundaryTimerEvent` | Unterbricht oder begleitet die Aktivität; der Timer wird beim Abschluss der Aktivität abgebrochen. | `cancelActivity="false"` erzeugt einen nicht-unterbrechenden Pfad. |
| `<boundaryEvent><messageEventDefinition>` | `BoundaryMessageEvent` | Wartet asynchron auf eine Nachricht an der Aktivität. | — |
| `<boundaryEvent><errorEventDefinition>` | `BoundaryErrorEvent` | Fängt einen `bpmnError`-Code des Service Task. | **Ohne auflösbaren `errorRef` fängt das Ereignis jeden Fehler** (Abschnitt 6.4). |
| `<boundaryEvent><escalationEventDefinition>` | `BoundaryEscalationEvent` | Fängt eine Eskalation der Aktivität. | — |
| `<boundaryEvent><compensateEventDefinition>` | `BoundaryCompensationEvent` | Registriert einen Kompensationshandler bei erfolgreichem Abschluss der Aktivität. | Wird nicht sofort ausgeführt. |

---

## 2. Aktivitäten

| BPMN-XML | Laufzeittyp | Verhalten | Pflichtangaben |
|---|---|---|---|
| `<userTask>` | `UserTask(assignee)` | Erzeugt einen offenen User Task; Abschluss über die REST-API. | Assignee wird aufgelöst, Default `unassigned`. |
| `<serviceTask>` | `ServiceTask { topic, multi_instance }` | Pausiert und wird von externen Workern per Fetch-and-Lock bearbeitet. | Topic wird aufgelöst, Fallback ist die Node-ID. |
| `<scriptTask>` | `ScriptTask { script }` | Führt Rhai inline aus und läuft weiter. | Nicht-leerer Skriptinhalt. |
| `<sendTask>` | `SendTask { message_name }` | Publiziert eine Nachricht und läuft sofort weiter. | `messageRef` oder `message_name`; Fallback `generic_send`/`generic_throw`. |
| `<receiveTask>` | `MessageCatchEvent { message_name }` | Wartet wie ein Message Catch auf Korrelation. | `messageRef` ist **Pflicht**; ohne ihn schlägt der Deploy fehl. |
| `<manualTask>` | `UserTask(assignee)` | Wird wie ein User Task behandelt — es gibt keine automatische Fortsetzung. | Assignee optional. |
| `<businessRuleTask>` | `ServiceTask { topic }` | Wird als externer Task behandelt. | Topic ist **Pflicht**; es gibt keine DMN-Auswertung. |
| `<task>` (generisch) | — | Wird **abgelehnt**. | Muss in `userTask`, `serviceTask`, `scriptTask` oder `sendTask` umgewandelt werden. |
| `<callActivity calledElement="…">` | `CallActivity` | Ruft eine global deployte Definition auf; Variablen werden propagiert. | `calledElement` muss auf eine deployte Definition zeigen. |
| `<subProcess>` (embedded) | `EmbeddedSubProcess` + `SubProcessEndEvent` | Wird beim Parsen in den Hauptgraphen geflattet; zur Laufzeit gibt es keine verschachtelte Instanz. | Rückkehr über simuliertes `SubProcessEndEvent` im selben Variablen-Scope. |

**Reihenfolge der Attributauflösung:**

| Ziel | Reihenfolge |
|---|---|
| Service-Task-Topic | `data-topic` → `data-handler` → `camunda:topic` → Node-ID |
| User-Task-Assignee | `data-assignee` → `camunda:assignee` → `unassigned` |
| Nachrichtenname | `messageRef` → `<message id name>`; sonst elementspezifischer Fallback |
| Skriptinhalt | inline `<script>` → `data-script` |

---

## 3. Gateways

| BPMN-XML | Laufzeittyp | Split | Join |
|---|---|---|---|
| `<exclusiveGateway>` | `ExclusiveGateway { default }` | Genau ein Pfad mit wahrer Bedingung; optionaler `default`-Flow. | Kein Warten; jeder ankommende Token läuft weiter. |
| `<parallelGateway>` | `ParallelGateway` | Alle ausgehenden Pfade werden genommen. | Wartet auf alle ankommenden Tokens (`JoinBarrier`). |
| `<inclusiveGateway>` | `InclusiveGateway` | Alle Pfade mit wahrer Bedingung; unbedingte Pfade immer. | Erwartet die Anzahl der tatsächlich genommenen Split-Zweige (`expected_count`). Ohne passenden Split fällt der Join auf `incoming_flow_count` zurück (AND-artig). |
| `<eventBasedGateway>` | `EventBasedGateway` | Pausiert, bis genau eines der Ziel-Catch-Ereignisse (Timer oder Nachricht) auslöst. | — |
| `<complexGateway>` | `ComplexGateway { join_condition, default }` | Alle passenden Bedingungen; optionaler Default. | Wartet auf alle Tokens oder löst vorzeitig aus, wenn `activationCondition` über die zusammengeführten Variablen wahr ist. |

**Bedingte Sequenzflüsse:** Operatoren `==`, `!=`, `>`, `>=`, `<`, `<=` sowie Truthy-Prüfungen. Eine Bedingung, die auf einem nicht existierenden Pfad ausgewertet wird, erzeugt `NoMatchingCondition` (`400`).

---

## 4. Listener

| Erweiterung | Verhalten |
|---|---|
| `extensionElements > executionListener event="start"` | Führt ein Rhai-Skript vor der Aktivität aus. |
| `extensionElements > executionListener event="end"` | Führt ein Rhai-Skript nach der Aktivität aus. |

Listener können Variablen lesen und verändern. Die Rhai-Grenzen (`RHAI_MAX_OPERATIONS`, `RHAI_MAX_MEMORY_BYTES`, `RHAI_TIMEOUT_MS`) gelten auch hier; Details in [`reference/konfiguration.md`](konfiguration.md).

Scope-Event-Listener (Timer, Nachricht, Fehler) werden als eigene Event-Sub-Prozesse am Scope registriert, unterbrechend oder nicht-unterbrechend.

---

## 5. Multi-Instance

Multi-Instance wird **nur für `serviceTask`** ausgewertet.

| Variante | Unterstützt | Regel |
|---|---|---|
| Parallele Multi-Instance mit `collection` | ja | Die Collection wird über den Variablennamen aufgelöst; `elementVariable` benennt das Element. |
| Parallele Multi-Instance mit `loopCardinality` | ja | Die Kardinalität muss eine positive Ganzzahl sein. |
| `collection` **und** `loopCardinality` gleichzeitig | nein | `InvalidDefinition` |
| Weder `collection` noch `loopCardinality` | nein | `InvalidDefinition` |
| Sequenzielle Multi-Instance (`isSequential="true"`) | nein | `InvalidDefinition` |
| Multi-Instance an `userTask`, `scriptTask`, `sendTask`, `receiveTask`, `manualTask`, `businessRuleTask` | nein | `InvalidDefinition` |

Die Einschränkung ist eine Laufzeitgrenze, keine Modellierungshilfe: Ein sequenzieller Loop muss als Schleife mit Gateway modelliert oder in mehrere Service Tasks zerlegt werden.

---

## 6. Auflösungsregeln

### 6.1 Timer

Unterstützt werden ISO-8601-Dauern (`PT30S`), absolute Daten, Cron-Ausdrücke und Wiederholungsintervalle. Die konkrete Form steht im `timerEventDefinition`-Body. Ein nicht parsebarer Ausdruck wird abgelehnt.

### 6.2 Nachrichten

`messageRef` verweist auf `<message id="…" name="…">`. Aufgelöst wird die `id` auf den `name`; die Korrelation erfolgt über `messageName` und optional `businessKey`. Fehlt die Auflösung, verwendet der Parser einen elementspezifischen Fallback (`generic_message`, `generic_throw`, `generic_send`).

### 6.3 Eskalationen

`escalationRef` wird über `<escalation id errorCode>` aufgelöst. Ohne Treffer gilt `generic_escalation`.

### 6.4 Fehler (`errorRef`)

Der Parser baut eine Lookup-Tabelle aus **allen** `<bpmn:error>`-Elementen der Datei:

```text
errorRef → <bpmn:error id="…"> → errorCode → name → id
```

- **Treffer:** `BoundaryErrorEvent` erhält den aufgelösten `errorCode` und fängt nur `bpmnError`-Aufrufe mit exakt diesem String.
- **Kein Treffer:** `BoundaryErrorEvent` erhält `error_code = None` und **fängt jeden** `bpmnError`-Code an dieser Aktivität. Das ist ein Catch-all, kein Deploy-Fehler.
- **Mehrere Boundary Error Events** an derselben Aktivität: Der Parser nimmt das erste passende Element in Iterationsreihenfolge. Ein Catch-all kann damit ein spezifischeres Ereignis verdecken.

Deshalb ist die explizite Deklaration empfohlen:

```xml
<bpmn:error id="err_invalid_address" errorCode="INVALID_ADDRESS" />
…
<bpmn:boundaryEvent id="email-fehler" attachedToRef="send-email">
  <bpmn:errorEventDefinition errorRef="err_invalid_address" />
</bpmn:boundaryEvent>
```

Der Worker sendet dann `{"errorCode":"INVALID_ADDRESS"}`. Der vollständige Ablauf steht in [`how-to/external-task-worker-schreiben.md`](../how-to/external-task-worker-schreiben.md).

---

## 7. Ablehnungsfälle

Diese Fälle lassen den Deploy mit `InvalidDefinition` (`400`) scheitern:

| Fall | Meldung (gekürzt) |
|---|---|
| Leerer `scriptTask` | `scriptTask '<id>' has an empty script` |
| `businessRuleTask` ohne Topic | `businessRuleTask '<id>' has no topic …; DMN is not supported` |
| Generisches `<task>` | `unsupported element <task id='…'>: convert to userTask, serviceTask, scriptTask, or sendTask` |
| `receiveTask` ohne `messageRef` | `receiveTask '<id>' has no messageRef` |
| `intermediateCatchEvent` ohne Timer/Message | `intermediateCatchEvent '<id>' has no timerEventDefinition or messageEventDefinition` |
| Sequenzielle Multi-Instance | `Sequential multi-instance is not supported yet` |
| Multi-Instance ohne Collection und ohne Kardinalität | `Multi-instance element '<id>' requires either collection or loopCardinality` |
| Multi-Instance mit beiden Angaben | `Multi-instance element '<id>' must define either collection or loopCardinality, not both` |
| Nicht-numerische `loopCardinality` | `… must be a positive integer` |
| Bedingung ohne passenden Flow | Laufzeitfehler `NoMatchingCondition` (`400`), kein Deploy-Fehler |

---

## 8. Nicht unterstützte Elemente

Die folgenden Elemente werden **nicht** ausgewertet. Je nach Einbettung ignoriert der Parser sie oder der Deploy scheitert:

| Element | Verhalten |
|---|---|
| `signalEventDefinition` (Start, Intermediate, Boundary) | Nicht unterstützt; das Ereignis wird nicht ausgelöst. |
| `cancelEventDefinition` | Nicht unterstützt. |
| `linkEventDefinition` | Nicht unterstützt. |
| Transaction Sub-Process | Nicht unterstützt. |
| Ad-Hoc Sub-Process | Nicht unterstützt. |
| Data Objects, Data Stores, Data Input/Output Associations | Werden ignoriert. Datenaustausch erfolgt ausschließlich über den JSON-Variablenzustand. |
| DMN / Decision Tasks | Es gibt keine DMN-Engine. `businessRuleTask` ist ein externer Task. |
| `multiInstanceLoopCharacteristics` außerhalb von `serviceTask` | `InvalidDefinition` (Abschnitt 5). |

Die vollständige Abweichungsliste gegenüber BPMN 2.0 und Camunda 7 steht in [`reference/kompatibilitaet.md`](kompatibilitaet.md).

---

## 9. Belegstellen

| Aussage | Quelle |
|---|---|
| Element-Enum und Laufzeitfelder | `engine-core/src/domain/element.rs` |
| Parsing und Attributauflösung | `bpmn-parser/src/parser.rs` |
| XML-Modell und `errorRef`-Feld | `bpmn-parser/src/models.rs` |
| Fehlerauflösung und Catch-all | `bpmn-parser/src/parser.rs` (`error_lookup`), `engine-core/src/engine/executor/helpers.rs` (`find_boundary_error_event`) |
| Ablehnung von Multi-Instance | `bpmn-parser/src/parser.rs` (`parse_multi_instance`, `reject_multi_instance`) |

## Zugehörige Seiten

| Frage | Seite |
|---|---|
| Wie ein Modell deployt und eine zweite Version erzeugt wird | [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md) |
| Wie sich ein Element gegenüber Camunda 7 verhält | [`reference/kompatibilitaet.md`](kompatibilitaet.md) |
| Wie ein Worker einen Fehler meldet, den der Boundary Event fängt | [`how-to/external-task-worker-schreiben.md`](../how-to/external-task-worker-schreiben.md) |
| Warum Sub-Prozesse geflattet werden und was das kostet | [`explanation/konzepte.md`](../explanation/konzepte.md) |
| Welche Statuscodes ein abgelehnter Deploy liefert | [`reference/rest-api.md`](rest-api.md) |

## Offen

- **Incident-Art bei ungefangenem Error End Event:** In Abschnitt 1.2 steht nur die Code-Auflösung. Ob ein ungefangenes Error End Event einen Incident erzeugt und ob dieser die Art `bpmn_error` trägt, ist **nicht belegt**: Der Enum-Wert existiert, der Auslöser nicht. **Offen:** mit einem Live-Lauf klären.
- **README-Abweichungen:** `README.md` beschreibt `receiveTask`, `manualTask`, `businessRuleTask` und generische `<task>` pauschal als `ServiceTask` und einen leeren `scriptTask` als Service Task. Der Parser (Stand 0.7.24) behandelt sie abweichend: `receiveTask` → `MessageCatchEvent`, `manualTask` → `UserTask`, generisches `<task>` → Ablehnung, leerer `scriptTask` → Ablehnung. Diese Seite folgt dem Parser. **Offen:** README angleichen.
- **Mehrere Boundary Error Events:** Die Auswahl „erstes passendes Element" ist aus der Iterationsreihenfolge abgeleitet, nicht durch einen Test belegt. **Offen:** Prioritätsregel mit einem Parser-Test absichern.
- **Timer-Cron:** Die Unterstützung von Cron-Ausdrücken ist aus README und Parser-Funktionen abgeleitet; ein Beispiel je Variante fehlt auf dieser Seite.
- **Elementanzahl:** Diese Seite erhebt keinen Anspruch auf Vollständigkeit gegenüber der BPMN-2.0-Spezifikation. Sie listet die Elemente, die der Parser kennt oder ausdrücklich ablehnt.
