---
title: External-Task-Worker schreiben und testen
description: Einen Worker gegen die BPMNinja-Service-Task-API bauen, der Tasks per fetchAndLock abholt, abschließt, bei technischem Fehler einen Incident erzeugt und bei fachlichem Fehler in ein Boundary Error Event läuft — mit Lock-Dauer in Millisekunden und einer Prüfliste am Ende.
audience:
  - Entwickler und Integratoren
  - Betriebsteams und Plattform-Administratoren
prerequisites:
  - Laufende BPMNinja-Engine unter http://localhost:8081 (siehe tutorials/erster-prozess.md)
  - Laufender NATS-JetStream-Broker (docker compose up -d nats)
  - Node.js 20 oder neuer mit npm
  - curl installiert
  - Grundkenntnisse in BPMN 2.0 (Service Task, Boundary Error Event)
content_type: how-to
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/service-task-api.md
  - reference/rest-api.md
  - reference/cli.md
  - tutorials/erster-prozess.md
  - how-to/konfiguration-anpassen.md
  - explanation/konzepte.md
  - _review/glossary.md
---

# External-Task-Worker schreiben und testen

## Aufgabe

Du willst einen **Service Task** in einem BPMN-Prozess durch eigenen Code ausführen lassen, statt ihn in der Engine laufen zu lassen. Dazu baust du einen Worker, der Tasks per Fetch-and-Lock abholt, verarbeitet und das Ergebnis an die Engine zurückmeldet — inklusive der beiden Fehlerpfade, die im Betrieb zählen: **technischer Fehler** (Retry, dann Incident) und **fachlicher Fehler** (`bpmnError`, läuft in ein Boundary Error Event).

> **Wo du gerade einsteigst:** Diese Seite ist Schritt 2 des Lesepfads B („Ich will einen Worker bauen"). Voraussetzung ist das Tutorial [`tutorials/erster-prozess.md`](../tutorials/erster-prozess.md). Danach folgen [`reference/service-task-api.md`](../reference/service-task-api.md) und [`reference/rest-api.md`](../reference/rest-api.md). Alle Pfade: [Dokumentationslandingpage](../README.md#lesepfade).

## Voraussetzungen

- Die Engine läuft und ist erreichbar: `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/health` liefert `200` (leerer Body). Wenn nicht, führe zuerst [`tutorials/erster-prozess.md`](../tutorials/erster-prozess.md) aus.
- NATS JetStream läuft (`docker compose ps nats` zeigt `running`).
- Node.js ≥ 20 und npm sind installiert (`node --version`).
- Du hast ein Terminal im Projektverzeichnis und ein zweites Terminal für `curl`-Aufrufe.
- Du hast das Repository lokal ausgecheckt — der Client wird aus dem Repository gebaut (Schritt 3).
- Du kennst den Unterschied zwischen **Service Task** (BPMN-Element) und **External Task** (die Arbeitseinheit, die daraus in der Fetch-and-Lock-Queue entsteht). Kurzfassung: Der Service Task ist der Knoten im Modell, der External Task ist der Eintrag, den ein Worker sperrt und abarbeitet. Ausführlich: [`explanation/konzepte.md`](../explanation/konzepte.md).

> **Auth-Hinweis:** Wenn du die Engine mit `BPMNINJA_API_KEY` oder `BPMNINJA_WORKER_API_KEY` betreibst, muss der Worker den Key bei **jedem** Request senden (`Authorization: Bearer <key>` oder `X-API-Key: <key>`). Ein Worker-Key darf nur `fetchAndLock`, `complete`, `failure`, `extendLock` und `bpmnError`; alles andere antwortet mit **403**. Details: [`reference/rest-api.md`](../reference/rest-api.md#rollenmatrix-für-service-task-routen).

## Erwartetes Ergebnis

Am Ende dieses Guides hast du belegt:

1. Ein Worker holt einen Task per `fetchAndLock` und schließt ihn mit `complete` ab — die Instanz läuft weiter.
2. Ein `failure` mit `retries: 0` erzeugt einen **Incident** an der Instanz, der per `GET /api/instances/{id}/incidents` sichtbar ist.
3. Ein `bpmnError` mit passendem `errorCode` läuft in den **Boundary Error Event** und damit auf den alternativen Pfad.
4. Du sendest `lockDuration` und `newDuration` in **Millisekunden** — nicht in Sekunden.

---

## Schritt 1: Prozessmodell mit Service Task und Boundary Error Event anlegen

Lege eine Datei `worker-prozess.bpmn` an. Sie enthält einen Service Task mit Topic `send-email`, einen Boundary Error Event für den Fehlercode `INVALID_ADDRESS` und einen zweiten Service Task als Fehlerpfad:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions
    xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"
    targetNamespace="http://bpmninja.local/howto">
  <bpmn:error id="err_invalid_address" errorCode="INVALID_ADDRESS" />
  <bpmn:process id="worker-prozess" name="Worker-Prozess" isExecutable="true">
    <bpmn:startEvent id="start" />
    <bpmn:serviceTask id="send-email" name="E-Mail senden" data-topic="send-email" />
    <bpmn:boundaryEvent id="email-fehler" attachedToRef="send-email">
      <bpmn:errorEventDefinition errorRef="err_invalid_address" />
    </bpmn:boundaryEvent>
    <bpmn:serviceTask id="adresse-korrigieren" name="Adresse korrigieren" data-topic="fix-address" />
    <bpmn:endEvent id="ende-ok" />
    <bpmn:endEvent id="ende-fehler" />
    <bpmn:sequenceFlow id="f1" sourceRef="start" targetRef="send-email" />
    <bpmn:sequenceFlow id="f2" sourceRef="send-email" targetRef="ende-ok" />
    <bpmn:sequenceFlow id="f3" sourceRef="email-fehler" targetRef="adresse-korrigieren" />
    <bpmn:sequenceFlow id="f4" sourceRef="adresse-korrigieren" targetRef="ende-fehler" />
  </bpmn:process>
</bpmn:definitions>
```

Das Topic wird in dieser Reihenfolge gelesen: `data-topic`, dann `data-handler`, dann `camunda:topic`, dann die Node-ID. Mit `data-topic="send-email"` ist das Topic eindeutig `send-email`.

> **Fehlerauflösung:** Der Parser baut eine Lookup-Tabelle aus allen `<bpmn:error>`-Elementen (`id` → `errorCode` → `name` → `id`). Das Beispiel deklariert deshalb `err_invalid_address` mit `errorCode="INVALID_ADDRESS"` und verweist darauf. **Ohne passendes `<bpmn:error>`** erhält das Boundary Event `error_code = None` und fängt jeden `bpmnError`-Code an dieser Aktivität — ein Catch-all, kein exakter Match. Details: [`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md).

**Erwartetes Ergebnis:** Die Datei ist wohlgeformtes XML. Prüfe das mit `xmllint --noout worker-prozess.bpmn` (falls installiert; der Deploy in Schritt 2 validiert das XML ohnehin).

---

## Schritt 2: Prozess deployen und Instanz starten

Deploye das Modell:

```bash
jq -Rs '{name:"worker-prozess", xml:.}' worker-prozess.bpmn > deploy.json
curl -s -X POST http://localhost:8081/api/deploy \
  -H "Content-Type: application/json" \
  --data-binary @deploy.json
```

Erwartete Ausgabe (HTTP 200):

```json
{"definition_key":"7c1d2e3f-4a5b-4c6d-8e9f-0a1b2c3d4e5f","version":1}
```

Starte eine Instanz und übergib die E-Mail-Adresse als Variable:

```bash
curl -s -X POST http://localhost:8081/api/start \
  -H "Content-Type: application/json" \
  -d '{"definition_key":"7c1d2e3f-4a5b-4c6d-8e9f-0a1b2c3d4e5f","variables":{"email":"kunde@example.com"}}'
```

Erwartete Ausgabe:

```json
{"instance_id":"11111111-2222-4333-8444-555555555555"}
```

Notiere die `instance_id`. Den Wartezustand liest du mit `GET /api/instances/{id}`; er zeigt `{"WaitingOnServiceTask":{"task_id":"…"}}`. Der Zustand ist korrekt: Die Engine hat den Task in die Fetch-and-Lock-Queue gestellt und wartet auf einen Worker.

**Erwartetes Ergebnis:** Ein Eintrag in der Service-Task-Liste ist sichtbar:

```bash
curl -s http://localhost:8081/api/service-tasks
```

Die Antwort enthält einen Task mit `topic: "send-email"`. Der Endpunkt ist **Admin-only**: Läuft die Engine mit gesetztem Worker-Key, antwortet er mit `403`; prüfe den Task dann mit Admin-Key.

---

## Schritt 3: Worker-Projekt anlegen und Client bereitstellen

Lege ein Verzeichnis `worker/` an. Den Client baust du **aus dem Repository** — ein veröffentlichtes Registry-Paket ist nicht belegt:

```bash
mkdir -p worker && cd worker
npm init -y

# Client aus dem Repository bauen (belegter Weg, siehe reference/cli.md §6)
cd ../bpmn-ninja-external-task-client
npm install
npm run build

# Anschließend im Worker-Projekt als lokale Abhängigkeit einbinden
cd ../worker
npm install ../bpmn-ninja-external-task-client
```

> **Warnung — Registry-Paket nicht belegt:** `npm install @bpmninja/external-task-client` setzt ein veröffentlichtes Paket voraus. Weder Existenz noch Version noch Export-Signatur dieses Pakets sind gegen die npm-Registry geprüft (`_qa/verification-report.md` §5). Verwende deshalb den Repository-Build oben. Prüfe vor dem ersten Lauf das `name`-Feld in `bpmn-ninja-external-task-client/package.json`; der Import in Schritt 4 muss exakt diesem Namen entsprechen.

> **Offen:** Der Import-Name im folgenden Beispiel (`@bpmninja/external-task-client`) ist der in der Knowledge Base dokumentierte Paketname. Er ist nicht gegen ein veröffentlichtes Paket geprüft. Die verwendete API (`ExternalTaskClient`, `workerId`, `maxRetries`, `lockDuration`, `asyncResponseTimeout`, `service.complete`, `service.failure`, `service.bpmnError`, `service.extendLock`) ist in `docs/knowledge-base/modules/external-task-client/interfaces.md` belegt.

---

## Schritt 4: Worker-Code schreiben

Lege `worker/worker.mjs` an:

```javascript
import { ExternalTaskClient } from "@bpmninja/external-task-client";

const client = new ExternalTaskClient({
  baseUrl: "http://localhost:8081",
  workerId: "howto-worker-01",
  // Optional: apiKey: process.env.BPMNINJA_WORKER_API_KEY,
  maxRetries: 3,                 // Standard: 3 — gilt global pro Client, nicht pro Task
  lockDuration: 30_000,          // Millisekunden — NICHT 30
  asyncResponseTimeout: 10_000,  // Long-Polling, Millisekunden
  autoExtendLock: true,          // verlängert die Sperre bei 80 % von lockDuration
});

client.subscribe("send-email", async (task, service) => {
  const email = task.variables_snapshot?.email;

  // Fachlicher Fehler: Adresse ist syntaktisch ungültig.
  if (typeof email !== "string" || !email.includes("@")) {
    // bpmnError wird NICHT wiederholt; es läuft in den Boundary Error Event
    // mit passendem errorCode (hier: INVALID_ADDRESS).
    await service.bpmnError("INVALID_ADDRESS");
    return;
  }

  try {
    // Hier steht deine echte Arbeit. Sie muss idempotent sein:
    // Der Lock kann ablaufen, ein zweiter Worker kann denselben Task holen.
    await sendMail(email);
    await service.complete({ gesendet: true });
  } catch (err) {
    // Technischer Fehler: Die Engine dekrementiert retries; bei 0 entsteht ein Incident.
    // Der dritte Parameter setzt die ENGINE-Retries, nicht die Client-Retries.
    await service.failure(err.message, String(err.stack ?? ""), 0);
  }
});

client.subscribe("fix-address", async (task, service) => {
  await service.complete({ adresse_korrigiert: true });
});

async function sendMail(email) {
  // Platzhalter für den echten Versand.
  console.log(`sende an ${email}`);
}

client.start();
console.log("Worker gestartet, Topic: send-email, fix-address");
```

**Wichtige Eigenschaften dieses Codes:**

- **Handler-Signatur:** `(task: Task, service: TaskService) => Promise<void>`. Die beiden Parameter kommen **einzeln**, nicht als Objekt. Ein `async ({ task, service }) => …` würde zwei `undefined`-Werte destrukturieren und beim ersten Zugriff mit `TypeError` abbrechen.
- **Variablenfeld:** `task.variables_snapshot` ist das belegte Feld im `Task`-Objekt. Ein `task.variables` existiert nicht und liefert immer `undefined` — der Worker würde dann jede Adresse als ungültig behandeln, ohne dass ein Fehler sichtbar wird.
- **`lockDuration: 30_000`** ist die Sperre in Millisekunden. Der Client sendet sie unverändert an die Engine; die Engine rechnet intern in Sekunden um.
- **`autoExtendLock: true`** verlängert die Sperre automatisch bei 80 % von `lockDuration`. Das ersetzt die manuelle `extendLock`-Pflege, nicht die Idempotenz.
- **`service.bpmnError("INVALID_ADDRESS")`** wird nicht wiederholt. Der Fehlercode muss zum aufgelösten `errorCode` des Boundary Error Event passen. Ohne Treffer bleibt der Fehler ungefangen: Es entsteht ein Incident der Art **`service_task`** mit der Meldung `Unhandled BPMN error '<CODE>'` (Live-Lauf vom 2026-10-24, `_qa/verification-report.md` §2.3).
- **`service.failure(msg, details, 0)`** setzt die von der Engine geführten `retries` auf 0 und erzeugt sofort einen Incident. Der dritte Parameter steuert die Engine-Retries; die Client-Retries (`maxRetries`) gelten global pro Client und sind davon unabhängig.
- **Idempotenz:** Der Worker kann denselben Task mehrfach sehen, wenn die Sperre abläuft, bevor `complete` ankommt. `sendMail` muss deshalb so gebaut sein, dass ein zweiter Aufruf keinen doppelten Effekt hat (z. B. über eine Message-ID oder einen Datenbank-Constraint). Warum das keine Option ist: [`explanation/konzepte.md`](../explanation/konzepte.md).

**Erwartetes Ergebnis:** `node worker/worker.mjs` startet und gibt `Worker gestartet, Topic: send-email, fix-address` aus. Der Worker pollt jetzt per Long-Polling.

---

## Schritt 5: Erfolgspfad testen

Starte den Worker in einem Terminal:

```bash
cd worker && node worker.mjs
```

Starte in einem zweiten Terminal eine neue Instanz mit gültiger Adresse:

```bash
curl -s -X POST http://localhost:8081/api/start \
  -H "Content-Type: application/json" \
  -d '{"definition_key":"7c1d2e3f-4a5b-4c6d-8e9f-0a1b2c3d4e5f","variables":{"email":"kunde@example.com"}}'
```

**Erwartetes Ergebnis:** Das Worker-Terminal zeigt `sende an kunde@example.com`. Die Instanz ist abgeschlossen:

```bash
curl -s http://localhost:8081/api/instances/<instance_id>
```

```json
{"id":"...","state":"Completed","variables":{"email":"kunde@example.com","gesendet":true}}
```

---

## Schritt 6: Technischen Fehler und Incident testen

Ändere `sendMail` so, dass sie wirft:

```javascript
async function sendMail(email) {
  throw new Error("SMTP nicht erreichbar");
}
```

Starte den Worker neu und starte eine neue Instanz mit gültiger Adresse. Der Worker ruft `service.failure(..., 0)` auf.

**Erwartetes Ergebnis:** Die Instanz bleibt im Zustand `WaitingOnServiceTask`, und ein Incident ist sichtbar:

```bash
curl -s http://localhost:8081/api/instances/<instance_id>/incidents
```

```json
[
  {
    "id": "51b97a7b-fd1e-4082-956f-0678b1aeef6c",
    "kind": "service_task",
    "node_id": "send-email",
    "task_id": "c2251b91-a72b-40a9-86ac-a22579e16b9c",
    "message": "SMTP nicht erreichbar",
    "details": "Error: SMTP nicht erreichbar\n    at sendMail …",
    "created_at": "2026-10-24T11:37:02.107853Z",
    "occurrences": 1,
    "resolved_at": null,
    "resolved_by": null
  }
]
```

Der Task ist ab jetzt **nicht mehr fetchbar** (`retries <= 0`), bis du ihn per `POST /api/service-task/{id}/retry` (Retries zurücksetzen) oder `POST /api/service-task/{id}/resolve` (manuell abschließen) behandelst. Beide Endpunkte sind **Admin-only** — ein Worker-Key erhält **403**. Vertrag: [`reference/service-task-api.md`](../reference/service-task-api.md).

---

## Schritt 7: Fachlichen Fehler über Boundary Error Event testen

Setze `sendMail` zurück auf die funktionierende Variante und starte eine neue Instanz mit ungültiger Adresse:

```bash
curl -s -X POST http://localhost:8081/api/start \
  -H "Content-Type: application/json" \
  -d '{"definition_key":"7c1d2e3f-4a5b-4c6d-8e9f-0a1b2c3d4e5f","variables":{"email":"keine-adresse"}}'
```

**Erwartetes Ergebnis:** Der Worker ruft `bpmnError("INVALID_ADDRESS")` auf. Die Engine verlässt den Service Task über den Boundary Error Event und erreicht den zweiten Service Task `fix-address`. Der Worker schließt ihn ab. Die Instanz endet am `ende-fehler`-Pfad:

```bash
curl -s http://localhost:8081/api/instances/<instance_id>
```

```json
{"id":"...","state":"Completed","variables":{"email":"keine-adresse","adresse_korrigiert":true}}
```

Es entsteht **kein** Incident, weil der Fehler gefangen wurde.

---

## Woran du erkennst, dass du fertig bist

Belege alle vier Abnahmekriterien in einem Durchlauf:

| Kriterium | Prüfung | Erwartet |
|---|---|---|
| Task wird verarbeitet | Schritt 5 | Instanz `Completed`, Variable `gesendet: true` |
| `failure` mit `retries: 0` erzeugt Incident | Schritt 6 | `GET /api/instances/{id}/incidents` liefert Eintrag mit `kind: "service_task"` |
| `bpmnError` läuft in Boundary Error Event | Schritt 7 | Instanz `Completed` über `fix-address`, kein Incident |
| Lock-Dauer in ms | `fetchAndLock`-Request im Netzwerk-Log | `lockDuration: 30000`, nicht `30` |

Zusätzlich: Prüfe, dass ein `lockDuration: 30` (Sekunden-Fehler) mit **400** abgelehnt wird:

```bash
curl -s -o /dev/null -w "%{http_code}\n" -X POST http://localhost:8081/api/service-task/fetchAndLock \
  -H "Content-Type: application/json" \
  -d '{"workerId":"test","maxTasks":1,"topics":[{"topicName":"send-email","lockDuration":30}]}'
```

Erwartete Ausgabe: `400`.

---

## Typische Fehler

### Fehler 1: `fetchAndLock` antwortet mit 400 und der Worker holt nie Tasks

**Symptom:** Der Worker läuft, aber es kommen keine Tasks an. Im Log steht ein 400 auf `fetchAndLock`.

**Ursachen:**

1. **`lockDuration` außerhalb 1 000..=3 600 000 ms.** Der häufigste Fall ist `30` statt `30000`. Der Server validiert vor dem Long-Poll und antwortet sofort mit 400.
2. **`maxTasks` außerhalb 1..=100** oder **`topics` leer bzw. mehr als 32 Einträge.** Ebenfalls 400.
3. **Topic-Schreibweise weicht ab.** Das Topic im Worker muss exakt dem `data-topic` im BPMN entsprechen (Groß-/Kleinschreibung zählt). Prüfe das Topic mit `curl -s http://localhost:8081/api/service-tasks`.

**Behebung:** Korrigiere die Werte im Client-Konstruktor. Setze `lockDuration` auf einen Wert zwischen `1000` und `3600000`.

### Fehler 2: `complete` antwortet mit 409 oder der Task wird doppelt bearbeitet

**Symptom:** Zwei Worker-Läufe für denselben Task, oder `complete` schlägt fehl, weil der Task bereits abgeschlossen ist.

**Ursache:** Die Sperre ist abgelaufen, bevor der Worker `complete` aufgerufen hat. Ein zweiter Worker hat den Task in der Zwischenzeit geholt. Das ist kein Fehler der Engine, sondern eine Folge von `lockDuration` < Bearbeitungsdauer.

**Behebung:** Erhöhe `lockDuration` oder aktiviere `autoExtendLock: true` in der Client-Konfiguration — der Client verlängert die Sperre dann bei 80 % von `lockDuration`. Unabhängig davon muss der Handler idempotent sein: Ein zweiter Lauf darf keinen doppelten Effekt haben.

### Fehler 3: `bpmnError` erzeugt einen Incident statt in den Boundary-Pfad zu laufen

**Symptom:** Die Instanz bleibt stehen, und `GET /api/instances/{id}/incidents` zeigt einen Eintrag mit `kind: "service_task"` und der Meldung `Unhandled BPMN error '<CODE>'`.

**Ursache:** Der `errorCode` im `bpmnError`-Aufruf passt zu keinem `<bpmn:error>`, auf das ein Boundary Error Event am Service Task verweist — der aufgelöste `errorCode` stimmt nicht überein. Der Fehler bleibt damit ungefangen.

**Behebung:** Vergleiche den String im `bpmnError`-Aufruf zeichengenau mit dem `errorCode` des referenzierten `<bpmn:error>`. Prüfe zusätzlich, ob der Boundary Event mit `attachedToRef` am richtigen Service Task hängt. Die Auflösung ist in Schritt 1 und in [`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md) dokumentiert.

---

## Nächster sinnvoller Schritt

Du hast alle drei Ergebniswege eines External Task belegt. Der nächste Schritt ist der Vertrag dahinter — die genauen Statuscodes, Validierungsgrenzen und die Lock-Semantik:

→ **[`reference/service-task-api.md`](../reference/service-task-api.md)** — kanonische Definition der Lock-Dauer, `fetchAndLock`-Validierung, `complete`/`failure`/`bpmnError`/`extendLock`/`retry`/`resolve`.

Wenn du den Worker in einer Umgebung mit Keys betreiben willst, lies:

→ **[`how-to/konfiguration-anpassen.md`](../how-to/konfiguration-anpassen.md)** — Admin- und Worker-Key trennen, Rollen prüfen, fail-closed-Verhalten verstehen.

Wenn du verstehen willst, warum die Engine At-least-once garantiert und was das für deinen Code bedeutet, lies:

→ **[`explanation/konzepte.md`](../explanation/konzepte.md)** — Service Task vs. External Task, Idempotenz und Incident-Modell.

---

## Offen

- **Registry-Paket:** Ein veröffentlichtes Paket `@bpmninja/external-task-client` ist nicht belegt. Schritt 3 baut den Client deshalb aus dem Repository (`bpmn-ninja-external-task-client`); der Import in Schritt 4 folgt dem `name`-Feld des Repository-Pakets und ist vor dem ersten Lauf zu prüfen. **Offen:** Registry-Nachweis (`npm view … version`) oder dauerhafte Dokumentation des Repository-Builds.
- **`autoExtendLock`:** Das 80-%-Verhalten ist in der Client-Dokumentation belegt, aber nicht gegen einen Live-Lauf verifiziert.
- **`GET /api/service-tasks` in Schritt 2:** Der Endpunkt ist Admin-only. Läuft die Engine mit gesetztem Worker-Key, antwortet er mit `403`; prüfe den Task dann mit Admin-Key.
- **Ankerlinks:** Die internen Verweise auf `#fehler-1-…` bis `#fehler-3-…` zeigen auf Überschriften mit Umlauten und Sonderzeichen. Die Linkprüfung prüft Anker nicht. **Offen:** Ankerauflösung im Zielsystem einmalig prüfen.
