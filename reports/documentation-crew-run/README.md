---
title: BPMNinja — Dokumentation
description: Einstiegslandingpage der BPMNinja-Dokumentation — „Was möchtest du tun?", Zielgruppen mit Einstiegen, fünf Lesepfade mit Erfolgskriterium, eine Tabelle aller Seitentitel, ein Suchwegweiser, Voraussetzungen, Installationspfad, erste erfolgreiche Aktion, Hilfe bei Problemen, Barrierefreiheit und Metriken zum Einstieg.
audience:
  - Einsteiger und Lernende
  - Prozessmodellierer
  - Entwickler und Integratoren
  - Betriebsteams und Plattform-Administratoren
  - Stakeholder und Entscheider
prerequisites:
  - Keine Vorkenntnisse für den Einstieg über das Tutorial
  - Für den Schnellstart: Rust-Toolchain (cargo), Docker mit Docker Compose, curl
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - tutorials/erster-prozess.md
  - how-to/external-task-worker-schreiben.md
  - how-to/prozess-deployen.md
  - how-to/konfiguration-anpassen.md
  - reference/README.md
  - reference/rest-api.md
  - reference/service-task-api.md
  - reference/cli.md
  - reference/konfiguration.md
  - reference/bpmn-elemente.md
  - reference/kompatibilitaet.md
  - explanation/architektur.md
  - explanation/konzepte.md
  - _review/glossary.md
---

# BPMNinja — Dokumentation

BPMNinja ist eine in Rust geschriebene Engine, die BPMN-2.0-Prozesse token-basiert ausführt und ihren Zustand in NATS JetStream persistiert. Sie löst das Problem, Prozessmodelle reproduzierbar laufen zu lassen, ohne dass die Engine selbst zum Cluster werden muss: Arbeit wird über External Tasks an eigene Worker delegiert, der Zustand überlebt Neustarts, und jeder Wait State ist ein belegbarer Persistenzpunkt.

Diese Dokumentation ist nach Diátaxis in vier Bereiche geteilt. Jede Seite hat genau eine Aufgabe: lernen, eine Aufgabe erledigen, einen Vertrag nachschlagen oder eine Entscheidung verstehen.

> **Anrede:** Diese Dokumentation duzt durchgängig. Das ist eine Stilentscheidung, keine Wertung — sie gilt für alle Seiten, auch für diese.

## Was möchtest du tun?

Wähle die Zeile, die deiner Aufgabe am nächsten kommt. Jede Zeile führt direkt auf eine Seite, die diese Aufgabe löst.

| Ich möchte … | Startseite | Bereich | Zeit bis zum ersten Ergebnis |
|---|---|---|---|
| BPMNinja zum ersten Mal ausprobieren | [Ersten Prozess ausführen](tutorials/erster-prozess.md) | Tutorial | 10–15 Minuten |
| verstehen, was Token, Wait State und External Task bedeuten | [Zentrale Konzepte und Trade-offs](explanation/konzepte.md) | Explanation | 10 Minuten |
| einen Worker gegen die Engine bauen | [External-Task-Worker schreiben und testen](how-to/external-task-worker-schreiben.md) | How-to | 20–30 Minuten |
| ein Modell deployen, korrigieren oder versionieren | [Prozess deployen und Versionen verwalten](how-to/prozess-deployen.md) | How-to | 15–20 Minuten |
| die Engine für meinen Betrieb konfigurieren | [Konfiguration anpassen](how-to/konfiguration-anpassen.md) | How-to | 20 Minuten |
| prüfen, ob mein BPMN-Element unterstützt wird | [BPMN-Elemente und ihre Abbildung](reference/bpmn-elemente.md) | Referenz | 3–5 Minuten |
| prüfen, ob sich etwas wie in Camunda verhält | [Kompatibilität mit BPMN 2.0 und Camunda 7](reference/kompatibilitaet.md) | Referenz | 5 Minuten |
| einen Endpunkt, Statuscode oder Feldnamen nachschlagen | [REST-API](reference/rest-api.md) | Referenz | 1–2 Minuten |
| die Lock-Dauer oder `fetchAndLock`-Validierung prüfen | [Service-Task-API](reference/service-task-api.md) | Referenz | 1–2 Minuten |
| einen Build- oder Startbefehl finden | [CLI und Build-Befehle](reference/cli.md) | Referenz | 1–2 Minuten |
| einen Umgebungsvariablen-Default nachschlagen | [Konfiguration und Umgebungsvariablen](reference/konfiguration.md) | Referenz | 1–3 Minuten |
| bewerten, ob die Engine zu meinem Vorhaben passt | [Architektur verstehen](explanation/architektur.md) | Explanation | 20 Minuten |
| einen Begriff abgrenzen (Definition ↔ Instance) | [Glossar](_review/glossary.md) | Nachschlagen | 1 Minute |

> **Hinweis:** Das Glossar liegt unter `_review/glossary.md`, weil es zugleich Redaktionswerkzeug ist. Es ist die verbindliche Begriffsliste und für Leserinnen und Leser öffentlich gedacht.

## Zielgruppen und Einstiege

| Zielgruppe | Typische Frage | Einstieg | Danach |
|---|---|---|---|
| Einsteiger und Lernende | „Wie bekomme ich einen Prozess von XML bis zum sichtbaren Abschluss?" | [Ersten Prozess ausführen](tutorials/erster-prozess.md) | [Zentrale Konzepte und Trade-offs](explanation/konzepte.md) |
| Prozessmodellierer | „Wird mein Element unterstützt, und warum deployt mein Diagramm nicht?" | [BPMN-Elemente und ihre Abbildung](reference/bpmn-elemente.md) | [Prozess deployen und Versionen verwalten](how-to/prozess-deployen.md) |
| Entwickler und Integratoren | „Wie baue ich einen Worker, der Tasks holt und Fehler korrekt meldet?" | [External-Task-Worker schreiben und testen](how-to/external-task-worker-schreiben.md) | [Service-Task-API](reference/service-task-api.md) |
| Betriebsteams und Plattform-Administratoren | „Welche Keys, Grenzwerte und Prüfungen brauche ich für den Betrieb?" | [Konfiguration anpassen](how-to/konfiguration-anpassen.md) | [REST-API](reference/rest-api.md) (Auth, Health, Readiness) |
| Stakeholder und Entscheider | „Was kann die Engine bewusst nicht, und was kostet der Betrieb?" | [Architektur verstehen](explanation/architektur.md) | [Kompatibilität mit BPMN 2.0 und Camunda 7](reference/kompatibilitaet.md) |

## Lesepfade

Ein Lesepfad ist eine empfohlene Reihenfolge. Du musst ihn nicht einhalten — jede Seite funktioniert allein. Wenn du neu bist, sparen die Pfade Zeit.

### Pfad A — Einsteiger: „Ich will es einmal laufen sehen" (ca. 30 Minuten)

1. [Ersten Prozess ausführen](tutorials/erster-prozess.md) — Deploy, Start, User Task abschließen, `Completed` prüfen.
2. [Zentrale Konzepte und Trade-offs](explanation/konzepte.md) — warum die Engine anhält und was ein Token ist.
3. [Architektur verstehen](explanation/architektur.md) — wie der Zustand persistiert und beim Start wiederhergestellt wird.

**Fertig, wenn:** Du den Unterschied zwischen Process Definition und Process Instance ohne Nachschlagen erklären kannst.

### Pfad B — Entwickler: „Ich will einen Worker bauen" (ca. 60 Minuten)

1. [Ersten Prozess ausführen](tutorials/erster-prozess.md) — Voraussetzung: laufende Engine.
2. [External-Task-Worker schreiben und testen](how-to/external-task-worker-schreiben.md) — Erfolgs-, Incident- und Boundary-Pfad.
3. [Service-Task-API](reference/service-task-api.md) — Lock-Dauer, Validierungsgrenzen, Statuscodes.
4. [REST-API](reference/rest-api.md) — Rollenmatrix und Fehlerformat.

**Fertig, wenn:** Ein `failure` mit `retries: 0` einen Incident erzeugt, ein `bpmnError` in den Boundary-Pfad läuft und der Handler idempotent ist.

### Pfad C — Betrieb: „Ich will es produktiv betreiben" (ca. 45 Minuten)

1. [Konfiguration anpassen](how-to/konfiguration-anpassen.md) — `BIND_ADDR`, Keys, `REQUIRE_NATS`, Limits, Retention.
2. [REST-API](reference/rest-api.md) — Auth-Modi, `/api/health` vs. `/api/ready`, Statuscodes.
3. [Architektur verstehen](explanation/architektur.md) — Retry-Queue, Retention-Abgleich, Single-Writer-Grenze.
4. [CLI und Build-Befehle](reference/cli.md) — Startbefehle und Compose-Portbindungen.

**Fertig, wenn:** Die Prüfliste am Ende von „Konfiguration anpassen" vollständig grün ist.

### Pfad D — Stakeholder: „Ich will eine Eignungsentscheidung" (ca. 30 Minuten)

1. [Architektur verstehen](explanation/architektur.md) — Komponenten, Skalierung, bewusste Grenzen.
2. [Zentrale Konzepte und Trade-offs](explanation/konzepte.md) — was die Abweichungen zu BPMN 2.0 und Camunda kosten.
3. [Kompatibilität mit BPMN 2.0 und Camunda 7](reference/kompatibilitaet.md) — Abweichungen mit Richtung und Migrationscheckliste.

**Fertig, wenn:** Du die drei Betriebsrisiken nennen kannst (fail-closed Auth, begrenzte Retry-Queue mit Drop, Retention-Abgleich beim Start) und je eine Gegenmaßnahme kennst.

### Pfad E — Modellierer: „Ich will mein Modell deployen und versionieren" (ca. 25 Minuten)

1. [BPMN-Elemente und ihre Abbildung](reference/bpmn-elemente.md) — welche Elemente akzeptiert werden und welche den Deploy ablehnen.
2. [Prozess deployen und Versionen verwalten](how-to/prozess-deployen.md) — Version 1, Version 2, `start` vs. `start/latest`, Löschen mit `409`.
3. [Kompatibilität mit BPMN 2.0 und Camunda 7](reference/kompatibilitaet.md) — was sich gegenüber Camunda ändert.

**Fertig, wenn:** Zwei Versionen desselben Prozesses deployt sind und `POST /api/start/latest` die höhere Version startet.

## Alle Seiten auf einen Blick

Die Tabelle nennt jeden Seitentitel, den Bereich und den Zweck. Sie ist die vollständige Inhaltsübersicht dieses Bestands.

| Seite | Bereich | Was du dort bekommst | Zeit |
|---|---|---|---|
| [Ersten Prozess ausführen](tutorials/erster-prozess.md) | Tutorial | Deploy, Start, User Task abschließen, `Completed` prüfen — mit erwarteter Ausgabe je Schritt | 10–15 min |
| [External-Task-Worker schreiben und testen](how-to/external-task-worker-schreiben.md) | How-to | Worker mit `fetchAndLock`, `complete`, `failure`, `bpmnError` und Prüfliste | 20–30 min |
| [Prozess deployen und Versionen verwalten](how-to/prozess-deployen.md) | How-to | Version 1 und 2, `start` vs. `start/latest`, XML abrufen, Löschen mit `409` | 15–20 min |
| [Konfiguration anpassen](how-to/konfiguration-anpassen.md) | How-to | `BIND_ADDR`, Keys, `REQUIRE_NATS`, Limits, Logging, Retention | 20 min |
| [Referenz](reference/README.md) | Referenz | Einstieg in die Referenzschicht, Quellen der Wahrheit, Schnellverweise | 2 min |
| [REST-API](reference/rest-api.md) | Referenz | Routengruppen, Rollenmatrix, Statuscodes, Fehlerformat | 1–2 min |
| [Service-Task-API](reference/service-task-api.md) | Referenz | Lock-Dauer in ms, `fetchAndLock`-Validierung, sechs Ergebnis-Endpunkte | 1–2 min |
| [Konfiguration und Umgebungsvariablen](reference/konfiguration.md) | Referenz | Alle Variablen mit Default, Buckets, Object Stores, Streams | 1–3 min |
| [CLI und Build-Befehle](reference/cli.md) | Referenz | Build, Test, Lint, Engine-Start, Compose, Desktop-UI, Client | 1–2 min |
| [BPMN-Elemente und ihre Abbildung](reference/bpmn-elemente.md) | Referenz | XML → Laufzeittyp, Attributauflösung, Multi-Instance, Ablehnungsfälle | 3–5 min |
| [Kompatibilität mit BPMN 2.0 und Camunda 7](reference/kompatibilitaet.md) | Referenz | Alle Abweichungen mit Richtung und Migrationscheckliste | 5 min |
| [Architektur verstehen](explanation/architektur.md) | Explanation | Komponenten, Token-Ausführung, Persistenz, Skalierung, drei Betriebsrisiken | 20 min |
| [Zentrale Konzepte und Trade-offs](explanation/konzepte.md) | Explanation | Acht Konzepte, je mit Idee, Trade-off, Abweichung, Konsequenz | 10 min |
| [Glossar](_review/glossary.md) | Nachschlagen | 34 Begriffe mit Definition, Synonymen und Verwechslungsgefahr | 1 min |

## Suchen und finden

Dieses Verzeichnis hat keine eigene Volltextsuche; ob die veröffentlichende Website eine anbietet, gehört nicht zu diesen Dateien. Die Tabellen sind deshalb so gebaut, dass du ohne Suche ans Ziel kommst.

| Du suchst … | Erster Ort | Warum dort |
|---|---|---|
| ein Symptom (401, 503, 400, 409, leerer Worker) | [Hilfe bei Problemen](#hilfe-bei-problemen) | Zwölf Symptome mit Ursache und Zielabschnitt, seitenübergreifend |
| einen Vertrag (Statuscode, Feldname, Default, Grenzwert) | [Was suchst du?](reference/README.md#was-suchst-du) | Suchzeilen der Referenz, je mit Zielseite |
| einen Begriff oder eine Abgrenzung | [Glossar](_review/glossary.md) | Verbindliche Definition, Synonyme, „abzugrenzen von" |
| eine Reihenfolge oder einen Einstieg | [Lesepfade](#lesepfade), [Alle Seiten auf einen Blick](#alle-seiten-auf-einen-blick) | Pfad mit Dauer und Erfolgskriterium je Rolle |
| eine Begründung oder einen Trade-off | [Architektur verstehen](explanation/architektur.md), [Zentrale Konzepte](explanation/konzepte.md) | Entscheidungen, Alternativen, bewusste Grenzen |
| einen Vergleich mit Camunda | [Kompatibilität](reference/kompatibilitaet.md) | Abweichungen mit Richtung und Migrationscheckliste |

**Wenn du etwas nicht findest:** Prüfe zuerst `/api/health` und `/api/ready` — die beiden Antworten trennen „Prozess lebt" von „Engine kann Zustand halten". Wenn das nicht hilft, ist die Lücke ein Befund: Melde den Begriff, den du gesucht hast (siehe [Metriken](#metriken-für-diesen-einstieg)).

## Voraussetzungen für den Schnellstart

Der Schnellstart ist das Tutorial [Ersten Prozess ausführen](tutorials/erster-prozess.md). Du brauchst dafür:

- **Rust-Toolchain** mit `cargo` (das Repository baut über das Workspace-Manifest).
- **Docker** und **Docker Compose** für den NATS-JetStream-Broker.
- **curl** für die HTTP-Aufrufe; für den Deploy zusätzlich **jq**.
- Ein Terminal im Repository-Wurzelverzeichnis.
- Die Ports **8081** (Engine) und **4222** (NATS) lokal frei.

> **Sicherheitshinweis:** Die Engine bindet standardmäßig an `0.0.0.0` (`BIND_ADDR`). Ohne gesetzten API-Key antworten alle `/api`-Routen dann mit **401** (fail-closed). Für den lokalen Schnellstart setzt du `BIND_ADDR=127.0.0.1`. Details: [CLI und Build-Befehle](reference/cli.md) und [Konfiguration anpassen](how-to/konfiguration-anpassen.md).

## Installationspfad

Es gibt drei Wege, BPMNinja zum Laufen zu bringen. Wähle einen — du brauchst nicht alle.

| Weg | Für wen | Erster Befehl | Danach |
|---|---|---|---|
| **Quellcode** (empfohlen für Einsteiger) | Du willst das Tutorial mitmachen oder am Code arbeiten | `docker compose up -d nats`, dann `BIND_ADDR=127.0.0.1 cargo run -p engine-server` | [Tutorial, Schritt 1](tutorials/erster-prozess.md#schritt-1-nats-mit-jetstream-starten) |
| **Container-Image** | Du willst die Engine ohne Rust-Toolchain starten | `docker compose up --build` | [CLI und Build-Befehle, §4](reference/cli.md) |
| **Desktop-App** | Du willst Modelle zeichnen und Instanzen ansehen | Fertige Binaries aus den GitHub Releases (`.dmg`, `.exe`/`.msi`, `.AppImage`/`.deb`) | [CLI und Build-Befehle, §5](reference/cli.md) |

> **Offen:** Ein dokumentierter Ein-Befehl-Installer existiert nicht. Die drei Wege oben sind die im Repository belegten. Ob es weitere Distributionswege gibt, ist nicht belegt.

## Erste erfolgreiche Aktion

Das ist der kürzeste Weg zu einem sichtbaren Erfolg. Er setzt voraus, dass NATS und die Engine laufen (Schritte 1 und 2 des Tutorials).

```bash
# 1. Engine-Status prüfen — erwartet: 200 (leerer Body)
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8081/api/health

# 2. Readiness prüfen — erwartet: 200
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8081/api/ready
```

Wenn beide Aufrufe das erwartete Ergebnis liefern, ist die Engine bereit. Der nächste Schritt ist der Deploy: [Tutorial, Schritt 4](tutorials/erster-prozess.md#schritt-4-prozess-deployen).

## Hilfe bei Problemen

| Symptom | Wahrscheinliche Ursache | Wo die Lösung steht |
|---|---|---|
| Alle `/api`-Routen antworten mit **401** | Kein Key gesetzt, Bind-Adresse nicht Loopback (Default `0.0.0.0`) | [Konfiguration anpassen — Fehler 1](how-to/konfiguration-anpassen.md#fehler-1-alle-api-routen-antworten-mit-401) |
| `/api/ready` antwortet mit **503** oder der Server startet nicht | NATS/JetStream nicht erreichbar, `REQUIRE_NATS=true` (Default) | [Konfiguration anpassen — Fehler 2](how-to/konfiguration-anpassen.md#fehler-2-apiready-bleibt-503-oder-die-engine-startet-nicht) |
| Deploy antwortet mit **400** und `InvalidDefinition` | Namespace, abgelehntes Element, mehrere `<process>`-Elemente | [BPMN-Elemente](reference/bpmn-elemente.md), [Deployen — Fehler 1](how-to/prozess-deployen.md#fehler-1-deploy-antwortet-mit-400-und-invaliddefinition) |
| Deploy antwortet mit **415** oder **422** | Falscher `Content-Type` oder fehlendes `name`/`xml` | [Deployen — Fehler 2](how-to/prozess-deployen.md#fehler-2-deploy-antwortet-mit-415-oder-422) |
| Löschen einer Definition antwortet mit **409** | Es laufen noch Instanzen auf dieser Version | [Deployen — Fehler 3](how-to/prozess-deployen.md#fehler-3-löschen-antwortet-mit-409) |
| Neue Instanz läuft auf der alten Version | `POST /api/start` mit alter `definition_key`, oder `<process id>` geändert | [Deployen — Fehler 4](how-to/prozess-deployen.md#fehler-4-die-neue-instanz-läuft-auf-der-alten-version) |
| `fetchAndLock` antwortet mit **400**, der Worker holt nie Tasks | `lockDuration` in Sekunden, `maxTasks` außerhalb `1..=100`, Topic-Schreibweise | [Worker — Fehler 1](how-to/external-task-worker-schreiben.md#fehler-1-fetchandlock-antwortet-mit-400-und-der-worker-holt-nie-tasks) |
| Ein Task wird doppelt bearbeitet oder `complete` antwortet mit **409** | Sperre abgelaufen, bevor `complete` ankam | [Worker — Fehler 2](how-to/external-task-worker-schreiben.md#fehler-2-complete-antwortet-mit-409-oder-der-task-wird-doppelt-bearbeitet) |
| Ein `bpmnError` erzeugt einen Incident statt in den Boundary-Pfad zu laufen | `errorCode` passt zu keinem `<bpmn:error>` des Boundary Event | [Worker — Fehler 3](how-to/external-task-worker-schreiben.md#fehler-3-bpmnerror-erzeugt-einen-incident-statt-in-den-boundary-pfad-zu-laufen) |
| Nach einem Upgrade fehlen ältere Historieneinträge | Retention-Abgleich beim ersten Start hat Limits gesenkt | [Konfiguration anpassen — Fehler 3](how-to/konfiguration-anpassen.md#fehler-3-nach-dem-upgrade-fehlen-ältere-historieneinträge) |
| Die Log-Datei füllt das Dateisystem | `LOG_FILE` gesetzt, aber keine Rotation | [Konfiguration anpassen — Fehler 4](how-to/konfiguration-anpassen.md#fehler-4-die-log-datei-füllt-das-dateisystem) |
| Ein Begriff ist unklar (Definition, Instance, Token, Wait State, Topic) | – | [Glossar](_review/glossary.md) |

**Wenn hier nichts passt:** Prüfe zuerst `/api/health` und `/api/ready`. Diese beiden Antworten trennen „Prozess lebt" von „Engine kann Zustand halten" und schließen die häufigsten Ursachen aus.

**Wenn ein Fehler fehlt:** Melde ihn mit dem Symptom, der wörtlichen Fehlermeldung und der Seite, auf der du warst. Jede Meldung, die eine neue Zeile in dieser Tabelle erzeugt, ist ein belegter Dokumentationsfehler — siehe [Metriken](#metriken-für-diesen-einstieg).

## Barrierefreiheit und mobile Nutzung

Die Regeln stehen in `_review/style-guide.md` §7. Diese Zusagen gelten für alle Seiten:

- **Tabellen** haben eine Kopfzeile, und die erste Spalte benennt den Datensatz. Die Tabellen dieser Seite haben höchstens fünf Spalten, damit sie auf schmalen Bildschirmen lesbar bleiben.
- **Linktexte** nennen das Ziel, nie „hier" oder „klicken". Dadurch bleibt ein Link in einer Screenreader-Liste verständlich.
- **Statuscodes** tragen immer ein Textäquivalent; Farbe und Symbol sind nie der einzige Bedeutungsträger.
- **Diagramme:** Alle zehn Mermaid-Diagramme tragen einen Alt-Text und wurden am 2026-10-24 gerendert (`_qa/verification-report.md` §3).
- **Code-Blöcke** tragen durchgängig eine Sprachkennung, damit Syntaxhervorhebung und Vorlesehilfen greifen.
- **Anker:** 57 interne Anker wurden gegen die GitHub-Slug-Regeln geprüft, 0 Defekte (`_qa/verification-report.md` §4). Ein Rendering im Zielsystem ist damit wahrscheinlich, aber nicht bewiesen; die betroffenen Seiten führen den Hinweis unter **Offen**.

## Metriken für diesen Einstieg

Diese Tabelle ist die Messvorgabe für den Einstieg. Sie ist **nicht erhoben** — es wurden keine Nutzertests durchgeführt. Alle Zielwerte sind Annahmen, keine Messergebnisse.

| Metrik | Definition | Zielwert | Stand |
|---|---|---|---|
| **Time to First Success (TTFS)** | Zeit von der ersten Landingpage-Ansicht bis zum ersten `200` aus `/api/health` | ≤ 10 Minuten bei 4 von 5 Testpersonen | nicht erhoben |
| **Absprungstellen** | Seite und Abschnitt, an dem eine Sitzung endet, ohne dass ein Abnahmekriterium erreicht wurde | ≤ 25 % Abbruch vor Tutorial-Schritt 4 | nicht erhoben |
| **Support-Fragen** | Fragen, die „Hilfe bei Problemen" nicht beantwortet | ≤ 2 neue Themen pro Monat | nicht erhoben |
| **Linkintegrität** | Relative Datei- und Bildlinks, die auflösen | 100 % | 0 Defekte in veröffentlichten Seiten; Anker: 57 geprüft, 0 Defekte (`_qa/verification-report.md` §4) |
| **Seitenabschluss** | Anteil der Seiten mit Fertig-Kriterium (Tutorial, How-to) bzw. `## Offen` (alle) | 100 % | 4 von 4 Tutorial/How-tos mit Fertig-Kriterium; alle veröffentlichten Seiten mit `## Offen` |

## Status dieser Dokumentation

Alle veröffentlichten Seiten tragen Front Matter mit `status` und `last_reviewed`. Der Stand ist **draft**: inhaltlich vollständig. REST-Beispiele und Mermaid-Diagramme sind am 2026-10-24 gegen eine laufende Engine geprüft (`_qa/verification-report.md`). **Nicht** abgeglichen: `docs/openapi.yaml` (bekannte Abweichung bei `bpmnError`), das npm-Paket des TypeScript-Clients, Last-, Timeout- und Fehlerpfade jenseits der belegten Statuscodes. Offene Punkte stehen auf jeder Seite unter **Offen** und dürfen nicht als Zusage gelesen werden.

## Offen

- **Fehlende Seiten:** Der Dateiplan der Informationsarchitektur führt weitere Seiten (`reference/fehlercodes.md`, `reference/metriken.md`, `reference/glossar.md`, `how-to/betrieb/produktion-checkliste.md`, `how-to/betrieb/nats-ausfall.md`, `how-to/betrieb/incident-triage.md`, `explanation/grenzen-und-trade-offs.md`). Sie existieren zum Redaktionsstand nicht und sind deshalb **nicht verlinkt**.
- **Fehlerseite (404):** Eine eigene Fehlerseite für nicht gefundene Pfade gehört zur veröffentlichenden Website, nicht in dieses Verzeichnis. Sie fehlt; ein Vorschlag für ihren Text steht in `_review/ux-review.md` §5 (R-07).
- **Ankerlinks:** 57 Anker sind gegen GitHub-Slug-Regeln geprüft (0 Defekte). Ob der Zielrenderer dieselben Slugs bildet, ist nicht bewiesen — deshalb führen die Quellseiten den Hinweis weiter.
- **Konfiguration:** Die kanonische Referenz für Umgebungsvariablen steht in [`reference/konfiguration.md`](reference/konfiguration.md). Die Bucket-Namen in §9 sind quellseitig noch nicht abschließend bestätigt (technischer Review B-02).
- **Seitenpfade:** Der Dateiplan nennt `reference/index.md` und `explanation/architektur-ueberblick.md`; der Bestand nutzt `reference/README.md` und `explanation/architektur.md`. Die Angleichung steht aus.
- **Metriken:** Alle Zielwerte sind Annahmen. Erhebungsmethode und Nutzertest-Design stehen in `_review/ux-review.md` §7.
- **`content_type`:** Diese Landingpage ist als `reference` deklariert, weil die Werteliste keinen Typ `landing` kennt (Style Guide R5.4).
