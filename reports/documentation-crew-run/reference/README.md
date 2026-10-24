---
title: Referenz
description: Referenzlandingpage für BPMNinja — Tabellen der Inhalte zu REST-API, Service-Task-API, CLI, Konfiguration, BPMN-Elementen und Kompatibilität, Quellen der Wahrheit, Schnellverweise auf häufig gesuchte Verträge sowie Rückwege in Tutorial, How-to und Explanation.
audience:
  - Einsteiger und Lernende
  - Prozessmodellierer
  - Entwickler und Integratoren
  - Betriebsteams und Plattform-Administratoren
  - Stakeholder und Entscheider
  - Beitragende und Maintainer
prerequisites:
  - Grundverständnis von BPMN 2.0 (Start Event, Task, Gateway, End Event)
  - Für API-Seiten: Vertrautheit mit HTTP und JSON
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/rest-api.md
  - reference/service-task-api.md
  - reference/cli.md
  - reference/konfiguration.md
  - reference/bpmn-elemente.md
  - reference/kompatibilitaet.md
  - how-to/prozess-deployen.md
  - tutorials/erster-prozess.md
  - how-to/external-task-worker-schreiben.md
  - how-to/konfiguration-anpassen.md
  - explanation/architektur.md
  - _review/glossary.md
---

# Referenz

Diese Seite ist der Einstieg in die Referenzschicht von BPMNinja. Sie beschreibt **was ist** — Verträge, Feldnamen, Standardwerte, Grenzwerte, Statuscodes. Sie beschreibt nicht, **wie** du eine Aufgabe erledigst (das steht in `how-to/`) und nicht, **warum** etwas so gebaut ist (das steht in `explanation/`).

> **Wo du gerade einsteigst:** Diese Seite ist der Nachschlage-Einstieg. Wenn du noch keinen Prozess ausgeführt hast, beginne mit [`tutorials/erster-prozess.md`](../tutorials/erster-prozess.md). Alle Lesepfade: [Dokumentationslandingpage](../README.md#lesepfade).

## Was suchst du?

| Ich suche … | Seite | Abschnitt |
|---|---|---|
| einen Endpunkt, eine Methode oder einen Statuscode | [`reference/rest-api.md`](rest-api.md) | Routengruppen |
| die Einheit der Lock-Dauer | [`reference/service-task-api.md`](service-task-api.md) | Lock-Dauer |
| die Validierungsgrenzen von `fetchAndLock` | [`reference/service-task-api.md`](service-task-api.md) | `fetchAndLock` — Validierung |
| das Fehlerformat `{ "error": … }` | [`reference/rest-api.md`](rest-api.md) | Fehlerantworten |
| **wie ich eine zweite Version deploye** | [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md) | Schritte 4–5 |
| **welche Statuscodes ein Deploy liefert** | [`reference/rest-api.md`](rest-api.md) | Abschnitt „Definitions" |
| **ob ich eine Definition mit laufenden Instanzen löschen kann** | [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md) | Schritt 7 |
| einen Start-, Build- oder Testbefehl | [`reference/cli.md`](cli.md) | Build, Test, Lint, Format |
| einen Umgebungsvariablen-Default | [`reference/konfiguration.md`](konfiguration.md) | Abschnitte 1–10 |
| ob mein BPMN-Element unterstützt wird | [`reference/bpmn-elemente.md`](bpmn-elemente.md) | Abschnitte 1–8 |
| ob sich etwas wie in Camunda verhält | [`reference/kompatibilitaet.md`](kompatibilitaet.md) | Abschnitte 2–10 |
| die Bedeutung eines Begriffs | [`_review/glossary.md`](../_review/glossary.md) | Übersicht am Seitenanfang |

## Wie diese Referenz aufgebaut ist

Jede Detailseite folgt demselben Muster: eine Tabelle mit dem Vertrag, darunter Beispiele mit konkreten Werten, darunter ein Abschnitt **Offen** für Aussagen, die zum Redaktionsstand nicht gegen Code oder OpenAPI belegt sind. Eine Aussage ohne Beleg ist als „Offen" markiert und darf nicht als Zusage gelesen werden.

**Quellen der Wahrheit:**

| Bereich | Quelle der Wahrheit | Status |
|---|---|---|
| REST-Endpunkte, Request-/Response-Schemata | `docs/openapi.yaml` | verbindlich (Entscheidung E4 der Informationsarchitektur) |
| Lock-Dauer-Semantik (`lockDuration`, `newDuration`) | [`reference/service-task-api.md`](service-task-api.md) | kanonisch (Entscheidung E7) |
| Umgebungsvariablen und Defaults | [`reference/konfiguration.md`](konfiguration.md) | kanonisch, gegen den Quelltext geprüft |
| Fehlerformat und HTTP-Status | [`reference/rest-api.md`](rest-api.md) | belegt; die vollständige `EngineError`-Zuordnung folgt mit `reference/fehlercodes.md` |
| BPMN-Element-Abbildung | [`reference/bpmn-elemente.md`](bpmn-elemente.md) | kanonisch, gegen den Parser geprüft |
| Abweichungen von BPMN 2.0 und Camunda 7 | [`reference/kompatibilitaet.md`](kompatibilitaet.md) | kanonisch, eine Seite für alle Abweichungen (IA-Entscheidung E6) |
| Versionskette `bpmn_process_id` → `version` | [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md) | belegt für den Ablauf; Migrationssemantik in [`explanation/konzepte.md`](../explanation/konzepte.md) |

> **Endpunktzahl:** Diese Referenz nennt bewusst **keine** Gesamtzahl der REST-Endpunkte. Der Bestand enthielt sieben widersprüchliche Angaben (46 vs. 48, siehe Audit AUD-006). Verbindlich ist ausschließlich `docs/openapi.yaml`.

---

## Tabellen der Inhalte

### API-Referenz

| Seite | Inhalt | Zielgruppe | Status |
|---|---|---|---|
| [`reference/rest-api.md`](rest-api.md) | Routengruppen mit Methode, Pfad, Auth-Rolle und Statuscodes; Fehlerformat `{ "error": … }`; Verweis auf `docs/openapi.yaml` | Integratoren, Betriebsteams | draft |
| [`reference/service-task-api.md`](service-task-api.md) | `fetchAndLock`-Validierung, `complete`/`failure`/`bpmnError`/`extendLock`/`retry`/`resolve`, Lock-Ablauf, Idempotenz. **Kanonische Definition der Lock-Dauer in Millisekunden.** | Integratoren, Betriebsteams | draft |

### Betrieb und Konfiguration

| Seite | Inhalt | Zielgruppe | Status |
|---|---|---|---|
| [`reference/cli.md`](cli.md) | Befehle, Argumente und Beispiele für Build, Test, Lint, Start, Docker Compose, Desktop-UI und External-Task-Client | Beitragende, Betriebsteams | draft |
| [`reference/konfiguration.md`](konfiguration.md) | Alle Umgebungsvariablen mit Default, Gültigkeitsbereich und Wirkung; KV-Buckets, Object Stores und Streams | Betriebsteams, Beitragende | draft |

### Modellierung

| Seite | Inhalt | Zielgruppe | Status |
|---|---|---|---|
| [`reference/bpmn-elemente.md`](bpmn-elemente.md) | Element-Abbildung XML → Laufzeittyp, Attributauflösung, Multi-Instance-Regeln und Ablehnungsfälle | Prozessmodellierer, Entwickler | draft |
| [`reference/kompatibilitaet.md`](kompatibilitaet.md) | Abweichungen von BPMN 2.0 und Camunda 7, API-Unterschiede und Migrationscheckliste | Prozessmodellierer, Integratoren | draft |

### Nachschlagen

| Seite | Inhalt | Zielgruppe | Status |
|---|---|---|---|
| [`_review/glossary.md`](../_review/glossary.md) | Verbindliche Begriffsliste mit Definition, zulässigen Synonymen, abzugrenzenden Begriffen und Beispiel je Eintrag | Alle | draft |

---

## Schnellverweise auf häufig gesuchte Verträge

| Frage | Antwort | Belegstelle |
|---|---|---|
| In welcher Einheit sende ich `lockDuration`? | **Millisekunden**, gültig 1 000 bis 3 600 000 ms. Der Server rechnet intern in Sekunden um: `(ms + 999) / 1000`. | [`reference/service-task-api.md`](service-task-api.md) |
| Welche Statuscodes kann `fetchAndLock` liefern? | `200` (Liste, ggf. leer), `400` (Validierung: `topics` 1..=32, `topicName` nicht leer, `lockDuration` 1 000..=3 600 000 ms, `maxTasks` 1..=100), `401` (Gate aktiv, Key fehlt oder ist falsch). **Kein `403`** — die Route ist für die Worker-Rolle zugelassen. | [`reference/service-task-api.md`](service-task-api.md) |
| Wie sieht eine Fehlerantwort aus? | Immer JSON: `{ "error": "Human-readable error message" }` — keine HTML-Fehlerseiten. | [`reference/rest-api.md`](rest-api.md) |
| Wie entsteht Version 2 einer Definition? | Erneuter Deploy mit derselben `bpmn_process_id` und geändertem Inhalt. Die alte Version bleibt deployt. | [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md), Schritt 4 |
| Woran erkenne ich, welche Version eine Instanz nutzt? | `GET /api/instances/{id}` nennt die `definition_key`; `GET /api/definitions` ordnet `definition_key` der `version` zu. | [`reference/rest-api.md`](rest-api.md), [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md) |
| Was liefert `GET /api/definitions/{id}/xml`? | Rohes XML als `text/plain`, **kein** `{ "xml": … }`-Objekt. Fehlt das XML: `404` mit `{ "error": "No XML found for definition '…'" }`. | [`reference/rest-api.md`](rest-api.md) |
| Welcher Default gilt für `REQUIRE_NATS`? | `true` — fail-closed. Ohne NATS bricht der Start ab; `/api/ready` liefert `503`. | [`reference/konfiguration.md`](konfiguration.md) |
| Welcher Default gilt für `BIND_ADDR`? | `0.0.0.0`. Ohne gesetzten Key und Nicht-Loopback-Bind antworten alle `/api`-Routen mit `401`. | [`reference/rest-api.md`](rest-api.md) |
| Welche Rolle darf `retry`/`resolve` aufrufen? | Nur der Admin-Key (`BPMNINJA_API_KEY`). Ein Worker-Key erhält `403`. | [`reference/rest-api.md`](rest-api.md) |
| Wie viele Incidents pro Instanz? | Maximal 100 (`MAX_INCIDENTS_PER_INSTANCE`). Beim Trimmen weichen aufgelöste vor offenen, jeweils die ältesten. | [`explanation/konzepte.md`](../explanation/konzepte.md) |
| Wie starte ich die Engine lokal? | `docker compose up -d nats`, dann `cargo run -p engine-server` (Port `8081`). | [`reference/cli.md`](cli.md) |

> **Anker:** Die Schnellverweise nennen Abschnitte als Text statt als Anker, weil Anker auf Überschriften mit Umlauten nicht im Zielrenderer geprüft sind (Style Guide R6.7).

---

## Verwandte Seiten außerhalb der Referenz

| Wenn du … | dann lies |
|---|---|
| BPMNinja zum ersten Mal ausprobieren willst | [`tutorials/erster-prozess.md`](../tutorials/erster-prozess.md) |
| ein Modell deployen oder versionieren willst | [`how-to/prozess-deployen.md`](../how-to/prozess-deployen.md) |
| einen Worker bauen willst | [`how-to/external-task-worker-schreiben.md`](../how-to/external-task-worker-schreiben.md) |
| die Engine für deine Umgebung konfigurieren willst | [`how-to/konfiguration-anpassen.md`](../how-to/konfiguration-anpassen.md) |
| verstehen willst, warum die Engine so gebaut ist | [`explanation/architektur.md`](../explanation/architektur.md) |
| die Konzepte hinter den Verträgen verstehen willst | [`explanation/konzepte.md`](../explanation/konzepte.md) |
| einen Begriff abgrenzen willst | [`_review/glossary.md`](../_review/glossary.md) |
| als Beitragende die Moduldokumentation brauchst | `docs/knowledge-base/index.md` (liegt außerhalb dieses Ausgabeverzeichnisses) |

---

## Offen

- **Geplante Seiten:** `reference/fehlercodes.md`, `reference/metriken.md` und `reference/glossar.md` sind im Dateiplan als P0/P1 geführt, aber zum Redaktionsstand noch nicht geschrieben. Diese Landingpage verlinkt sie deshalb **nicht**; die belegten Inhalte stehen ersatzweise in `reference/rest-api.md` (Fehlerformat) und `explanation/architektur.md` (Grenzen).
- **Glossar-Pfad:** Das verbindliche Glossar liegt unter `_review/glossary.md`, nicht unter `reference/glossar.md`. **Offen:** Entscheidung, ob das Glossar in die Referenzschicht verschoben oder als redaktionelles Dokument öffentlich verlinkt bleibt.
- **Endpunktzahl:** Eine belastbare Gesamtzahl existiert erst, wenn `docs/openapi.yaml` als generierte Quelle in CI geprüft wird (Audit AUD-006). Bis dahin nennt diese Referenz keine Zahl.
- **Persistierte Strukturnamen:** Die Bucket- und Store-Namen in `reference/konfiguration.md` §9 folgen `persistence-nats/src/client.rs`; die Knowledge Base nennt für zwei Buckets abweichende Namen. **Offen:** quellseitig bestätigen (technischer Review B-02), bevor Betriebsteams danach arbeiten.
- **Versionsstand:** Die Seiten nennen keinen Produktversionsstand („gültig ab"). `bpmninja/README.md` nennt Version 0.7.24 für Rust/Desktop und 1.0.0 für den npm-Client; eine Zuordnung der Referenzseiten zu diesen Versionen steht aus.
- **Ankerlinks:** Die Linkprüfung prüft Anker nicht. 57 Anker wurden separat gegen GitHub-Slug-Regeln geprüft (0 Defekte, `_qa/verification-report.md` §4); ein Rendering im Zielsystem ist nicht bewiesen.
