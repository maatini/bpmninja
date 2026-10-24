---
title: QA-Bericht — BPMNinja-Dokumentation, Iteration 0
description: Ganzheitliche Qualitätsprüfung des Dokumentationsbestands gegen die sechs QA-Dimensionen, mit belegten Blockern, Nicht-Blockern, Prüflücken und einer Freigabeempfehlung.
audience:
  - Beitragende und Maintainer
  - Stakeholder und Entscheider
prerequisites:
  - Kenntnis der Informationsarchitektur unter _architecture/information-architecture.md
  - Kenntnis des Style Guides unter _review/style-guide.md
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _qa/content-manifest.md
  - _review/technical-review.md
  - _review/technical-review-round-1.md
  - _review/style-report.md
  - _review/ux-review.md
  - _architecture/diagram-plan.md
  - _architecture/information-architecture.md
  - _qa/revision-round-1.md
---

# QA-Bericht — BPMNinja-Dokumentation, Iteration 0

**Prüfdatum:** 2026-10-24
**Prüfumfang:** 24 Markdown-Dateien im Ausgabeverzeichnis — 11 veröffentlichte Seiten, 2 Landingpages, 11 Steuerungsdateien
**Schwelle:** 90 / 100
**Iteration:** 0
**Ergebnis:** **nicht freigegeben** (`changes_requested`) — 7 offene Blocker

---

## 1. Management Summary

Der Bestand ist in **Struktur, Ton und Diátaxis-Schnitt** deutlich weiter als der Ausgangszustand: Es gibt einen Einstieg mit vier Lesepfaden, ein Tutorial mit Abnahmekriterien, zwei How-tos mit Fehlerabschnitten, eine kanonische Lock-Dauer-Definition, ein Glossar mit Verwechslungstabelle und einen Style Guide mit 47 Regeln. Die Landingpage, die Informationsarchitektur und die Explanation-Seiten sind inhaltlich belegt und für Stakeholder lesbar. Die Nachbesserungsrunde 1 hat zwei Blocker vollständig geschlossen (B-04 REST-API-Fehlerformat, B-07 Worker-Beispiel) und einen dritten teilweise (B-03).

**Freigabefähig ist der Bestand dennoch nicht.** Vier Ursachen:

1. **Zwei P0-Referenzseiten fehlen vollständig** (`reference/bpmn-elemente.md`, `reference/kompatibilitaet.md`). Sie sind die Seiten, auf denen die teuersten Nutzerfehler entschieden werden — Element-Abbildung und Camunda-Abweichungen. Beide werden im Text als „geplant (P0)" genannt, aber die Lücke ist inhaltlich nicht geschlossen.
2. **Eine als kanonisch referenzierte Seite ist ein Fragment.** `reference/konfiguration.md` besteht aus vier Wörtern (`title`), ohne Body und ohne sieben der acht Pflicht-Front-Matter-Felder. Sechs Seiten verweisen für Default-Werte auf sie oder müssen den Verweis umgehen. Damit hat der Bestand **keine** Referenz für Umgebungsvariablen — die wichtigste Nachschlageaufgabe von Betriebsteams.
3. **Ein Beispiel ist unbelegt und ein Anker ist defekt.** Die `errorRef`-Semantik in `how-to/external-task-worker-schreiben.md` Schritt 1 hat **null Treffer in 49 Quelltexten** (B-08). In `reference/service-task-api.md` §5 steht ein defekter Anker mit doppeltem Schrägstrich (B-03).
4. **Ein neuer Widerspruch zwischen zwei Explanation-Seiten** ist bei der Nachprüfung aufgefallen: `explanation/architektur.md` §5.1 nennt KV-Buckets mit `bpm_*`-Präfix, `explanation/konzepte.md` nennt Kurznamen. Beide Varianten sind in unterschiedlichen Quellen belegt (B-10). Ein Leser, der Buckets manuell prüft, greift mit einer der beiden Varianten daneben.

Die technische Tiefe ist hoch, die Belegdichte in den Explanation-Seiten ist gut, und die meisten UX-Befunde sind behoben. Der Bestand ist damit **nahe an der Freigabe, aber nicht freigabefähig**: Es fehlen Vollständigkeit an zwei P0-Stellen, ein konsistenter Bucket-Katalog und der Nachweis, dass die Beispiele laufen.

**Gewichteter Score: 75 / 100** (berechnet vom QA-Werkzeug mit den Gewichten Vollständigkeit 20 %, Korrektheit 25 %, Klarheit 15 %, Konsistenz 15 %, Diátaxis 15 %, Nutzbarkeit 10 %). Der Abstand zur Schwelle ist nicht durch Politur zu schließen, sondern durch vier inhaltliche Lieferungen: Konfigurationsreferenz, die zwei P0-Seiten, die Reparatur der verbliebenen Defekte und einen Verifikationslauf der Beispiele.

---

## 2. Dimensionsscores

| Dimension | Score | Gewicht | Kurzbegründung |
|---|---:|---:|---|
| **Vollständigkeit** | 62 | 20 % | Zwei P0-Seiten fehlen, eine Referenzseite ist ein 4-Wort-Fragment, eine Seite ist abgebrochen. Voraussetzungen, Ergebnisse und Fehlerfälle sind auf den vorhandenen Seiten dagegen durchgängig vorhanden. Betriebs-Runbook (NATS-Ausfall) fehlt. |
| **Technische Korrektheit** | 70 | 25 % | Belegdichte hoch, Aussagen gegen Knowledge Base und ADRs nachvollziehbar. Aber: kein Beispiel gegen Live-Server oder `docs/openapi.yaml` verifiziert (B-06), unbelegte `errorRef`-Semantik (B-08), widersprüchlicher KV-Bucket-Katalog (B-10), mehrere ungenaue Semantik-Aussagen (W-01 bis W-06). |
| **Klarheit** | 86 | 15 % | Aufgabe, Zielgruppe und nächster Schritt sind auf jeder Seite erkennbar; „Woran du erkennst, dass du fertig bist" und „Hilfe bei Problemen" sind vorhanden. Abzug für Sätze über 35 Wörter in `explanation/architektur.md` und für den fehlenden Einstieg für Prozessmodellierer. |
| **Konsistenz** | 78 | 15 % | Front Matter ist bis auf B-01 vollständig und gleich aufgebaut; Terminologie folgt dem Glossar. Abzug für Sie/du-Bruch (`README.md` siezt gegen den Rest), `content_type: reference` für zwei Landingpages, „Workflow-Engine" auf der Landingpage, Anker mit Umlauten, ✅/❌ in der Rollenmatrix. |
| **Diátaxis** | 84 | 15 % | Die vier Bereiche sind sauber getrennt, jede Seite hat genau einen Zweck. Abzug für Referenztabellen im How-to (Konfigurationswerte), vollständige Fehlerbehandlung im Tutorial (gehört ins How-to) und zwei Landingpages ohne eigenen Typ. |
| **Nutzbarkeit** | 80 | 10 % | Vier Lesepfade, Glossar von allen Seiten verlinkt, Symptomtabelle mit neun Einträgen, durchgängige „Nächster sinnvoller Schritt"-Blöcke. Abzug für fehlenden Einstieg für Prozessmodellierer (UX-10), leere Konfigurationsreferenz (UX-11), fehlende Bereichs-`index.md` (UX-13) und ungeprüfte Anker. |

**Gewichteter Gesamtscore: 75 / 100** — Schwelle 90 nicht erreicht.

### 2.1 Nicht bewertbare Bereiche (dokumentierte Prüflücke)

Jede Prüflücke ist score-mindernd in der betroffenen Dimension berücksichtigt. Sie werden hier offengelegt, damit die Freigabeempfehlung nachvollziehbar bleibt.

- **Live-Verifikation der Beispiele** — kein laufender Server, kein `docs/openapi.yaml` im Prüfzugriff. Betrifft *Technische Korrektheit*. Folge: B-06 bleibt offen, auch wenn einzelne Beispiele gegen die Knowledge Base plausibel sind.
- **Rendering der Mermaid-Diagramme** — `verify_mermaid` meldet keine Syntaxbefunde (5 Diagramme, keine leeren Blöcke, keine Tabulatoren), ersetzt aber kein Rendering im Zielrenderer. Betrifft *Nutzbarkeit* und *Barrierefreiheit*. Folge: B-09 bleibt offen.
- **Ankerauflösung** — `check_links` prüft Anchors nicht. Betrifft *Nutzbarkeit*. Folge: Umlaut-Anker im Glossar-Sprungtableau und der defekte Anker in B-03 sind nur manuell auffindbar.
- **Nutzerverhalten** — keine Nutzertests durchgeführt. Aussagen über Verständlichkeit für Einsteiger stützen sich auf Struktur und Stilbefunde, nicht auf Beobachtung. Betrifft *Klarheit* und *Nutzbarkeit*.
- **Vollständigkeit der Konfigurationsreferenz** — nicht prüfbar, weil die Seite fehlt (B-01). Die Default-Werte wurden stattdessen gegen `engine-server/interfaces.md` und `docs/architecture.md` gegengeprüft; das ersetzt keine Referenzseite.
- **OpenAPI-Diff** — Abgleich der dokumentierten Endpunkte gegen `docs/openapi.yaml` nicht möglich. Die Endpunktzahl ist zwischen Audit (46) und Route-Map (48) bereits widersprüchlich (AUD-006) und bleibt ungeklärt.

---

## 3. Blocker

Ein Blocker verhindert die Freigabe. Jeder Blocker nennt Datei, Stelle und Belegstatus.

### B-01 — `reference/konfiguration.md` ist ein 4-Wort-Fragment (offen)

- **Datei/Stelle:** `reference/konfiguration.md`, gesamte Datei.
- **Befund:** Die Datei enthält ausschließlich `---` und `title: Konfiguration und Umgebungs`. Es fehlen `description`, `audience`, `prerequisites`, `content_type`, `status`, `last_reviewed`, `owner`, `related` sowie der komplette Body. `doc_list` weist 4 Wörter und Status `–` aus.
- **Wirkung:** Sechs Seiten verweisen für Default-Werte auf diese Seite. Betriebsteams finden keine Umgebungsvariablen-Referenz — die häufigste Nachschlageaufgabe fällt aus. `how-to/konfiguration-anpassen.md` muss die Werte deshalb selbst als Tabelle führen, was zugleich ein Diátaxis-Problem erzeugt.
- **Beleglage:** Die Werte selbst sind belegt (`PORT=8081`, `BIND_ADDR=0.0.0.0`, `REQUIRE_NATS=true`, `MAX_XML_BYTES=10485760`, `MAX_UPLOAD_BYTES=5242880`, `PERSISTENCE_RETRY_QUEUE_CAPACITY=10000`, `BPMNINJA_EVENT_RETENTION_DAYS=7`, `BPMNINJA_HISTORY_RETENTION_DAYS=30`, `BPMNINJA_STREAM_MAX_BYTES=1073741824`, `BPMNINJA_STREAM_REPLICAS=1`, `LOG_FILE=engine_logs.jsonl`, `RUST_LOG=info`, `LOG_FORMAT=text`). Es fehlt die Seite, nicht der Inhalt.
- **Abhilfe:** Seite vollständig schreiben: alle Variablen mit Typ, Default, Wirkung, Pflicht/optional, Geheimnis-Kennzeichnung; Front Matter nach Standard F1.

### B-03 — `reference/service-task-api.md` §5: defekter Anker und unvollständige Statusliste (teilweise behoben)

- **Datei/Stelle:** `reference/service-task-api.md`, §5 (Fehlerbehandlung), Anker `..//explanation/konzepte.md` sowie die Status-Tabelle zum `bpmnError`-Pfad.
- **Befund:** Der `bpmnError`-Pfad ist inhaltlich nachgebessert (Runde 1). Verblieben sind zwei Defekte: (a) der Anker enthält einen doppelten Schrägstrich (`..//`) und löst nicht auf; (b) die Status-Tabelle zum `bpmnError`-Pfad führt keinen `409`-Fall, obwohl `ServiceTaskNotLocked` in `engine-server/interfaces.md:40` auf `409` abgebildet wird.
- **Wirkung:** Der Leser landet bei einem Klick auf einen nicht existierenden Pfad; die Statusliste ist unvollständig und widerspricht der REST-API-Referenz, die `409` korrekt als Zustandskonflikt führt.
- **Abhilfe:** Anker auf `../explanation/konzepte.md` korrigieren; `409` in die Tabelle aufnehmen und mit `ServiceTaskNotLocked` belegen.

### B-05 — Zwei P0-Referenzseiten fehlen (offen)

- **Datei/Stelle:** `reference/bpmn-elemente.md` und `reference/kompatibilitaet.md` — beide Dateien existieren nicht.
- **Befund:** `explanation/architektur.md` §2.3 und §7 verweisen auf die Element-Abbildung; `explanation/konzepte.md` führt die Kompatibilitätsseite im Abschnitt „Offen". Die IA (Entscheidung E6: eine einzige Kompatibilitätsseite) ist beschlossen, die Seiten sind nicht geschrieben.
- **Wirkung:** Genau die zwei Fragen, die Nutzer am teuersten falsch beantworten — „Wird mein Element unterstützt?" und „Verhält sich das wie in Camunda?" — haben keine Referenz. Nutzer müssen die Knowledge Base lesen, die laut E3 ausdrücklich Beitragenden-Referenz ist, nicht Nutzerdokumentation.
- **Abhilfe:** Beide Seiten schreiben, mit Beleg aus der Knowledge Base; Element-Abbildung als Tabelle (Element → Unterstützung → Einschränkung → Belegstelle), Kompatibilitätsseite als Abweichungsliste mit Richtung (gleich / abweichend / nicht unterstützt).

### B-06 — Kein Beispiel gegen laufende Engine oder `docs/openapi.yaml` verifiziert (offen)

- **Datei/Stelle:** `tutorials/erster-prozess.md`, `how-to/external-task-worker-schreiben.md`, `how-to/konfiguration-anpassen.md`, `reference/rest-api.md`, `reference/service-task-api.md`.
- **Befund:** Kein Code-Beispiel trägt einen Verifikationsnachweis. Die Beispiele sind gegen die Knowledge Base plausibilisiert, aber nicht ausgeführt. `docs/openapi.yaml` war im Prüfzugriff nicht verfügbar, ein laufender Server ebenfalls nicht.
- **Wirkung:** Die zentrale Nutzererwartung an ein Tutorial — „kopieren, ausführen, es funktioniert" — ist unbelegt. B-07 hat gezeigt, dass die Plausibilisierung nicht ausreicht: Dort standen drei falsche Client-API-Aufrufe in einem Beispiel, das gegen die Knowledge Base „richtig aussah".
- **Abhilfe:** Verifikationslauf gegen laufende Engine dokumentieren (Datum, Version `0.7.24` / Client `1.0.0`, Ergebnis pro Beispiel) und Endpunkt-/Payload-Abgleich gegen `docs/openapi.yaml`; Ergebnis als Verifikationsvermerk an den Beispielen.

### B-08 — `errorRef`-Semantik unbelegt (offen)

- **Datei/Stelle:** `how-to/external-task-worker-schreiben.md`, Schritt 1.
- **Befund:** Die Aussage zur `errorRef`-Semantik hat **null Treffer in 49 Quelltexten** (`source_search` über den gesamten Bestand). Es gibt weder eine Knowledge-Base-Stelle noch eine ADR, die die beschriebene Auflösungsreihenfolge oder das Fehlerverhalten stützt.
- **Wirkung:** Eine unbelegte technische Aussage in einem How-to ist ein Korrektheitsrisiko erster Ordnung: Der Leser baut auf ein Verhalten, das niemand geprüft hat.
- **Abhilfe:** Entweder mit einer konkreten Quelle belegen (Knowledge Base, Code, Test) oder die Aussage auf das belegbar Gesicherte reduzieren und den Rest als offene Frage kennzeichnen. Keine dritte Möglichkeit.

### B-09 — Mermaid-Diagramme ohne Rendering-Nachweis (offen)

- **Datei/Stelle:** `_architecture/diagram-plan.md` D1–D5, eingebettet in `explanation/architektur.md` §1, §2.2, §4.4, `reference/service-task-api.md` §5, `explanation/konzepte.md` §6.
- **Befund:** `verify_mermaid` meldet für alle fünf Diagramme keine Syntaxbefunde (keine leeren Blöcke, keine Tabulatoren, bekannte Diagrammtypen: `flowchart LR`, `stateDiagram-v2`, `sequenceDiagram` ×2, `flowchart TD`). Ein Rendering im Zielrenderer ist damit nicht nachgewiesen.
- **Wirkung:** Ein nicht renderndes Diagramm ist für den Leser ein leerer Kasten an einer Stelle, an der die Erklärung sonst nur im Diagramm steht (insbesondere D2 Zustandsmodell und D4 Startreihenfolge).
- **Abhilfe:** Rendering im Zielrenderer einmal durchführen und Ergebnis dokumentieren; zusätzlich je Diagramm einen Satz Alternativtext prüfen.

### B-10 — Widersprüchlicher KV-Bucket-Katalog zwischen zwei Explanation-Seiten (offen, neu)

- **Datei/Stelle:** `explanation/architektur.md` §5.1 gegen `explanation/konzepte.md` (Persistenz-Abschnitt).
- **Befund:** `explanation/architektur.md` §5.1 nennt Buckets mit `bpm_*`-Präfix (belegt durch `docs/architecture.md:418`, dort `bpm_definitions`). `explanation/konzepte.md` nennt Kurznamen ohne Präfix (belegt durch `persistence/gotchas.md:74`: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history`). Zusätzlich fehlt in beiden Aufzählungen der zehnte Bucket `history_completed_instances` (belegt: `persistence/gotchas.md:88`).
- **Wirkung:** Wer Buckets manuell prüft oder aufräumt, greift mit einer der beiden Varianten daneben. Der Widerspruch steht zwischen zwei Seiten, die dieselbe Zielgruppe haben, und ist damit für den Leser nicht als „eine davon ist alt" erkennbar.
- **Abhilfe:** Eine Variante als kanonisch festlegen (Empfehlung: die in `persistence/gotchas.md` belegten Kurznamen, da dort die Änderungswarnung steht), die andere Seite anpassen, `history_completed_instances` in beiden Aufzählungen ergänzen und die Abweichung in `_review/technical-review.md` als AUD-022 nachziehen.

---

## 4. Nicht-Blocker

### 4.1 Behobene Blocker aus Runde 1

| ID | Stelle | Status | Beleg |
|---|---|---|---|
| **B-04** | `reference/rest-api.md` | **behoben** | Abschnitt `## Offen` vollständig; Fehlertabelle mit 9 Statuscodes; Pagination dokumentiert. |
| **B-07** | `how-to/external-task-worker-schreiben.md` Schritt 4 | **behoben** | Handler-Signatur `(task, service)`, `task.variables_snapshot`, `complete({ gesendet: true })`, `failure(message, details, retries)`, `bpmnError(errorCode)` — gegen `external-task-client/interfaces.md` (Zeilen 23, 24, 36, 40, 50, 51, 54) verifiziert. |

### 4.2 Warnungen (W-01 bis W-06)

Präzisionsmängel, die die Freigabe nicht blockieren, aber vor Veröffentlichung mitgenommen werden sollten:

- **W-01** — Lock-Dauer-Einheit: `lockDuration`/`newDuration` sind in **Millisekunden** mit Bereich `1_000..=3_600_000`; intern `(ms + 999) / 1000`, geklemmt auf `1..=3600` s. Kanonisch in `reference/service-task-api.md` (IA-Entscheidung E7). Andere Seiten müssen darauf verweisen statt zu wiederholen.
- **W-02** — `fetchAndLock`-Validierung: `topics` 1..=32, `topicName` nicht leer, `lockDuration` 1 000..=3 600 000 ms, `maxTasks` 1..=100, sonst `400`. In `reference/rest-api.md` vollständig, in `reference/service-task-api.md` nicht gespiegelt.
- **W-03** — Authentifizierung: `BPMNINJA_API_KEY` (Admin), `BPMNINJA_WORKER_API_KEY` (Worker); ohne Schlüssel auf Nicht-Loopback-Bind fail-closed → `401`; Worker auf Admin-Route → `403`; `BPMNINJA_ALLOW_UNAUTHENTICATED=true` nur für Entwicklung. Die Dev-Ausnahme ist nicht auf jeder Seite als „nur Entwicklung" markiert.
- **W-04** — SSE: `GET /api/events`, Events `instance_changed`, `task_changed`, `definition_changed`; Broadcast-Kanal-Kapazität 256, **kein Catch-up**. Der Catch-up-Verlust ist in `reference/rest-api.md` erwähnt, aber nicht im Glossar erklärt (Begriff „Catch-up" fehlt).
- **W-05** — Grenzwerte: `MAX_INCIDENTS_PER_INSTANCE=100`; Rhai 50 000 Operationen / 2 MiB / 1 000 ms; Log-Puffer 5 000 (Speicher) bzw. 50 000 (NATS-Stream `ENGINE_LOGS`). Die drei Zahlen stehen an verschiedenen Stellen, ohne gemeinsame Referenz.
- **W-06** — Client-Defaults: `maxRetries` 3 (global pro Client), `lockDuration` 30 000 ms, `autoExtendLock` false (Verlängerung bei 80 %). `autoExtendLock` ist im How-to nicht erwähnt, obwohl es das Verhalten bei langen Tasks bestimmt.

### 4.3 Stil- und Konsistenzbefunde

- **R2.6 Sie/du-Bruch:** `README.md` siezt, alle anderen Seiten duzen. Style Guide verlangt „du".
- **R1.6 Sperrliste:** „Workflow-Engine" auf der Landingpage (`README.md`).
- **`content_type: reference`** für zwei Landingpages (`README.md`, `reference/README.md`) — Landingpages haben keinen eigenen Typ im Front-Matter-Standard F1.
- **Umlaut-Anker** im Glossar-Sprungtableau (R6.7) — Auflösung ungeprüft, siehe Prüflücke „Ankerauflösung".
- **✅/❌ in der Rollenmatrix** (`reference/rest-api.md`) — Symbole statt Text, für Screenreader problematisch.
- **Sätze über 35 Wörter** in `explanation/architektur.md`.
- **Fehlende Glossareinträge:** „Long-Polling", „Catch-up", „fail-closed", „Backoff", „Boundary Error Event", „Call Activity", „JoinBarrier", „OrphanedToken" — alle acht Begriffe werden im Bestand verwendet, ohne im Glossar erklärt zu sein.

### 4.4 Diátaxis-Vermischungen

- **Referenztabellen im How-to:** `how-to/konfiguration-anpassen.md` führt Konfigurationswerte als Tabelle — Folge von B-01, löst sich mit der Konfigurationsreferenz auf.
- **Vollständige Fehlerbehandlung im Tutorial:** `tutorials/erster-prozess.md` enthält die Fehlerbehandlung in voller Tiefe; das gehört ins How-to, das Tutorial braucht nur den glücklichen Pfad plus Verweis.
- **Zwei Landingpages ohne eigenen Typ:** siehe 4.3.

### 4.5 UX-Befunde

- **UX-10** — Kein Einstieg für Prozessmodellierer: Die vier Lesepfade decken Einsteiger, Entwickler, Betrieb und Stakeholder ab; wer nur ein Modell hochladen will, findet keinen eigenen Pfad.
- **UX-11** — Leere Konfigurationsreferenz (Folge von B-01).
- **UX-13** — Fehlende `index.md` in den Bereichen; Navigation hängt an der Landingpage.

### 4.6 Steuerungsdateien mit Nachführungsbedarf

- **`_qa/content-manifest.md` ist veraltet:** Es nennt 14 Dateien / 27 218 Wörter, tatsächlich sind es 24 Dateien. Einzelabweichungen: `explanation/konzepte.md` 642 → 2 007 Wörter (dort als gekürzt geführt, tatsächlich vollständig), `README.md` 972 → 2 379, `reference/README.md` 1 054 → 1 339, `how-to/konfiguration-anpassen.md` 2 492 → 2 433, `explanation/architektur.md` 3 433 → 2 824.
- **`_audit/audit-report.md`** enthält zwei Pfadverweise außerhalb des Ausgabewurzel (Zeile 83) — Ursache von 2 der 4 Linkbefunde.
- **`_review/style-guide.md`** Zeilen 346/347 verweisen auf `ablauf.svg` als Beispieltext, nicht als Navigation — kein echter Linkfehler, aber im Linklauf auffällig.

---

## 5. Geprüfte Bereiche

| Prüfgegenstand | Werkzeug / Methode | Ergebnis |
|---|---|---|
| Dateibestand und Wortzahlen | `doc_list` | 24 Dateien, 11 veröffentlichte Seiten, 2 Landingpages, 11 Steuerungsdateien |
| Quellenlage | `source_inventory` | 49 Quelltexte (Knowledge Base, `bpmninja/README.md`, `docs/architecture.md`) |
| Interne Links und Bildpfade | `check_links` | 171 Links, **4 defekt** — keiner davon auf einer veröffentlichten Seite |
| Mermaid-Syntax | `verify_mermaid` | 5 Diagramme, keine Syntaxbefunde |
| Belegprüfung `errorRef` | `source_search` | 0 Treffer in 49 Dateien → B-08 |
| Belegprüfung KV-Buckets | `source_search` | `docs/architecture.md:418` (`bpm_definitions`) vs. `persistence/gotchas.md:74` (Kurznamen) → B-10 |
| Belegprüfung `history_completed_instances` | `source_search` | 1 Treffer: `persistence/gotchas.md:88` |
| Belegprüfung `ServiceTaskNotLocked` | `source_search` | 1 Treffer: `engine-server/interfaces.md:40` → `409` |
| Belegprüfung `GET /api/info` | `source_search` | 1 Treffer: `docs/architecture.md:496` |
| Belegprüfung `BPMNINJA_ALLOW_UNAUTHENTICATED` | `source_search` | 5 Treffer, Dev-only-Aussage gedeckt |
| Client-API-Signaturen | `source_read` `external-task-client/interfaces.md` | Handler, `variables_snapshot`, `complete`, `failure`, `bpmnError`, `extendLock` bestätigt → B-07 behoben |
| Fehler- und Statusmodell | `source_read` `engine-server/interfaces.md` | `AppError`-Abbildung, Route-Map (48 Endpunkte), `ServiceTaskNotLocked` → `409` |
| Persistenz-Gotchas | `source_read` `persistence/gotchas.md` | Bucket-Namen, Änderungswarnung, Archiv-Bucket |
| Alle 11 veröffentlichten Seiten | `doc_read` | vollständig gelesen und gegen Quellen geprüft |
| Steuerungsdateien | `doc_read` | IA, Diagrammplan, Content-Manifest, Technical Review (2 Runden), UX-Review, Style Report, Style Guide, Glossar, Revisionsbericht |

**Positiv belegt:** Einstieg mit vier Lesepfaden; Tutorial mit Abnahmekriterien; zwei How-tos mit Fehlerabschnitten; kanonische Lock-Dauer-Definition (E7); Glossar mit Verwechslungstabelle und 12 gesperrten Begriffen; Style Guide mit 47 Regeln in 9 Abschnitten, jeder mit Positiv- und Negativbeispiel; Symptomtabelle mit 9 Einträgen; durchgängige „Nächster sinnvoller Schritt"-Blöcke; Front Matter nach Standard F1 auf allen Seiten außer B-01.

---

## 6. Nicht prüfbare Bereiche

Diese Bereiche konnten nicht geprüft werden. Sie sind nicht „bestanden", sondern **offen** und in den Dimensionsscores als Abzug enthalten.

1. **Ausführung der Beispiele** — kein laufender Server, kein Zugriff auf `docs/openapi.yaml`. Betroffen: alle Code-Beispiele in Tutorial, beiden How-tos und beiden API-Referenzen.
2. **Rendering der Diagramme** — `verify_mermaid` prüft Syntax, nicht Darstellung. Betroffen: D1–D5.
3. **Ankerauflösung** — `check_links` prüft Anchors nicht. Betroffen: Glossar-Sprungtableau, Querverweise zwischen Explanation-Seiten, der defekte Anker aus B-03.
4. **Nutzerverhalten** — keine Nutzertests. Aussagen zur Verständlichkeit für Einsteiger sind strukturell begründet, nicht beobachtet.
5. **Vollständigkeit der Konfigurationsreferenz** — nicht prüfbar, weil die Seite fehlt (B-01).
6. **OpenAPI-Abgleich** — Endpunktzahl widersprüchlich (Audit 46, Route-Map 48, AUD-006); ohne Spezifikation nicht auflösbar.
7. **Barrierefreiheit im Zielrenderer** — Kontrast, Fokusführung und Diagramm-Alternativtexte sind auf Markdown-Ebene nicht abschließend beurteilbar.

---

## 7. Freigabeempfehlung

**Empfehlung: `changes_requested` — nicht freigeben.**

Der Bestand erreicht **75 / 100** bei einer Schwelle von 90 und trägt **7 offene Blocker** (B-01, B-03, B-05, B-06, B-08, B-09, B-10). Zwei Blocker aus Runde 1 sind belegt geschlossen (B-04, B-07); B-03 ist teilweise geschlossen.

**Freigabekriterien für Iteration 1** — alle Punkte müssen belegt sein, nicht zugesagt:

| Kriterium | Nachweis |
|---|---|
| `reference/konfiguration.md` vollständig | Alle Variablen mit Typ, Default, Wirkung, Pflicht/optional; Front Matter F1 vollständig |
| `reference/bpmn-elemente.md` und `reference/kompatibilitaet.md` vorhanden | Element-Abbildung als Tabelle mit Belegstelle je Zeile; Abweichungsliste mit Richtung |
| B-03 geschlossen | Anker korrigiert, `409` in der `bpmnError`-Statusliste mit Beleg |
| B-08 geschlossen | `errorRef`-Aussage belegt oder auf das Belegte reduziert |
| B-10 geschlossen | Ein Bucket-Katalog kanonisch, zweite Seite angeglichen, `history_completed_instances` ergänzt |
| B-06 geschlossen | Verifikationslauf dokumentiert (Datum, Versionen, Ergebnis je Beispiel) plus OpenAPI-Abgleich |
| B-09 geschlossen | Rendering im Zielrenderer dokumentiert |
| Nicht-Blocker aus 4.3 bis 4.5 | Sie/du vereinheitlicht, `content_type` der Landingpages geklärt, acht Glossareinträge ergänzt, UX-10/11/13 adressiert |
| Steuerungsdateien | `_qa/content-manifest.md` auf 24 Dateien und aktuelle Wortzahlen nachgeführt |

**Risiko bei Freigabe trotz offener Blocker:** Ein Betriebsteam ohne Konfigurationsreferenz und ein Modellierer ohne Element-Abbildung sind genau die zwei Zielgruppen, für die der Bestand am wenigsten bietet. Ein nicht verifiziertes Worker-Beispiel hat in Runde 1 bereits drei falsche Aufrufe enthalten — das Risiko ist belegt, nicht hypothetisch.

**Nächster Schritt:** Nachbesserungsrunde 2 mit den sieben Blockern als Abnahmeliste; erneute QA mit denselben sechs Dimensionen und derselben Schwelle.
