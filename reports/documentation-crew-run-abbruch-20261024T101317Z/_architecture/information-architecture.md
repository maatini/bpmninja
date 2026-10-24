# BPMNinja-Dokumentation — Informationsarchitektur, Redaktionsplan und Landing-Page

**Redaktionsstand:** 2026-07-17 · **Status:** Entwurf zur Umsetzung — **keine Freigabe** (Baseline-Score 54/100, siehe `_audit/audit-report.md`)
**Grundlage:** `_research/domain-research.md`, `_audit/audit-report.md`, Quellinventar des Repositories `/Volumes/SSD2TB/work/bpmninja`
**Sprache:** Deutsch (verbindlich, siehe §5.3 und A-12)

> **Entscheidung des Redaktionsleiters vorab:** Die vier Dateien in diesem Dokument sind der **Bauplan**. Sie ersetzen keine Inhalte. Jede strukturelle Entscheidung nennt die Nutzeraufgabe, die sie löst. Wo eine Aussage nicht belegt ist, steht **„Codeabgleich offen"** — solche Aussagen dürfen nicht in veröffentlichte Seiten wandern, bevor das Engineering sie bestätigt hat.

---

## Datei 1 — `_architecture/information-architecture.md`

```markdown
---
title: Informationsarchitektur der BPMNinja-Dokumentation
description: Zielgruppen, Diátaxis-Zuordnung, Navigationskonzept, Front-Matter-Standard und Qualitätskriterien für die Nutzerdokumentation von BPMNinja.
audience: [dokumentationsteam, engineering, produktverantwortliche]
prerequisites: [_audit/audit-report.md, _research/domain-research.md]
content_type: explanation
status: entwurf
last_reviewed: 2026-07-17
owner: documentation-lead
related: [_architecture/content-plan.md, _architecture/migration-map.md, README.md]
---

# Informationsarchitektur der BPMNinja-Dokumentation

## 0. Zweck und Geltungsbereich

Dieses Dokument legt fest, **für wen** die BPMNinja-Dokumentation geschrieben wird, **welche Art von Inhalt** wohin gehört, **wie Nutzer navigieren** und **wer für die Pflege verantwortlich ist**. Es ist die verbindliche Grundlage für alle Seiten unter `docs/`.

Es gilt **nicht** für `docs/knowledge-base/`. Die Knowledge Base bleibt bestehen und behält ihre Zielgruppe („coding agents (AI and human)", `docs/knowledge-base/index.md:3`). Sie wird über eine klar gekennzeichnete Brücke erschlossen (§4.5), nicht umgeschrieben.

**Leitentscheidung:** Die Nutzerdokumentation wird **neu aufgebaut** und **verweist** auf die KB, statt sie zu duplizieren. Grund: Die KB enthält belegte, präzise Fakten (Statuscodes, Grenzwerte, Invarianten), die für Nutzer wertvoll sind — aber in einer für Agenten geschriebenen Form. Eine Kopie würde bei jeder Codeänderung doppelt veralten. Ein Verweis veraltet einmal.

---

## 1. Nutzergruppen und ihre Aufgaben

Fünf Gruppen. Jede hat **einen** Einstieg, **eine** typische Frage, **ein** messbares Erfolgskriterium und **einen** bevorzugten Diátaxis-Content-Typ. Die Zuordnung folgt der Diátaxis-Kompassregel (siehe §2.1).

### 1.1 Einsteiger und Lernende

| Merkmal | Ausprägung |
|---|---|
| **Wer** | Entwickler, die BPMNinja zum ersten Mal sehen; Studierende; Evaluierer mit 30 Minuten Zeit |
| **Einstieg** | `README.md` → `docs/tutorials/erster-prozess.md` |
| **Typische Frage** | „Wie bekomme ich in 15 Minuten einen Prozess zum Laufen?" |
| **Erfolgskriterium** | Eine Prozessinstanz ist gestartet, ein User Task wurde abgeschlossen, die Instanz steht auf `completed` — ohne dass die Person eine zweite Datei öffnen musste. |
| **Bevorzugter Content-Typ** | **Tutorial** (action + acquisition) |
| **Nicht hilfreich** | Architekturtabellen, Env-Variablen-Referenzen, Alternativvergleiche |
| **Löst Audit-Befund** | A-01 (kein Einstiegspunkt), A-11 (kein nächster Schritt) |

### 1.2 Entwickler und Integratoren

| Merkmal | Ausprägung |
|---|---|
| **Wer** | Backend-Entwickler, die einen External-Task-Worker bauen; Teams, die von Camunda migrieren; Beitragende am Repository |
| **Einstieg** | `README.md` → `docs/how-to/external-task-worker-schreiben.md` bzw. `docs/how-to/von-camunda-migrieren.md` |
| **Typische Frage** | „Wie schreibe ich einen Worker, der den Vertrag einhält — und was muss ich an meinem Camunda-Modell ändern?" |
| **Erfolgskriterium** | Ein Worker holt Tasks per `fetchAndLock`, schließt sie ab, und der Entwickler kann `failure` von `bpmnError` unterscheiden und die Einheit von `lockDuration` korrekt belegen. |
| **Bevorzugter Content-Typ** | **How-to Guide** (action + application), gestützt auf **Reference** |
| **Nicht hilfreich** | Tutorial-Schritt-für-Schritt für Dinge, die die Person schon kann; Marketing |
| **Löst Audit-Befund** | A-02 (kein Worker-How-to), A-04 (keine Camunda-Migration), A-05 (Einheiten-Widerspruch) |

### 1.3 Betriebsteams und Plattform-Administratoren

| Merkmal | Ausprägung |
|---|---|
| **Wer** | SRE, Plattform-Teams, die BPMNinja deployen, überwachen und im Störfall betreiben |
| **Einstieg** | `README.md` → `docs/how-to/deployment-absichern.md` → `docs/how-to/monitoring-einrichten.md` |
| **Typische Frage** | „Warum bekomme ich 401 auf allen Routen? Welche Metrik alarmiere ich? Was tue ich, wenn `bpmn_persistence_retry_dropped_total` steigt?" |
| **Erfolgskriterium** | Ein Deployment mit `BIND_ADDR=0.0.0.0` ist abgesichert (Keys gesetzt, CORS begrenzt), Health/Ready sind überwacht, und für den Persistenzausfall existiert ein durchgespieltes Runbook. |
| **Bevorzugter Content-Typ** | **How-to Guide** + **Runbook** (Runbook = How-to mit Alarmbezug) |
| **Nicht hilfreich** | Gotchas in Agenten-Sprache; verstreute Einzelfakten |
| **Löst Audit-Befund** | A-03 (keine Betriebsdoku), A-07 (Bucket-Namen), A-08 (History-Modell) |

### 1.4 Stakeholder und Entscheider

| Merkmal | Ausprägung |
|---|---|
| **Wer** | Tech-Leads, Architekten, Produktverantwortliche, die eine Adoptions- oder Ablöseentscheidung treffen |
| **Einstieg** | `README.md` → `docs/explanation/abweichungen-von-bpmn-2-0.md` |
| **Typische Frage** | „Was kann BPMNinja nicht — und welche Konsequenz hat das für unser Modell?" |
| **Erfolgskriterium** | Die Person kann die fünf gravierendsten Abweichungen (Multi-Instance, Sub-Prozess-Flattening, Rhai statt JS/Groovy, ein `<process>` pro Datei, Task-Typ-Mapping) benennen und gegen das eigene Portfolio bewerten. |
| **Bevorzugter Content-Typ** | **Explanation** (cognition + acquisition) |
| **Nicht hilfreich** | Endpunkt-Tabellen, Code-Beispiele ohne Kontext |
| **Löst Audit-Befund** | A-04, A-10 (README-Mischform), Domänenrecherche §1 („Abweichungen sind der wichtigste Dokumentationsgegenstand") |

### 1.5 Beitragende und Maintainer (Querschnittsgruppe)

| Merkmal | Ausprägung |
|---|---|
| **Wer** | Personen, die Dokumentation oder Code beitragen; das Redaktionsteam selbst |
| **Einstieg** | `docs/knowledge-base/maintenance.md` (bestehend) + §7 dieses Dokuments |
| **Typische Frage** | „Wo trage ich eine neue Env-Variable ein, ohne dass sie an drei Stellen veraltet?" |
| **Erfolgskriterium** | Jede neue Tatsache hat **genau eine** autoritative Stelle; alle anderen verweisen darauf. |
| **Bevorzugter Content-Typ** | **Reference** (Single Source of Truth) + Redaktionsregeln |
| **Löst Audit-Befund** | A-05 bis A-09 (Widersprüche durch Mehrfachnennung) |

---

## 2. Diátaxis-Zuordnung

### 2.1 Entscheidungsregel

Der Diátaxis-Kompass (diataxis.fr/compass/) unterscheidet zwei Achsen: **Handlung vs. Kognition** und **Aneignung vs. Anwendung**.

| Achse | Frage | Content-Typ | Verzeichnis |
|---|---|---|---|
| Handlung + Aneignung | „Ich lerne, indem ich es tue" | **Tutorial** | `docs/tutorials/` |
| Handlung + Anwendung | „Ich muss eine konkrete Aufgabe lösen" | **How-to Guide** | `docs/how-to/` |
| Kognition + Anwendung | „Ich brauche eine Tatsache nachzuschlagen" | **Reference** | `docs/reference/` |
| Kognition + Aneignung | „Ich will verstehen, warum" | **Explanation** | `docs/explanation/` |

**Praxistest für jede neue Seite:** Wenn ein Satz mit „In diesem Tutorial lernst du…" beginnt → Tutorial. Wenn er mit „So erreichst du…" beginnt → How-to. Wenn er als Tabelle funktioniert → Reference. Wenn er mit „Weil…" beginnt → Explanation.

### 2.2 Was gehört wohin — mit Nutzeraufgabe

| Inhalt | Zielverzeichnis | Nutzeraufgabe, die es löst |
|---|---|---|
| Erster lauffähiger Prozess (StartEvent → UserTask → EndEvent) | `docs/tutorials/erster-prozess.md` | „Ich will BPMNinja einmal erfolgreich durchlaufen" |
| External-Task-Worker von Null bauen | `docs/how-to/external-task-worker-schreiben.md` | „Ich will meinen Service an die Engine anbinden" |
| Deployment absichern (Auth, CORS, Bind, Limits) | `docs/how-to/deployment-absichern.md` | „Ich will nicht versehentlich offen deployen" |
| NATS/JetStream konfigurieren (Retention, Credentials) | `docs/how-to/nats-konfigurieren.md` | „Ich will Persistenz korrekt dimensionieren" |
| Monitoring einrichten (Health/Ready, Metriken) | `docs/how-to/monitoring-einrichten.md` | „Ich will Ausfälle sehen, bevor Nutzer sie melden" |
| Logs verwalten (Rotation, `LOG_FILE=off`, `ENGINE_LOGS`) | `docs/how-to/logs-verwalten.md` | „Ich will verhindern, dass `engine_logs.jsonl` die Platte füllt" |
| Camunda-Modell migrieren | `docs/how-to/von-camunda-migrieren.md` | „Ich will mein bestehendes Modell übertragen" |
| Persistenzausfall behandeln | `docs/runbooks/persistenz-ausfall.md` | „Ich will im Störfall handeln, nicht suchen" |
| REST-API (Endpunkte, Statuscodes, Payloads) | `docs/reference/rest-api.md` + Detailseiten | „Ich will einen Aufruf korrekt formulieren" |
| Umgebungsvariablen und Defaults | `docs/reference/umgebungsvariablen.md` | „Ich will wissen, was ich setzen muss" |
| BPMN-Element-Unterstützung (29 Varianten) | `docs/reference/bpmn-elemente.md` | „Ich will wissen, ob mein Element deploybar ist" |
| External-Task-Vertrag (Felder, Grenzen, Fehlercodes) | `docs/reference/external-task-vertrag.md` | „Ich will den Vertrag exakt einhalten" |
| Abweichungen von BPMN 2.0 | `docs/explanation/abweichungen-von-bpmn-2-0.md` | „Ich will die Grenzen verstehen und bewerten" |
| Token-basiertes Ausführungsmodell | `docs/explanation/ausfuehrungsmodell.md` | „Ich will verstehen, warum sich Prozesse so verhalten" |
| Architekturentscheidungen (ADRs) | `docs/explanation/architektur.md` | „Ich will die Bauentscheidungen nachvollziehen" |
| Glossar (verwechselbare Begriffe) | `docs/reference/glossar.md` | „Ich will Prozessdefinition von Prozessinstanz unterscheiden" |

### 2.3 Mischformen und ihre Trennung

Der Audit nennt drei Mischformen. Jede wird nach einer festen Regel getrennt.

**Mischform 1: `README.md` (4.998 Wörter, alle vier Quadranten — A-10).**
*Regel:* Der README wird **Landing-Page** (≤ 800 Wörter). Er enthält: Was ist BPMNinja (3 Sätze), Zielgruppen-Tabelle mit Links, 5 Startbefehle, Linktabelle in die vier Quadranten, Status-Hinweis. Alles andere wandert in die Zielverzeichnisse (siehe `_architecture/migration-map.md`).
*Nutzeraufgabe:* „Ich will in 30 Sekunden wissen, ob ich hier richtig bin und wo ich weiterklicke."

**Mischform 2: Gotchas (Hybrid aus Reference und Explanation — A-22).**
*Regel:* Gotchas bleiben in der KB (Agenten-Zielgruppe). Für Nutzer wird **jede** Gotcha klassifiziert:
- Enthält sie eine **Tatsache** (Grenzwert, Name, Statuscode) → wandert als Zeile in die zuständige Reference-Seite.
- Enthält sie eine **Handlungsanweisung** („setze X, sonst Y") → wandert als Schritt in das zuständige How-to.
- Enthält sie ein **Warum** („weil der Lock über `.await` gehalten würde") → wandert als Absatz in die zuständige Explanation-Seite.
Die Gotcha-Datei selbst wird **nicht** gelöscht, sondern verlinkt aus der Reference („Hintergrund für Agenten und Maintainer").

**Mischform 3: `docs/architecture.md` (2.833 Wörter, Stand 2026-04-10 — A-15).**
*Regel:* Die Datei ist **Explanation** und wird nach `docs/explanation/architektur.md` übernommen — aber **erst nach inhaltlicher Aktualisierung** (Incidents, Auth, `MAX_XML_BYTES`, `BIND_ADDR` fehlen). Bis dahin bleibt sie am alten Ort und wird als „veraltet" markiert. Die Reference-Anteile (Route-Overview, Bucket-Tabellen) werden **herausgelöst** und in `docs/reference/` geführt.

**Mischform 4: Sprachmix Deutsch/Englisch (A-12).**
*Regel:* Nutzerdokumentation ist **durchgehend Deutsch**. Fachbegriffe bleiben englisch, wenn sie im Code oder in der API so heißen (`fetchAndLock`, `lockDuration`, `Service Task`, `Incident`). Beim ersten Vorkommen: deutsch in Klammern („Der Lock (`lockDuration`) …"). Die KB bleibt gemischt — sie ist nicht Gegenstand dieses Standards.

---

## 3. Front-Matter-Standard

Jede Datei unter `docs/tutorials/`, `docs/how-to/`, `docs/reference/`, `docs/explanation/`, `docs/runbooks/` beginnt mit exakt diesem YAML-Block. Felder sind **verpflichtend**, sofern nicht anders markiert.

```yaml
---
title: "Erster Prozess in 15 Minuten"          # ≤ 60 Zeichen, deutsch, kein Punkt am Ende
description: "Ein BPMN-Prozess von Deployment bis Abschluss — Schritt für Schritt."  # ≤ 160 Zeichen, erscheint in Suche und Vorschau
audience: [einsteiger]                          # Liste aus: einsteiger | entwickler | betreiber | stakeholder | beitragende
prerequisites: []                               # Liste von Dateipfaden oder externen Voraussetzungen (z. B. "Docker installiert")
content_type: tutorial                          # tutorial | how-to | reference | explanation | runbook
status: entwurf                                 # entwurf | review | freigegeben | veraltet
last_reviewed: 2026-07-17                       # ISO-Datum der letzten inhaltlichen Prüfung
owner: documentation-lead                       # Rolle, nicht Person (siehe §7)
related: []                                     # Liste von Dateipfaden (Querverweise, max. 5)
---
```

### 3.1 Feldregeln

| Feld | Regel | Warum (Nutzeraufgabe) |
|---|---|---|
| `title` | Deutsch, ≤ 60 Zeichen, beschreibt die **Aufgabe** oder den **Gegenstand**, nicht die Zielgruppe | Nutzer erkennt im Suchergebnis, ob die Seite passt |
| `description` | Ein Satz, ≤ 160 Zeichen, beginnt mit einem Verb oder Substantiv | Erscheint in Suchergebnis und Link-Vorschau; entscheidet über Klick |
| `audience` | Mindestens ein Wert; bei mehreren Zielgruppen **eine** Seite pro Zielgruppe anlegen, nicht mischen | Verhindert die README-Mischform (A-10) |
| `prerequisites` | Leer nur bei Explanation; Tutorials und How-tos nennen jede Voraussetzung | Verhindert die „Wand" aus A-01 |
| `content_type` | Genau ein Wert; bestimmt das Verzeichnis | Erzwingt Diátaxis-Trennung (A-22) |
| `status` | `entwurf` → `review` → `freigegeben`; `veraltet` bei überfälliger Prüfung | Nutzer sieht, ob er sich verlassen kann |
| `last_reviewed` | ISO-Datum; wird bei **inhaltlicher** Prüfung gesetzt, nicht bei Formatänderung | Aktualität ist messbar (A-15–A-17) |
| `owner` | Rolle aus §7, nicht Personenname | Pflege überlebt Personalwechsel |
| `related` | Max. 5 Pfade; nur Dateien, die **existieren oder im Content-Plan stehen** | Verhindert tote Links (A-18) |

### 3.2 Statusregeln

- `entwurf`: Inhalt unvollständig oder unbelegt. **Darf nicht** aus dem README verlinkt werden.
- `review`: Inhalt vollständig, technische Aussagen belegt (Codeabgleich erfolgt). Wartet auf Freigabe.
- `freigegeben`: Alle Abnahmekriterien aus §7.3 erfüllt. Darf verlinkt werden.
- `veraltet`: `last_reviewed` älter als 180 Tage **oder** eine referenzierte Code-Wahrheit hat sich geändert. Bleibt erreichbar, wird aber oben mit einem Hinweisbanner versehen.

---

## 4. Navigationskonzept

### 4.1 Landing-Page (`README.md`)

Der README ist die **einzige** Seite, die alle vier Zielgruppen bedient — und zwar durch **Weiterleitung**, nicht durch Inhalt. Aufbau (≤ 800 Wörter):

1. **Was ist BPMNinja** — 3 Sätze, keine Feature-Liste.
2. **Für wen ist diese Dokumentation** — Tabelle „Ich bin … → Ich will … → Hier entlang" (5 Zeilen, §1).
3. **In 5 Befehlen starten** — NATS mit JetStream, Engine mit gesetzten Keys, Health-Check. Kein Docker-Compose ohne Auth (A-10).
4. **Wo finde ich was** — Linktabelle in die vier Diátaxis-Verzeichnisse mit je einem Satz.
5. **Status und Grenzen** — Verweis auf `docs/explanation/abweichungen-von-bpmn-2-0.md`; Hinweis, dass die KB für Agenten ist.
6. **Nächster Schritt** — ein Absatz, der Einsteiger ins Tutorial schickt (A-11).

### 4.2 Inhaltsverzeichnisse (TOCs)

Jedes der fünf Verzeichnisse (`tutorials/`, `how-to/`, `reference/`, `explanation/`, `runbooks/`) erhält eine `index.md` mit:
- Einem Satz, welche Frage dieses Verzeichnis beantwortet.
- Einer Tabelle: Titel · Zielgruppe · Voraussetzungen · Status.
- Einem „Wenn Sie hier nicht fündig werden"-Absatz mit Verweis auf die drei anderen Quadranten.

*Nutzeraufgabe:* „Ich weiß, dass ich eine Anleitung brauche, aber nicht welche."

### 4.3 Querverweise

Feste Regel: **Jede Seite verweist auf mindestens eine Seite eines anderen Quadranten.**
- Tutorial → Reference („Die Felder im Detail: …") und How-to („Wenn du einen Worker brauchst: …").
- How-to → Reference (Vertrag) und Explanation („Warum das so ist: …").
- Reference → Explanation (Hintergrund) und KB (Agenten-Detail).
- Explanation → Reference (Beleg) und How-to („In der Praxis: …").

*Nutzeraufgabe:* „Ich bin fertig mit dieser Seite — was jetzt?" (A-11)

### 4.4 Breadcrumbs

Jede Seite trägt unter dem Titel eine Breadcrumb-Zeile:
`Dokumentation › How-to Guides › External-Task-Worker schreiben`
Die Breadcrumb verlinkt auf `README.md` und die `index.md` des Verzeichnisses. Keine tieferen Ebenen — die Verzeichnisstruktur ist flach (max. eine Ebene unter `docs/`).

### 4.5 Brücke zur Knowledge Base

Die KB wird **nicht** in die Nutzernavigation eingebunden. Stattdessen:
- Jede Reference-Seite endet mit dem Block **„Für Maintainer und Agenten"** und verlinkt die zuständige KB-Datei.
- `docs/reference/index.md` enthält einen Absatz: „Die Knowledge Base (`docs/knowledge-base/`) ist die technische Tiefenquelle. Sie ist für Agenten und Maintainer geschrieben, nicht für Endnutzer."
- `docs/knowledge-base/index.md` erhält einen Kopfhinweis: „Diese KB ist **keine** Nutzerdokumentation. Einstieg für Menschen: `README.md`."

*Nutzeraufgabe:* „Ich will nicht versehentlich in einer Agenten-Datei landen." (A-18)

### 4.6 „Was möchten Sie tun?"-Einstiege

Der README enthält eine Tabelle mit **Aufgaben** statt Rollen — für Nutzer, die ihre Rolle nicht kennen:

| Was möchten Sie tun? | Seite |
|---|---|
| BPMNinja zum ersten Mal ausprobieren | `docs/tutorials/erster-prozess.md` |
| Einen Service an die Engine anbinden | `docs/how-to/external-task-worker-schreiben.md` |
| Ein Camunda-Modell übertragen | `docs/how-to/von-camunda-migrieren.md` |
| Sicher deployen | `docs/how-to/deployment-absichern.md` |
| Einen Ausfall behandeln | `docs/runbooks/persistenz-ausfall.md` |
| Einen Endpunkt nachschlagen | `docs/reference/rest-api.md` |
| Verstehen, was nicht geht | `docs/explanation/abweichungen-von-bpmn-2-0.md` |

### 4.7 Suchbegriffe

Da der Bestand keine Suche hat (A-18), werden Suchbegriffe **im Text** verankert. Verbindliche Synonyme, die in `description` oder erster Absatzzeile vorkommen müssen:

| Begriff | Muss zusätzlich genannt werden |
|---|---|
| External Task | Service Task, Worker, `fetchAndLock` |
| Lock | `lockDuration`, `extendLock`, Millisekunden |
| Deployment | Deploy, `POST /api/deployments`, BPMN-XML |
| Incident | Fehler, `resolve`, `retry` |
| Persistenz | NATS, JetStream, KV, Stream, Retention |
| Auth | `BPMNINJA_API_KEY`, `BPMNINJA_WORKER_API_KEY`, 401, fail-closed |
| Health | Liveness, Readiness, `/api/health`, `/api/ready` |
| Abweichung | Deviation, nicht unterstützt, Multi-Instance, Flattening |

*Nutzeraufgabe:* „Ich suche nach dem Wort, das ich kenne, nicht nach dem, das die Doku benutzt."

---

## 5. Qualitätskriterien und Verantwortlichkeiten

### 5.1 Rollen

| Rolle | Verantwortung | Nicht verantwortlich für |
|---|---|---|
| **Documentation Lead** (Redaktionsleitung) | IA, Content-Plan, Freigabe, Konfliktentscheidung, Eskalation | Fließtext-Kapitel schreiben |
| **Technical Writer** | Tutorials, How-tos, Runbooks schreiben; Front-Matter pflegen | Technische Wahrheit feststellen |
| **Reference Owner** (je Modul) | Reference-Seiten aktuell halten; Single Source of Truth sichern | Erklärtexte |
| **Engineering Liaison** | Beantwortet „Codeabgleich offen"-Fragen; bestätigt Grenzwerte, Namen, Einheiten | Formulierung |
| **Reviewer (Peer)** | Prüft Verständlichkeit, Diátaxis-Zuordnung, Links | Technische Korrektheit |
| **Accessibility Reviewer** | Alt-Texte, Diagramm-Beschreibungen, Tabellenbreite | Inhalt |

### 5.2 Qualitätskriterien pro Content-Typ

**Tutorial (`content_type: tutorial`)**
- [ ] Ein einziges Szenario, keine Alternativen, keine Referenztabellen.
- [ ] Jeder Schritt hat ein **erwartetes Ergebnis** (Ausgabe, Statuscode, sichtbarer Zustand).
- [ ] Abschnitt „Wenn etwas schiefgeht" mit mindestens drei konkreten Fehlern.
- [ ] Endet mit „Nächster Schritt" (Verweis auf How-to oder Reference).
- [ ] In ≤ 15 Minuten von einer Person ohne Vorwissen durchführbar (Nachweis: ein Probelauf mit einer Person, die BPMNinja nicht kennt).

**How-to Guide (`content_type: how-to`)**
- [ ] Titel beginnt mit einem Verb oder benennt die Aufgabe („Worker schreiben", „Deployment absichern").
- [ ] Voraussetzungen vollständig; jeder genutzte Befehl ist kopierbar.
- [ ] Kein Lehrtext („BPMN ist eine Notation…"); Verweis auf Explanation.
- [ ] Jede technische Aussage ist belegt: entweder durch Codeabgleich oder durch Verweis auf eine freigegebene Reference-Seite.

**Reference (`content_type: reference`)**
- [ ] Tabellenform, keine Prosa.
- [ ] Jede Zeile hat eine **einzige** autoritative Quelle; andere Seiten verweisen hierher.
- [ ] Keine Zahlen in Prosa (A-06): Endpunktzahl, Versionsnummern und Grenzwerte stehen in der Tabelle, nicht im Satz.
- [ ] Statuscodes, Defaults und Grenzwerte sind als solche markiert.

**Explanation (`content_type: explanation`)**
- [ ] Beantwortet „warum", nicht „wie".
- [ ] Nennt Konsequenzen für den Nutzer („Das bedeutet für dein Modell: …").
- [ ] Verweist auf die Reference, die die Aussage belegt.

**Runbook (`content_type: runbook`)**
- [ ] Beginnt mit dem **Alarmsignal** (Metrik, Statuscode, Logzeile).
- [ ] Enthält Entscheidungsbaum: „Wenn X → tue Y, sonst Z".
- [ ] Enthält Eskalationsweg und „Was **nicht** tun".
- [ ] Wurde mindestens einmal durchgespielt (Datum im Front-Matter).

### 5.3 Redaktionsregeln (verbindlich)

1. **Sprache:** Deutsch. Fachbegriffe englisch, wenn sie im Code so heißen; beim ersten Vorkommen erklärt.
2. **Eine Tatsache, eine Stelle.** Wird eine Tatsache an zwei Stellen gebraucht, verweist die zweite auf die erste.
3. **Keine unbelegten Aussagen.** Jede technische Aussage trägt entweder einen Codeabgleich-Vermerk (intern) oder einen Verweis auf eine freigegebene Seite. „Codeabgleich offen" blockiert die Freigabe.
4. **Keine Zahlen in Prosa.** Endpunktzahlen, Versionen, Grenzwerte nur in Tabellen.
5. **Barrierefreiheit:** Jedes Mermaid-Diagramm hat eine Textbeschreibung (A-20). Tabellen haben ≤ 5 Spalten oder eine Begleitliste. Alt-Texte sind Pflicht.
6. **Links:** Nur auf existierende oder im Content-Plan stehende Dateien. Tote Links blockieren die Freigabe (A-18).
7. **Front-Matter:** Vollständig, `status` korrekt, `last_reviewed` gesetzt.

### 5.4 Freigabeprozess

| Stufe | Wer | Ergebnis |
|---|---|---|
| 1. Entwurf | Technical Writer | `status: entwurf` |
| 2. Technische Prüfung | Engineering Liaison | Alle „Codeabgleich offen" geschlossen oder Seite bleibt `entwurf` |
| 3. Peer-Review | Reviewer | Verständlichkeit, Diátaxis, Links |
| 4. Accessibility | Accessibility Reviewer | Alt-Texte, Diagramme, Tabellen |
| 5. Freigabe | Documentation Lead | `status: freigegeben` — **nur** wenn §5.2-Checkliste vollständig |

**Eskalationsregel:** Kann ein „Codeabgleich offen"-Befund nicht innerhalb von 5 Arbeitstagen geschlossen werden, entscheidet der Documentation Lead sichtbar: (a) Seite bleibt `entwurf` und wird nicht verlinkt, (b) Seite wird mit dokumentiertem Risiko als `review` veröffentlicht und trägt einen Warnhinweis, (c) Seite wird zurückgestellt. Die Entscheidung wird im Content-Plan vermerkt.

### 5.5 Pflege und Review-Zyklus

- **Review-Intervall:** 180 Tage ab `last_reviewed`. Überfällige Seiten wechseln automatisch auf `veraltet`.
- **Trigger-Review:** Jede Änderung an `Cargo.toml`, `docs/openapi.yaml`, `engine-server/src/server/tasks.rs` oder `persistence-nats/src/trait_impl.rs` löst eine Prüfung der betroffenen Reference-Seiten aus.
- **Widerspruchs-Register:** Der Documentation Lead führt eine Liste aller offenen Widersprüche (A-05 bis A-09) mit Status und Verantwortlichem. Ein Widerspruch gilt erst als geschlossen, wenn **eine** autoritative Stelle existiert und alle anderen verweisen.

### 5.6 Messgrößen (Fortschritt gegenüber Baseline 54/100)

| Kennzahl | Baseline | Ziel für Freigabe |
|---|---|---|
| Diátaxis-Abdeckung (Quadranten mit ≥ 1 freigegebener Seite) | 2 von 4 | 4 von 4 |
| Offene Blocker (A-01 bis A-04) | 4 | 0 |
| Offene „hoch"-Befunde in Vollständigkeit/Korrektheit/Aktualität | 5 | 0 |
| Tote Links | unbekannt | 0 |
| Seiten mit vollständigem Front-Matter | 0 | 100 % |
| Diagramme ohne Textbeschreibung | alle | 0 |
| Gesamtscore | 54 | ≥ 80 |
```

---

## Datei 2 — `_architecture/content-plan.md`

```markdown
---
title: Content-Plan der BPMNinja-Nutzerdokumentation
description: Konkreter Dateiplan mit Pfaden, Titeln, Zweck, Zielgruppe, Abhängigkeiten und Priorität für alle neuen und übernommenen Seiten.
audience: [dokumentationsteam, engineering]
prerequisites: [_architecture/information-architecture.md]
content_type: explanation
status: entwurf
last_reviewed: 2026-07-17
owner: documentation-lead
related: [_architecture/information-architecture.md, _architecture/migration-map.md]
---

# Content-Plan

**Legende Priorität:** P0 = Blocker (Freigabe unmöglich ohne), P1 = hoch (Freigabe-Qualität), P2 = mittel (Vollständigkeit), P3 = später.
**Legende Belegstatus:** ✅ belegt (Quelle im Bestand) · ⚠️ Codeabgleich offen · ⛔ gesperrt (Widerspruch ungelöst)

---

## 1. Tutorials (`docs/tutorials/`)

### T-01 · `docs/tutorials/erster-prozess.md`
- **Titel:** Erster Prozess in 15 Minuten
- **Zweck:** Ein Nutzer ohne Vorwissen deployt ein BPMN-Modell (StartEvent → UserTask → EndEvent), startet eine Instanz, schließt den User Task ab und sieht `completed`.
- **Zielgruppe:** Einsteiger und Lernende
- **Voraussetzungen:** Docker; ein Terminal; keine BPMNinja-Kenntnisse
- **Inhalt (verbindlich):** (1) NATS mit JetStream starten (`--js`, User/Pass), (2) Engine mit `BPMNINJA_API_KEY` und `BIND_ADDR=127.0.0.1` starten, (3) `/api/health` prüfen, (4) BPMN-XML deployen, (5) Instanz starten, (6) offene User Tasks abfragen, (7) Task abschließen, (8) Status prüfen. Jeder Schritt mit erwarteter Ausgabe. Abschnitt „Wenn etwas schiefgeht": NATS nicht erreichbar, 401, Port 8081 belegt.
- **Abhängigkeiten:** R-02 (Umgebungsvariablen), R-01 (REST-API) — beide nur als Verweis am Ende, nicht im Ablauf.
- **Belegstatus:** ✅ (Werte
