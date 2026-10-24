---
title: Prozess deployen und Versionen verwalten
description: Ein BPMN-Modell als JSON deployen, eine zweite Version erzeugen, Instanzen wahlweise auf eine feste Version oder auf die neueste starten, XML abrufen und Definitionen löschen — mit erwarteten Antworten, Statuscodes und den vier Ablehnungsfällen.
audience:
  - Prozessmodellierer
  - Entwickler und Integratoren
prerequisites:
  - Laufende BPMNinja-Engine unter http://localhost:8081 (siehe tutorials/erster-prozess.md)
  - curl und jq installiert
  - Ein BPMN-Modell mit genau einem ausführbaren <process>-Element
  - Admin-Key, falls das Auth-Gate aktiv ist
content_type: how-to
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/bpmn-elemente.md
  - reference/kompatibilitaet.md
  - reference/rest-api.md
  - tutorials/erster-prozess.md
  - explanation/konzepte.md
  - _review/glossary.md
---

# Prozess deployen und Versionen verwalten

## Aufgabe

Du willst ein fertiges Modell deployen, eine korrigierte Fassung **danebenstellen** statt sie zu überschreiben, und entscheiden, welche Version neue Instanzen bekommt. Du willst außerdem wissen, warum ein Deploy mit `InvalidDefinition` scheitert, wie du das deployte XML zurückholst und was beim Löschen einer Definition mit laufenden Instanzen passiert.

Der Guide setzt voraus, dass ein Prozess schon einmal gelaufen ist. Wenn nicht, beginne mit [`tutorials/erster-prozess.md`](../tutorials/erster-prozess.md).

> **Wo du gerade einsteigst:** Diese Seite ist Schritt 1 des Lesepfads E („Ich will mein Modell deployen und versionieren"). Danach folgen [`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md) und [`reference/kompatibilitaet.md`](../reference/kompatibilitaet.md). Alle Pfade: [Dokumentationslandingpage](../README.md#lesepfade).

## Voraussetzungen

- Die Engine läuft: `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/health` liefert `200` (leerer Body).
- `curl` und `jq` sind installiert.
- Du hast eine `.bpmn`-Datei mit **genau einem** ausführbaren `<process>`-Element. Weitere Prozesse in derselben Datei werden ignoriert.
- Du kennst die `id` deines `<process>`-Elements. Sie ist die `bpmn_process_id` und bestimmt die Versionskette.
- Läuft die Engine mit gesetztem Key, brauchst du den **Admin-Key**: Deploy, Definitionsliste und Löschen sind Admin-Routen.

## Erwartetes Ergebnis

Am Ende dieses Guides hast du belegt:

1. Ein Modell ist deployt und hat eine `definition_key` (UUID) und `version: 1`.
2. Ein zweiter Deploy derselben `bpmn_process_id` hat **Version 2** erzeugt; Version 1 existiert weiter.
3. Du hast eine Instanz wahlweise auf eine **feste Version** (`definition_key`) oder auf die **neueste Version** (`bpmn_process_id`) gestartet.
4. Du kennst den Unterschied zwischen „eine Version löschen" und „alle Versionen löschen" und weißt, wann `409` kommt.

---

## Schritt 1: Modell vor dem Deploy prüfen

Prüfe zwei Dinge, bevor du den Deploy abschickst. Beide Fehler kosten sonst einen `400`-Roundtrip.

1. **Genau ein ausführbares `<process>`.** Nur das erste `<process isExecutable="true">` wird deployt; ist keines als executable markiert, das erste `<process>` überhaupt.
2. **Keine abgelehnten Elemente.** Ein generisches `<task>`, ein leerer `<scriptTask>`, ein `receiveTask` ohne `messageRef` und sequenzielle Multi-Instance lassen den Deploy mit `InvalidDefinition` scheitern.

Die vollständige Liste der Ablehnungsfälle steht in [`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md), Abschnitt 7.

**Erwartetes Ergebnis:** Die Datei ist wohlgeformtes XML.

```bash
xmllint --noout mein-prozess.bpmn && echo "XML ist wohlgeformt"
```

Wenn `xmllint` nicht installiert ist, überspringe die Prüfung — der Deploy in Schritt 2 validiert das XML ohnehin.

---

## Schritt 2: Version 1 deployen

`POST /api/deploy` akzeptiert **ausschließlich** `application/json` mit den Pflichtfeldern `name` und `xml`. Ein roher XML-Body wird mit `415` abgelehnt, ein fehlendes Pflichtfeld mit `422`.

```bash
jq -Rs '{name:"mein-prozess", xml:.}' mein-prozess.bpmn > deploy.json
curl -s -X POST http://localhost:8081/api/deploy \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  --data-binary @deploy.json
```

**Erwartete Ausgabe (HTTP 200):**

```json
{"definition_key":"3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90","version":1}
```

Notiere die `definition_key`. Sie adressiert **diese eine Version**, nicht den Prozess.

> **Was dabei passiert:** Der Deploy hat zwei Effekte, nicht einen. Die Engine parst das XML zu einer `ProcessDefinition` (KV-Bucket `definitions`) und legt das Roh-XML separat im Object Store `bpmn_xml` ab. Der Vorgang ist atomar: Schlägt das Parsen fehl, wird weder eine Definition noch das XML gespeichert. Ab jetzt ist die Version unveränderlich.

---

## Schritt 3: Deployte Definitionen prüfen

```bash
curl -s -H "X-API-Key: $BPMNINJA_API_KEY" http://localhost:8081/api/definitions
```

**Erwartetes Ergebnis:** Ein Array, das deine Definition mit `definition_key` und `version: 1` enthält.

> **Offen:** Die vollständige Feldliste von `GET /api/definitions` ist hier nicht belegt; verbindlich ist `docs/openapi.yaml`. Für diesen Guide genügen `definition_key` und `version`.

---

## Schritt 4: Version 2 erzeugen

Eine Definition ist unveränderlich. Eine Korrektur entsteht deshalb als **neue Version**: Du deployst erneut, mit derselben `bpmn_process_id` und geändertem Inhalt.

Ändere im XML nur das `name`-Attribut des `<process>`-Elements — die `id` bleibt gleich:

```xml
<bpmn:process id="mein-prozess" name="Mein Prozess (korrigiert)" isExecutable="true">
```

```bash
jq -Rs '{name:"mein-prozess", xml:.}' mein-prozess.bpmn > deploy.json
curl -s -X POST http://localhost:8081/api/deploy \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  --data-binary @deploy.json
```

**Erwartete Ausgabe (HTTP 200):**

```json
{"definition_key":"8a1c4d20-5b6e-4f81-9c33-7d2e1f0b6a55","version":2}
```

Die `definition_key` ist neu, die Version ist `2`. Version 1 bleibt deployt und bleibt die Version, auf der bereits laufende Instanzen stehen.

---

## Schritt 5: Instanzen starten — feste Version oder neueste

Es gibt zwei Start-Endpunkte, und der Unterschied ist der Grund, warum Versionierung überhaupt sichtbar wird.

**Feste Version** — du kennst die `definition_key` und willst genau diese:

```bash
curl -s -X POST http://localhost:8081/api/start \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  -d '{"definition_key":"3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90","variables":{"kunde":"Beispiel GmbH"}}'
```

**Neueste Version** — du kennst nur die `bpmn_process_id` und willst die höchste Version:

```bash
curl -s -X POST http://localhost:8081/api/start/latest \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  -d '{"bpmn_process_id":"mein-prozess","variables":{"kunde":"Beispiel GmbH"}}'
```

**Erwartete Ausgabe (HTTP 200):**

```json
{"instance_id":"a1b2c3d4-0000-4000-8000-000000000001"}
```

Beide Endpunkte liefern **nur** die `instance_id`. Den Zustand liest du mit `GET /api/instances/{id}`.

`business_key` ist auf beiden Start-Endpunkten top-level erlaubt und hat Vorrang vor einem gleichnamigen Schlüssel in `variables`; die Engine entfernt ihn dann aus den Variablen. `bpmn_id` bleibt als Alias für `bpmn_process_id` gültig.

---

## Schritt 6: XML einer Version abrufen

```bash
curl -s -H "X-API-Key: $BPMNINJA_API_KEY" \
  http://localhost:8081/api/definitions/3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90/xml
```

**Erwartetes Ergebnis:** Rohes XML als `text/plain` — **kein** `{ "xml": … }`-Objekt. Fehlt das XML, antwortet der Endpunkt mit `404` und `{"error":"No XML found for definition '…'"}`.

Das ist der Weg, um zu prüfen, welche Fassung tatsächlich deployt ist. Das XML im Object Store ist das Artefakt; die geparste Definition ist die Wahrheit für die Ausführung.

---

## Schritt 7: Definitionen löschen

**Eine Version löschen:**

```bash
curl -s -o /dev/null -w "%{http_code}\n" -X DELETE \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  http://localhost:8081/api/definitions/3f2b8c1e-9a4d-4e77-8b1f-2c5d6e7f8a90
```

Hängen noch Instanzen an dieser Version, antwortet der Endpunkt mit **409** (`DefinitionHasInstances`). Mit `?cascade=true` werden die Instanzen mitgelöscht.

**Alle Versionen einer `bpmn_process_id` löschen:**

```bash
curl -s -o /dev/null -w "%{http_code}\n" -X DELETE \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  http://localhost:8081/api/definitions/bpmn/mein-prozess
```

> **Warnung:** `?cascade=true` löscht Instanzen mit. Prüfe vorher mit `GET /api/instances`, ob dort noch etwas läuft, was du behalten willst. Welche Wirkung das Löschen auf `history_instances` und auf das XML im Object Store hat, ist **nicht belegt** — siehe **Offen**.

---

## Woran du erkennst, dass du fertig bist

| # | Abnahmekriterium | Prüfung | Erwartet |
|---|---|---|---|
| 1 | Zwei Versionen existieren | `GET /api/definitions` | zwei Einträge mit derselben `bpmn_process_id`, `version: 1` und `version: 2` |
| 2 | Die feste Version startet | `POST /api/start` mit der alten `definition_key` | `200` mit `instance_id` |
| 3 | Die neueste Version startet | `POST /api/start/latest` mit der `bpmn_process_id` | `200`; `GET /api/instances/{id}` zeigt `definition_key` der Version 2 |
| 4 | Das XML ist abrufbar | `GET /api/definitions/{id}/xml` | `200`, `text/plain`, Inhalt deiner Datei |

Wenn ein Punkt abweicht, springe in die [typischen Fehler](#typische-fehler).

---

## Typische Fehler

### Fehler 1: Deploy antwortet mit 400 und `InvalidDefinition`

**Symptom:** `{"error":"InvalidDefinition: …"}`

**Ursachen:** falscher oder fehlender Namespace `xmlns:bpmn`; ein abgelehntes Element (generisches `<task>`, leerer `<scriptTask>`, `receiveTask` ohne `messageRef`, sequenzielle Multi-Instance); ein `businessRuleTask` ohne Topic; ein `intermediateCatchEvent` ohne Timer oder Message.

**Behebung:** Vergleiche die Meldung mit der Liste in [`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md), Abschnitt 7. Entferne das Element oder ersetze es durch ein unterstütztes.

### Fehler 2: Deploy antwortet mit 415 oder 422

**Symptom:** `415` bei korrekt aussehendem Body, oder `422` bei gültigem JSON.

**Ursache:** `415` heißt falscher oder fehlender `Content-Type` — der Endpunkt akzeptiert nur `application/json`. `422` heißt formal gültiges JSON, aber ein Pflichtfeld (`name` oder `xml`) fehlt oder hat den falschen Typ.

**Behebung:** Setze den Header wie in Schritt 2 und erzeuge den Body mit `jq -Rs '{name:…, xml:.}'`, damit das XML als String eingebettet ist.

### Fehler 3: Löschen antwortet mit 409

**Symptom:** `409` mit `DefinitionHasInstances`.

**Ursache:** Es laufen noch Instanzen auf dieser Version. Die Engine löscht keine Definition, unter der Arbeit hängt.

**Behebung:** Instanzen abschließen oder bewusst mit `?cascade=true` mitlöschen. Prüfe vorher mit `GET /api/instances`, welche Instanzen betroffen sind.

### Fehler 4: Die neue Instanz läuft auf der alten Version

**Symptom:** Nach dem Deploy startet `POST /api/start` eine Version, die nicht den neuen Inhalt hat.

**Ursache:** `POST /api/start` nimmt eine `definition_key` — eine feste Version. Wenn du die alte `definition_key` mitschickst, startest du die alte Version. Ein weiterer Fall: Du hast die `id` des `<process>` geändert. Dann ist eine **neue Versionskette** entstanden und die Zählung beginnt wieder bei `1`.

**Behebung:** Für „immer die neueste" `POST /api/start/latest` mit der `bpmn_process_id` verwenden. Die `id` des `<process>` nur ändern, wenn du bewusst einen neuen Prozess anlegst.

---

## Nächster sinnvoller Schritt

Du hast deployt und versioniert. Die nächste Frage ist, welche Elemente dein Modell enthalten darf:

→ **[`reference/bpmn-elemente.md`](../reference/bpmn-elemente.md)** — XML → Laufzeittyp, Attributauflösung, Multi-Instance-Regeln und alle Ablehnungsfälle mit Meldung.

Wenn du von Camunda kommst oder ein bestehendes Modell migrierst, lies:

→ **[`reference/kompatibilitaet.md`](../reference/kompatibilitaet.md)** — alle Abweichungen mit Richtung und eine Migrationscheckliste.

Wenn du verstehen willst, warum eine Version unveränderlich ist und wann eine Migration nötig wird, lies:

→ **[`explanation/konzepte.md`](../explanation/konzepte.md)** — Migration, Modification und `OrphanedToken`.

Wenn du einen Service Task in deinem Modell durch einen Worker ausführen lassen willst, lies:

→ **[`how-to/external-task-worker-schreiben.md`](../how-to/external-task-worker-schreiben.md)** — `fetchAndLock`, `complete`, `failure` und `bpmnError`.

---

## Offen

- **Live-Beleg für Löschen:** Die Statuscodes für `DELETE /api/definitions/{id}` (`200`, `404`, `409`) und für `DELETE /api/definitions/bpmn/{bpmn_id}` (`200`, `404`) stammen aus der Knowledge Base und der Route-Map, nicht aus einem Live-Lauf. **Offen:** mit einer Testinstanz verifizieren.
- **Wirkung von `?cascade=true`:** Dass Instanzen mitgelöscht werden, ist belegt. Ob zusätzlich `history_instances`-Einträge und das XML im Object Store `bpmn_xml` entfernt werden, ist **nicht belegt**. **Offen:** vor einem produktiven Löschen prüfen.
- **`name` im Deploy-Body:** Das Feld ist Pflicht. Ob es die Versionskette beeinflusst oder nur ein Anzeigename ist, ist nicht belegt. Verbindlich für die Versionierung ist die `bpmn_process_id` aus dem XML.
- **Feldliste `GET /api/definitions`:** Nicht vollständig belegt; verbindlich ist `docs/openapi.yaml`.
- **Ankerlinks:** Diese Seite verwendet keine Anker auf Überschriften mit Umlauten. Der Verweis `#typische-fehler` zeigt auf eine ASCII-Überschrift.
