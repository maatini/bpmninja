---
title: Ersten Prozess ausführen
description: Deploye ein minimales BPMN-2.0-Modell (Start → User Task → End), starte eine Instanz, schließe den User Task ab und beobachte den Zustand Completed — mit curl, erwarteter Ausgabe nach jedem Schritt und einem Abnahmekriterium am Ende.
audience:
  - Einsteiger und Lernende
  - Entwickler und Integratoren
  - Betriebsteams und Plattform-Administratoren
  - Stakeholder und Entscheider
prerequisites:
  - Rust-Toolchain (cargo) installiert
  - Docker und Docker Compose installiert
  - curl installiert
  - Ein Terminal mit Zugriff auf das Repository
content_type: tutorial
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/rest-api.md
  - reference/service-task-api.md
  - reference/cli.md
  - explanation/architektur.md
  - explanation/konzepte.md
  - how-to/external-task-worker-schreiben.md
  - how-to/prozess-deployen.md
  - how-to/konfiguration-anpassen.md
  - _review/glossary.md
---

# Ersten Prozess ausführen

In diesem Tutorial bringst du einen BPMN-2.0-Prozess von der XML-Datei bis zum sichtbaren Abschluss. Du deployst ein Modell mit drei Elementen, startest eine Instanz, schließt den User Task ab und prüfst, dass die Instanz den Zustand `Completed` erreicht hat.

Das Tutorial ist eine Lektion mit einem garantierten Ergebnis. Es zeigt keine Alternativen und keine Optionsmatrizen. Wenn du ein eigenes Modell deployen und Versionen verwalten willst, ist das kein Tutorial-Stoff mehr — dafür gibt es den How-to-Guide [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md).

> **Wo du gerade einsteigst:** Diese Seite ist der erste Schritt des Lesepfads A („Ich will es einmal laufen sehen"). Danach folgen [`explanation/konzepte.md`](../explanation/konzepte.md) und [`explanation/architektur.md`](../explanation/architektur.md). Der Überblick über alle Pfade steht auf der [Dokumentationslandingpage](../README.md#lesepfade).

## Lernziele

Nach diesem Tutorial kannst du:

- erklären, was eine **Process Definition** und was eine **Process Instance** ist,
- ein minimales BPMN-2.0-XML mit `curl` deployen,
- eine Instanz starten und ihre `instance_id` aus der Antwort übernehmen,
- einen User Task abschließen und den Zustandswechsel von `WaitingOnUserTask` nach `Completed` nachvollziehen,
- die zwei häufigsten Fehler beim ersten Deploy und beim ersten Start selbst beheben.

## Voraussetzungen

- **Rust-Toolchain** mit `cargo` (das Repository baut über das Workspace-Manifest).
- **Docker** und **Docker Compose** für den NATS-JetStream-Broker.
- **curl** für die HTTP-Aufrufe; für den Deploy zusätzlich **jq**.
- Ein Terminal im Repository-Wurzelverzeichnis.
- Die Ports **8081** (Engine) und **4222** (NATS) sind lokal frei.

> **Hinweis zur Sicherheit:** Die Engine bindet standardmäßig an `0.0.0.0` (`BIND_ADDR`). Ohne gesetzten API-Key antworten alle `/api`-Routen dann mit **401** (fail-closed). Für dieses Tutorial setzt du `BIND_ADDR=127.0.0.1`, damit die Engine nur lokal erreichbar ist und ohne Key antwortet. Details: [`how-to/konfiguration-anpassen.md`](../how-to/konfiguration-anpassen.md#schritt-2-bind-adresse-bewusst-setzen).

## Geschätzte Dauer

**10 bis 15 Minuten**, davon etwa 3 bis 5 Minuten für den ersten `cargo`-Build.

## Begriffe, die du in diesem Tutorial brauchst

| Begriff | Bedeutung in diesem Tutorial |
|---|---|
| **Process Definition** | Das deployte, unveränderliche Prozessmodell. Es entsteht aus deiner XML-Datei und bekommt eine `definition_key` (UUID) und eine `version`. |
| **Process Instance** | Ein konkreter Lauf einer Definition mit eigenem Variablenzustand und eigenen Tokens. |
| **Token** | Die Ausführungsmarke, die genau eine Position im Prozessgraph repräsentiert. Bei einem linearen Prozess gibt es genau ein Token. |
| **User Task** | Ein Wait State: Die Engine hält an, persistiert den Zustand und wartet, bis du den Task über `POST /api/complete/{id}` abschließt. |

Die vollständige Abgrenzung — insbesondere Definition ↔ Instance und Service Task ↔ External Task — steht im [Glossar](../_review/glossary.md) und in [`explanation/konzepte.md`](../explanation/konzepte.md).

---

## Schritt 1: NATS mit JetStream starten

Die Engine braucht NATS JetStream als Persistenz. Starte den Broker aus dem Repository:

```bash
docker compose up -d nats
```

**Erwartetes Ergebnis:** Docker meldet einen gestarteten Container. Prüfe, dass NATS auf dem Loopback-Port erreichbar ist:

```bash
docker compose ps nats
```

Die Ausgabe enthält eine Zeile für den `nats`-Service mit dem Status `running` (oder `Up`) und der Portzuordnung `127.0.0.1:4222->4222/tcp`.

> **Warum Loopback?** Die mitgelieferte Compose-Datei veröffentlicht NATS nur auf `127.0.0.1`. Das ist beabsichtigt und kein Fehler.

---

## Schritt 2: Engine-Server starten

Starte den Server im Vordergrund und binde ihn nur an Loopback:

```bash
BIND_ADDR=127.0.0.1 cargo run -p engine-server
```

Der erste Build dauert einige Minuten. Lass das Terminal offen — der Server läuft, bis du ihn mit `Strg+C` beendest. Alle weiteren Befehle führst du in einem **zweiten** Terminal aus.

**Erwartetes Ergebnis:** Der Server lauscht auf `http://localhost:8081`. Prüfe das in einem zweiten Terminal:

```bash
curl -s http://localhost:8081/api/health
```

Erwartete Ausgabe: **HTTP 200 mit leerem Body** — der Liveness-Endpunkt bestätigt den laufenden Prozess über den Statuscode, nicht über ein JSON-Objekt. Prüfe den Code explizit:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8081/api/health
```

Erwartet: `200`.

Prüfe zusätzlich die Readiness — sie sagt aus, ob die Engine Zustand sicher halten kann:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8081/api/ready
```

Erwartete Ausgabe:

```text
200
```

Wenn hier `503` steht, hat die Engine keine funktionierende Persistenz. Siehe [Fehlerbehebung: Fehler 2](#fehler-2-apiready-antwortet-mit-503-oder-der-server-startet-nicht).

---

## Schritt 3: Prozessmodell als Datei anlegen

Lege eine Datei `erster-prozess.bpmn` im aktuellen Verzeichnis an:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions
    xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    targetNamespace="http://bpmninja.local/tutorial">
  <bpmn:process id="erster-prozess" name="Erster Prozess" isExecutable="true">
    <bpmn:startEvent id="start" name="Start" />
    <bpmn:userTask id="bestaetigen" name="Bestätigen" data-assignee="tutorial-leser" />
    <bpmn:endEvent id="ende" name="Ende" />
    <bpmn:sequenceFlow id="flow-1" sourceRef="start" targetRef="bestaetigen" />
    <bpmn:sequenceFlow id="flow-2" sourceRef="bestaetigen" targetRef="ende" />
  </bpmn:process>
</bpmn:definitions>
```

Das Modell enthält genau drei Elemente: einen Start Event ohne Trigger, einen User Task und einen End Event. Der User Task trägt `data-assignee="tutorial-leser"`; ohne dieses Attribut fällt der Parser auf `camunda:assignee` und dann auf `"unassigned"` zurück.

> **Wichtig:** BPMNinja deployt nur **ein** `<process>` pro Datei — das erste mit `isExecutable="true"`, sonst das erste überhaupt. Weitere Prozesse in derselben Datei werden ignoriert.

**Erwartetes Ergebnis:** Die Datei existiert und ist wohlgeformtes XML. Prüfe das mit:

```bash
xmllint --noout erster-prozess.bpmn && echo "XML ist wohlgeformt"
```

Wenn `xmllint` nicht installiert ist, überspringe diese Prüfung — der Deploy in Schritt 4 validiert das XML ohnehin.

---

## Schritt 4: Prozess deployen

Verpacke die XML-Datei in den JSON-Vertrag von `POST /api/deploy`. Der Endpunkt erwartet `application/json` mit den Feldern `name` und `xml`; ein roher XML-Body wird mit **415** abgelehnt.

```bash
jq -Rs '{name:"erster-prozess", xml:.}' erster-prozess.bpmn > deploy.json
curl -s -X POST http://localhost:8081/api/deploy \
  -H "Content-Type: application/json" \
  --data-binary @deploy.json
```

**Erwartetes Ergebnis:** HTTP 200 mit der `definition_key` (UUID) und der `version` der neuen Definition:

```json
{"definition_key":"3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90","version":1}
```

Notiere die `definition_key`. Du brauchst sie in Schritt 5. Wenn du sie verlierst, listest du sie mit `curl -s http://localhost:8081/api/definitions` erneut auf.

> **Was gerade passiert ist:** Die Engine hat dein XML geparst, die Definition gespeichert und die XML-Datei im Object Store abgelegt. Ab jetzt ist die Definition unveränderlich. Ein erneuter Deploy derselben `bpmn_process_id` erzeugt `version: 2`, nicht eine Änderung an Version 1. Warum das so ist: [`explanation/konzepte.md`](../explanation/konzepte.md#5-migration-laufende-instanzen-auf-eine-andere-version-heben). Der vollständige Ablauf mit Versionierung und Löschen steht in [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md).

---

## Schritt 5: Instanz starten

Starte eine Instanz der gerade deployten Definition. Ersetze `<definition_key>` durch den Wert aus Schritt 4:

```bash
curl -s -X POST http://localhost:8081/api/start \
  -H "Content-Type: application/json" \
  -d '{"definition_key":"<definition_key>","variables":{"kunde":"Beispiel GmbH"}}'
```

**Erwartetes Ergebnis:** HTTP 200 mit der Instanz-ID:

```json
{"instance_id":"a1b2c3d4-0000-4000-8000-000000000001"}
```

Der Start-Endpunkt liefert **nur** die ID. Den Zustand liest du mit einem zweiten Aufruf:

```bash
curl -s http://localhost:8081/api/instances/<instance_id>
```

Erwartet ist `"state": {"WaitingOnUserTask": {"task_id": "…"}}` in der vollständigen Instanzantwort. Notiere die `instance_id`. Der Wait State ist das erwartete Ergebnis: Die Engine hat das Token bis zum User Task bewegt, den Zustand persistiert und die Ausführung angehalten.

> **Wenn du stattdessen `Completed` siehst:** Dann hat dein Modell keinen User Task zwischen Start und End, oder der User Task wurde nicht als solcher geparst. Prüfe die XML-Datei aus Schritt 3.

---

## Schritt 6: User Task finden und abschließen

Liste die offenen User Tasks auf:

```bash
curl -s http://localhost:8081/api/tasks
```

**Erwartetes Ergebnis:** Ein Array mit genau einem Eintrag für deine Instanz. Der Eintrag enthält `task_id`, `instance_id`, `node_id`, `assignee`, `token_id`, `created_at` und `business_key`:

```json
[
  {
    "task_id": "b2c3d4e5-0000-4000-8000-000000000002",
    "instance_id": "a1b2c3d4-0000-4000-8000-000000000001",
    "node_id": "bestaetigen",
    "assignee": "tutorial-leser",
    "token_id": "c3d4e5f6-0000-4000-8000-000000000003",
    "created_at": "2026-10-24T11:35:22.240738Z",
    "business_key": "d4e5f6a7-0000-4000-8000-000000000004"
  }
]
```

Schließe den Task ab. Ersetze `<task_id>` durch die `task_id` aus der Antwort:

```bash
curl -s -X POST http://localhost:8081/api/complete/<task_id> \
  -H "Content-Type: application/json" \
  -d '{"variables":{"bestaetigt":true}}'
```

**Erwartetes Ergebnis:** HTTP **204** (No Content, leerer Body). Die Engine setzt das Token hinter den User Task, erreicht den End Event und markiert die Instanz als abgeschlossen.

---

## Schritt 7: Abschluss prüfen

Lies den Zustand der Instanz:

```bash
curl -s http://localhost:8081/api/instances/<instance_id>
```

**Erwartetes Ergebnis:** Der Zustand ist `Completed`, und die Variablen enthalten sowohl `kunde` aus dem Start als auch `bestaetigt` aus dem Task-Abschluss. Die Antwort ist umfangreicher als der Auszug; gezeigt sind die für den Lernpfad relevanten Felder:

```json
{
  "id": "a1b2c3d4-0000-4000-8000-000000000001",
  "definition_key": "3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90",
  "state": "Completed",
  "current_node": "ende",
  "variables": {
    "kunde": "Beispiel GmbH",
    "bestaetigt": true
  },
  "tokens": {},
  "incidents": [],
  "completed_at": "2026-10-24T11:35:25.781687Z"
}
```

Zusätzlich findest du die Instanz im Archiv der abgeschlossenen Instanzen:

```bash
curl -s "http://localhost:8081/api/history/instances?definition_key=<definition_key>"
```

**Erwartetes Ergebnis:** Ein Array, das deine Instanz enthält. Abgeschlossene Instanzen werden aus dem aktiven Bucket entfernt und in den History-Store verschoben — sie sind nicht gelöscht.

---

## Woran du erkennst, dass du fertig bist

Das Tutorial ist abgeschlossen, wenn **alle vier** Punkte zutreffen. Prüfe sie der Reihe nach — jeder Punkt ist ein einzelner Befehl.

| # | Abnahmekriterium | Prüfung | Erwartet |
|---|---|---|---|
| 1 | Die Engine ist bereit | `curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8081/api/ready` | `200` |
| 2 | Eine Definition existiert | `curl -s http://localhost:8081/api/definitions` | Array mit deiner `definition_key`, `version: 1` |
| 3 | Die Instanz ist abgeschlossen | `curl -s http://localhost:8081/api/instances/<instance_id>` | `"state":"Completed"` |
| 4 | Die Variablen sind erhalten | derselbe Aufruf | `kunde` **und** `bestaetigt` vorhanden |

Wenn ein Punkt abweicht, springe in die [Fehlerbehebung](#fehlerbehebung). Wenn alle vier zutreffen: **Geschafft.** Du hast eine Process Definition deployt, eine Process Instance gestartet, einen Wait State überwunden und den Abschluss belegt.

---

## Fehlerbehebung

Hier stehen nur die Fehler, die auf dem Tutorialpfad auftreten. Für alle anderen Symptome gilt die Übersicht [Hilfe bei Problemen](../README.md#hilfe-bei-problemen) auf der Landingpage.

### Fehler 1: Deploy antwortet mit HTTP 400 und `InvalidDefinition`

**Symptom:**

```json
{"error":"InvalidDefinition: ..."}
```

**Ursachen und Behebung:**

1. **Falsches Wurzelelement oder fehlender Namespace.** Das XML muss `<bpmn:definitions>` mit `xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"` als Wurzel haben. Ein `<definitions>` ohne Präfix oder mit falschem Namespace wird abgelehnt.
2. **Nicht unterstütztes Element.** BPMNinja lehnt eine Reihe von Elementen beim Parsen ab, statt sie stillschweigend zu akzeptieren. Dazu gehören `multiInstanceLoopCharacteristics` außerhalb von `serviceTask` (kein Multi-Instance), ein generisches `<task>`, ein `scriptTask` mit leerem `<script>`-Body, ein `receiveTask` ohne `messageRef`, ein `businessRuleTask` ohne Topic und ein `intermediateCatchEvent` ohne Timer oder Message. Entferne das Element oder ersetze es durch ein unterstütztes.
3. **Mehrere `<process>`-Elemente.** Nur das erste `isExecutable="true"`-Prozess wird deployt. Wenn dein eigentliches Modell das zweite ist, wird es ignoriert. Teile die Datei auf.
4. **Falscher `Content-Type` oder roher XML-Body.** `/api/deploy` akzeptiert ausschließlich `application/json` mit `{ "name": …, "xml": … }`. Ein XML-Body oder ein fehlendes Pflichtfeld wird mit **415** bzw. **422** abgelehnt. Setze den Header und den Body wie in Schritt 4.

Die Fehlermeldung nennt in der Regel das betroffene Element oder Attribut. Vergleiche sie mit der Statuscode-Tabelle in [`reference/rest-api.md`](../reference/rest-api.md#fehlerantworten) und der Ablehnungsliste in [`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md).

### Fehler 2: `/api/ready` antwortet mit 503 oder der Server startet nicht

**Symptom A — der Server bricht beim Start ab:**

```text
Error: failed to connect to NATS
```

**Symptom B — der Server läuft, aber Readiness schlägt fehl:**

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8081/api/ready
# 503
```

**Ursache:** `REQUIRE_NATS` ist standardmäßig `true` (fail-closed). Ohne erreichbaren NATS-JetStream startet die Engine entweder nicht oder meldet sich nicht bereit. Das ist beabsichtigt: Die Engine soll nicht stillschweigend ohne Persistenz laufen und bei einem Neustart allen Zustand verlieren.

**Behebung:**

1. Prüfe, ob der NATS-Container läuft: `docker compose ps nats`. Wenn nicht, wiederhole Schritt 1.
2. Prüfe, ob JetStream aktiv ist. Der Container muss mit `--js` starten; ohne JetStream sind KV-Stores und Object Stores nicht verfügbar.
3. Prüfe die Verbindungs-URL. Der Default ist `NATS_URL=nats://localhost:4222`. Wenn dein NATS mit Benutzer und Passwort läuft, muss die URL die Zugangsdaten enthalten, z. B. `nats://bpmninja:bpmninja@localhost:4222`.
4. Starte die Engine neu, nachdem NATS erreichbar ist.

**Nicht** die Lösung: `REQUIRE_NATS=false` zu setzen, nur um den Fehler loszuwerden. Das schaltet die Persistenz ab; alle Instanzen gehen bei einem Neustart verloren, und `/api/ready` bleibt `503`. Der Schalter ist ausschließlich für ephemere lokale Läufe gedacht. Details: [`how-to/konfiguration-anpassen.md`](../how-to/konfiguration-anpassen.md#fehler-2-apiready-bleibt-503-oder-die-engine-startet-nicht).

### Fehler 3 (häufige Begleiterscheinung): HTTP 401 auf allen `/api`-Routen

**Symptom:** Jeder Aufruf außer `/api/health` und `/api/ready` antwortet mit **401**.

**Ursache:** Die Engine bindet standardmäßig an `0.0.0.0`. Ohne gesetzten API-Key ist das ein fail-closed-Fall: Alle `/api`-Routen werden gesperrt.

**Behebung:** Starte die Engine mit `BIND_ADDR=127.0.0.1` (wie in Schritt 2) oder setze einen Key und sende ihn mit:

```bash
export BPMNINJA_API_KEY="mein-lokaler-key"
curl -s -H "X-API-Key: $BPMNINJA_API_KEY" http://localhost:8081/api/definitions
```

Der Key wird als `X-API-Key`-Header oder als `Authorization: Bearer …` akzeptiert. Details: [`reference/rest-api.md`](../reference/rest-api.md#authentifizierung).

---

## Aufräumen

Beende den Server mit `Strg+C` im ersten Terminal. Den NATS-Container stoppst du mit:

```bash
docker compose stop nats
```

Die deployte Definition und die abgeschlossene Instanz bleiben in NATS erhalten, solange das Volume besteht. Wenn du vollständig neu beginnen willst, entferne das Volume mit `docker compose down -v`.

---

## Nächster sinnvoller Schritt

Du hast einen Prozess mit einem **User Task** ausgeführt — also mit einem Wait State, den ein Mensch abschließt. Der nächste Schritt ist der wichtigste Integrationspfad von BPMNinja: ein **Service Task**, den ein externer Worker per Fetch-and-Lock abholt und abschließt.

→ **[`how-to/external-task-worker-schreiben.md`](../how-to/external-task-worker-schreiben.md)** — External-Task-Worker schreiben und testen: Topic im Modell vergeben, Worker mit `fetchAndLock` starten, Task abschließen, einen Incident mit `failure` und `retries: 0` erzeugen und einen fachlichen Fehler über `bpmnError` in ein Boundary Error Event leiten.

Wenn du dein eigenes Modell deployen und als **neue Version** danebenstellen willst, lies:

→ **[`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md)** — Version 1 und 2, `start` vs. `start/latest`, XML abrufen und Löschen mit `409`.

Wenn du zuerst verstehen willst, *warum* die Engine an einem Wait State anhält und wie Tokens durch den Graph wandern, lies:

→ **[`explanation/konzepte.md`](../explanation/konzepte.md)** — Token, Wait State und die Abweichungen zu BPMN 2.0 und Camunda.

Wenn du die Engine für deinen eigenen Betrieb konfigurieren willst (Keys, Readiness, Body-Limits, Logging), lies:

→ **[`how-to/konfiguration-anpassen.md`](../how-to/konfiguration-anpassen.md)** — Konfiguration anpassen mit Prüfschritt nach jedem Block.

---

## Offen

- **Exakte Response-Schemata:** Die in diesem Tutorial gezeigten JSON-Antworten stützen sich auf die in `docs/openapi.yaml` dokumentierten Felder (`definition_key`, `version`, `instance_id`, `state`, `variables`, Task-`id`/`name`/`assignee`/`instance_id`). Die vollständige Feldliste je Endpunkt ist in `docs/openapi.yaml` verbindlich; diese Seite zeigt nur die für den Lernpfad notwendigen Felder. Die Beispielantworten sind am 2026-10-24 gegen eine laufende Engine geprüft (`_qa/verification-report.md`); ein automatischer Abgleich gegen `docs/openapi.yaml` steht weiterhin aus.
- **`Content-Type` für `/api/deploy`:** `/api/deploy` akzeptiert ausschließlich `application/json` mit den Feldern `name` und `xml`; ein roher XML-Body wird mit **415** abgelehnt (belegt, Live-Lauf 2026-10-24). **Offen:** Die vollständige Liste der akzeptierten Medientypen ist in dieser Seite nicht belegt und muss aus `docs/openapi.yaml` ergänzt werden.
- **`xmllint`-Prüfung:** Optional und nicht Voraussetzung; auf Systemen ohne `libxml2` entfällt der Schritt ersatzlos.
- **Weiterführende Referenzen:** Die Element-Abbildung steht in [`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md), die Abweichungen in [`reference/kompatibilitaet.md`](../reference/kompatibilitaet.md). `reference/fehlercodes.md` und `reference/glossar.md` sind noch nicht geschrieben; das Glossar liegt unter [`_review/glossary.md`](../_review/glossary.md).
- **Ankerlinks:** Die internen Verweise auf `#fehler-1-…` und `#fehler-2-…` zeigen auf Überschriften mit Umlauten und Sonderzeichen. Die Linkprüfung prüft Anker nicht; 57 Anker wurden separat gegen GitHub-Slug-Regeln geprüft (0 Defekte, `_qa/verification-report.md` §4). Ein Rendering im Zielsystem ist nicht bewiesen.
