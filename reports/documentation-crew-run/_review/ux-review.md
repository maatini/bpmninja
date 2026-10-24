---
title: UX-Review — BPMNinja-Dokumentation
description: Nutzerreisen der fünf Rollen, belegte Befunde mit Schweregrad, die umgesetzten Änderungen an Landingpage, Lesepfaden und Querverweisen, offene Risiken und ein messbarer Verbesserungsplan mit Time to First Success, Absprungstellen und Support-Fragen.
audience:
  - Beitragende und Maintainer
  - Stakeholder und Entscheider
prerequisites:
  - Kenntnis der Landingpage unter README.md
  - Kenntnis der Informationsarchitektur unter _architecture/information-architecture.md
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - README.md
  - _architecture/information-architecture.md
  - _qa/content-manifest.md
  - _qa/verification-report.md
  - _review/glossary.md
  - _review/style-guide.md
  - how-to/prozess-deployen.md
---

# UX-Review — BPMNinja-Dokumentation

**Rolle:** Documentation Lead · **Stand:** 2026-10-24
**Auftrag:** Die Dokumentation für Einsteiger und Lernende, Prozessmodellierer, Entwickler und Integratoren, Betriebsteams und Stakeholder optimieren.
**Methode:** Primärlesung von Landingpage, Informationsarchitektur, Content-Manifest, Verifikationsbericht, Style Guide, Glossar und allen veröffentlichten Seiten; Prüfung aus fünf Rollenperspektiven mit je fünf Leitfragen; Umsetzung direkt in den Seiten; Beleg jeder Aussage durch Datei und Abschnitt.

**Einschränkung:** Es wurden **keine Nutzertests durchgeführt**. Jede Aussage über Nutzerverhalten ist eine **Annahme** und als solche gekennzeichnet. Belegt sind Aussagen über den Bestand: Dateien, Abschnitte, Linktexte, Wortzahlen, Prüfergebnisse.

**Werkzeuge (je einmal):** `doc_list`, `source_inventory` (49 Quelltexte), `check_links` (247 interne Links, 3 Befunde, **keiner in einer veröffentlichten Seite**), `verify_mermaid` (10 Diagramme, keine Syntaxbefunde). Beide Läufe erfolgten nach den Seitenänderungen und vor diesem Bericht; die vom Bericht selbst erzeugten Verweise sind nicht mitgezählt. Ein Rendering belegt `verify_mermaid` nicht; dafür steht `_qa/verification-report.md` §3 (10/10 SVG).

---

## 1. Ausgangslage

Der Bestand war inhaltlich weit und redaktionell vorstrukturiert: 13 Seiten mit vollständigem Front Matter, vier Lesepfade, ein Hilfe-Abschnitt, Metriken als Zielvorgabe. Die Lücke war nicht der Einstieg, sondern die **Passung zwischen Rollen und Seiten**.

| Kennzahl vor der Bearbeitung | Wert | Quelle |
|---|---:|---|
| Markdown-Dateien | 27 | `doc_list` |
| Veröffentlichte Seiten / Wörter | 13 / 27 591 | `doc_list` (das Manifest nennt 27 006) |
| Lesepfade | 4 (A–D) | `README.md` §Lesepfade |
| Rollen in IA / Rollen mit Lesepfad | 6 / 4 | IA §1 gegen `README.md` §Lesepfade |
| Seiten für „deployen und versionieren" | **0** (im Dateiplan **P0**) | IA §3.2 |
| Tabellen der Inhalte auf der Landingpage | 0 (nur Bullet-Listen) | `README.md` §Die vier Bereiche |
| Aussagen zur Suche / zur Barrierefreiheit für Leser | 0 / 0 | alle Seiten |

**Kernbefund:** Zwei Rollen fanden den Einstieg, aber nicht den zweiten Schritt. Der Prozessmodellierer hatte keinen Lesepfad, und die als P0 geplante Deploy-Seite fehlte — obwohl das Tutorial sie ankündigte und die Landingpage Modellierung als Aufgabe nannte. Wer ein Modell korrigieren wollte, landete in einer Element-Referenz ohne Rückweg zur Handlung.

**Nach der Bearbeitung:** 28 Markdown-Dateien, **14 veröffentlichte Seiten**, **30 326 Wörter** (27 591 aus 13 Seiten plus 2 735 aus vier erweiterten und einer neuen Seite).

---

## 2. Nutzerreisen — fünf Rollen

### 2.1 Einsteiger und Lernende

- **Wo startet sie?** `README.md` → „Was möchtest du tun?" → Tutorial (10–15 min). Das Tutorial steht jetzt zusätzlich in der Tabelle „Alle Seiten auf einen Blick".
- **Aufgabe in den ersten fünf Minuten:** NATS und Engine starten, `/api/health` prüfen. Neu in den Voraussetzungen: `jq` — ohne `jq` scheitert Tutorial-Schritt 4.
- **Nächster Schritt ohne Suche?** Ja: „Woran du erkennst, dass du fertig bist" → „Nächster sinnvoller Schritt" mit vier statt drei Folgezielen.
- **Unverständliche Begriffe:** Process Definition, Instance, Token, Wait State sind im Tutorial tabellarisch erklärt; der Rest über das Glossar, das der Suchwegweiser als ersten Ort nennt.
- **Woran erkennt sie, dass sie fertig ist?** Vier Abnahmekriterien mit Befehl und erwarteter Ausgabe.

*Annahme:* Einsteiger brechen ab, wenn der erste `cargo`-Build ohne Zwischenbeleg läuft — Risiko `R-03`.

### 2.2 Prozessmodellierer (fortgeschrittene Nutzer)

- **Wo startet sie?** Vorher ohne Lesepfad über die Zielgruppen-Tabelle. Nachher: Aufgabentabelle mit neuer Zeile „ein Modell deployen, korrigieren oder versionieren" und **Pfad E** mit drei Schritten.
- **Aufgabe in den ersten fünf Minuten:** „Wird mein Element akzeptiert?" war lösbar (§7 der Element-Referenz); „Wie bekomme ich Version 2?" **nicht** — jetzt `how-to/prozess-deployen.md` Schritt 4 mit erwarteter Antwort `version: 2`.
- **Nächster Schritt ohne Suche?** Vorher nein: `reference/bpmn-elemente.md` endete nach Belegstellen und **Offen**. Jetzt „Zugehörige Seiten" mit fünf Zielen; das Tutorial verweist auf den Deploy-Guide.
- **Unverständliche Begriffe:** `bpmn_process_id` vs. `definition_key` vs. `instance_id`, Flattening, Catch-all bei `errorRef` — erklärt im Glossar, in `explanation/konzepte.md` §7 und `reference/bpmn-elemente.md` §6.4.
- **Woran erkennt sie, dass sie fertig ist?** Vorher kein Kriterium. Jetzt vier: zwei Versionen existieren, feste Version startet, neueste startet, XML abrufbar.

*Annahme:* Modellierer suchen zuerst die Fehlermeldung, nicht das Konzept. Deshalb nennt die Landingpage-Fehlertabelle jetzt die Ablehnungsfälle der Element-Referenz.

### 2.3 Entwickler und Integratoren

- **Wo startet sie?** `README.md` → Aufgabentabelle → Worker-How-to; im Suchwegweiser ist „Symptom" der erste Einstieg.
- **Aufgabe in den ersten fünf Minuten:** Worker-Projekt, Modell-Deploy, Task abschließen. Der Modell-Deploy hängt nicht mehr am Tutorial, sondern ist im Deploy-Guide belegt (`415`/`422`, JSON-Vertrag).
- **Nächster Schritt ohne Suche?** Ja, „Nächster sinnvoller Schritt" mit drei Zielen.
- **Unverständliche Begriffe:** `lockDuration`/`newDuration`, `retries`/`maxRetries`, `complete`/`resolve`, `bpmnError`/`failure`; zusätzlich Versionskette und `start/latest`.
- **Woran erkennt sie, dass sie fertig ist?** Vier Abnahmekriterien inkl. `400` bei `lockDuration: 30`.

Am besten bediente Rolle. Einzige Lücke: das nicht belegte Registry-Paket (`R-01`); dokumentiert ist der Repository-Build.

### 2.4 Betriebsteams und Plattform-Administratoren

- **Wo startet sie?** `README.md` → „Konfiguration anpassen", plus Pfad C; der Suchwegweiser führt über „Default/Grenzwert" zur Konfigurationsreferenz.
- **Aufgabe in den ersten fünf Minuten:** `/api/health` und `/api/ready` prüfen, `BIND_ADDR` setzen — unverändert gut.
- **Nächster Schritt ohne Suche?** Ja, über Prüfliste und „Nächster sinnvoller Schritt"; die Fehlertabelle deckt jetzt auch „Log-Datei füllt das Dateisystem" ab.
- **Unverständliche Begriffe:** Readiness vs. Health, `REQUIRE_NATS`, Retention-Abgleich, Bucket-Namen (weiter mit Warnkasten als offen markiert).
- **Woran erkennt sie, dass sie fertig ist?** Neunzeilige Prüfliste in `how-to/konfiguration-anpassen.md`.

*Annahme:* Betriebsteams lesen zuerst die Fehlertabelle. Sie wuchs von 9 auf 12 Zeilen, je mit Zielseite.

### 2.5 Stakeholder und Entscheider

- **Wo startet sie?** `README.md` → „Architektur verstehen", plus Pfad D; Pfad D endet jetzt auf der Kompatibilitätsseite statt auf der REST-API.
- **Aufgabe in den ersten fünf Minuten:** Die drei Betriebsrisiken — `explanation/architektur.md` §7 als Tabelle, Erfolgskriterium in Pfad D.
- **Nächster Schritt ohne Suche?** Ja; neu erreichbar: „prüfen, ob sich etwas wie in Camunda verhält" → Abweichungsliste mit Migrationscheckliste.
- **Unverständliche Begriffe:** Flattening, Rhai, Migration vs. Modification, `OrphanedToken` (`explanation/konzepte.md` §5–8, Glossar).
- **Woran erkennt sie, dass sie fertig ist?** Erfolgskriterium in Pfad D.

*Annahme:* Entscheider lesen zwei Seiten. Pfad D hatte als dritten Schritt ein Vertragsdokument (REST-API) — für eine Eignungsfrage die falsche Reihenfolge; korrigiert.

---

## 3. Befunde

Schweregrad: **B** = verhindert die Aufgabe, **H** = kostet Zeit oder führt in die Irre, **N** = Kosmetik.

| # | Befund | Beleg | Schwere | Status |
|---|---|---|---|---|
| UX-01 | Aufgabentabelle nannte „deployen/versionieren" nicht | `README.md` §Was möchtest du tun? | **B** | behoben — neue Zeile |
| UX-02 | Rolle „Prozessmodellierer" ohne Lesepfad (A–D deckten vier Rollen) | `README.md` §Lesepfade vs. §Zielgruppen | **H** | behoben — Pfad E |
| UX-03 | P0-Seite „Prozess deployen und Versionen verwalten" fehlte; Versionierung nur in Bruchstücken | IA §3.2 | **B** | behoben — neue Seite (1 820 Wörter) |
| UX-04 | Tutorial kündigte an: „dafür gibt es später einen eigenen How-to-Guide" — Versprechen ohne Ziel | `tutorials/erster-prozess.md`, Einleitung | **H** | behoben — Satz verlinkt |
| UX-05 | Keine Tabelle der Inhalte und Seitentitel, nur vier Bullet-Listen | `README.md` §Die vier Bereiche | **H** | behoben — 14-Zeilen-Tabelle |
| UX-06 | Keine Aussage zur Suche, kein Wegweiser „Begriff → Ort" | alle 13 Seiten | **H** | behoben — „Suchen und finden" |
| UX-07 | Barrierefreiheit nur im Style Guide, nicht als Zusage an Leser | `_review/style-guide.md` §7 | **N** | behoben — Leserabschnitt |
| UX-08 | Fehlertabelle: 9 Zeilen, Deploy-Pfade und Log-Rotation fehlten | `README.md` §Hilfe bei Problemen | **H** | behoben — 12 Zeilen |
| UX-09 | Front Matter: `audience` ohne Prozessmodellierer; `related` ohne vier bestehende Seiten | `README.md` Front Matter | **H** | behoben (R5.6, R5.7) |
| UX-10 | Referenz-Landingpage ohne Suchzeile und Quelle der Wahrheit für Versionierung | `reference/README.md` §Was suchst du? | **H** | behoben — 3 Suchzeilen, 1 Quelle, 2 Schnellverweise |
| UX-11 | `reference/bpmn-elemente.md` endete ohne „nächsten Schritt" — Sackgasse für Modellierer | ebd., Seitenende | **H** | behoben — „Zugehörige Seiten" |
| UX-12 | Vier Referenzdetailseiten (`rest-api`, `service-task-api`, `cli`, `konfiguration`) ohne Navigationsblock | jeweiliges Seitenende | **H** | offen — §7 Schritt 4 |
| UX-13 | Keine Fehlerseite (404) | Bestand | **N** | offen — `R-07` |
| UX-14 | Glossar liegt als Leserseite unter `_review/glossary.md`; jede Seite führt dazu einen Offen-Punkt | `_review/glossary.md` | **H** | offen — §7 Schritt 3 |
| UX-15 | Content-Manifest veraltet: 13 Seiten / 27 006 Wörter statt 14 / 30 326 | `_qa/content-manifest.md` §1 | **H** | offen — §7 Schritt 1 |
| UX-16 | Landingpage trägt `content_type: reference`; ein Typ `landing` fehlt | R5.4 | **N** | offen — Entscheidung |
| UX-17 | Anker nur gegen GitHub-Slug-Regeln geprüft (57, 0 Defekte), nicht im Zielrenderer | `_qa/verification-report.md` §4 | **N** | offen — `R-05` |

**Bilanz:** 11 behoben, 6 offen mit benanntem nächstem Schritt. Kein Befund betrifft eine technische Aussage; alle betreffen Navigation, Auffindbarkeit und Rollenpassung.

---

## 4. Umgesetzte Änderungen

### 4.1 `how-to/prozess-deployen.md` — neu, 1 820 Wörter

Sieben Schritte (Modell prüfen, Version 1, Definitionsliste, Version 2, `start` vs. `start/latest`, XML abrufen, Löschen), erwartete Antwort nach jedem Schritt (`version: 1`, `version: 2`, `instance_id`, `text/plain`-XML, `409`), Abschnitt „Woran du erkennst, dass du fertig bist" mit vier Kriterien (R3.6), vier typische Fehler (`InvalidDefinition`, `415`/`422`, `409` beim Löschen, „neue Instanz auf alter Version"), Warnung vor `?cascade=true` und „Nächster sinnvoller Schritt" mit vier Zielen (R3.5).

**Belege:** `reference/rest-api.md` §Definitions/§Instances, `reference/bpmn-elemente.md` §Deploy-Regeln/§7, `tutorials/erster-prozess.md` §4, `explanation/konzepte.md` §5. Statuscodes für `DELETE` und die Wirkung von `cascade` sind als **nicht live geprüft** markiert; die Wirkung auf `history_instances` und `bpmn_xml` steht unter **Offen**.

### 4.2 `README.md` — 2 493 → 3 012 Wörter

| Änderung | Befund |
|---|---|
| Aufgabentabelle 13 statt 11 Zeilen (Deploy/Versionierung, Camunda-Vergleich) | UX-01 |
| „Danach"-Spalte rollenrichtig: Modellierer → Deploy-Guide, Stakeholder → Kompatibilität | UX-02 |
| Neuer Pfad E (Modellierer, ca. 25 min) mit Fertig-Kriterium | UX-02 |
| Pfad D endet auf der Kompatibilitätsseite statt auf der REST-API | §2.5 |
| Neue Tabelle „Alle Seiten auf einen Blick" (14 Zeilen: Seite, Bereich, Zweck, Zeit) | UX-05 |
| Neuer Abschnitt „Suchen und finden" (6 Suchwege, Aussage zur fehlenden Volltextsuche) | UX-06 |
| Neuer Abschnitt „Barrierefreiheit und mobile Nutzung" (6 Zusagen mit Beleg) | UX-07 |
| Fehlertabelle 12 statt 9 Zeilen | UX-08 |
| Front Matter: `audience` + Prozessmodellierer, `related` 14 Pfade | UX-09 |
| Voraussetzungen nennen `jq` | Fehlerquelle in Tutorial-Schritt 4 |
| Metriken: 0 Linkdefekte, 57 Anker, 4 von 4 Seitenabschluss | belastbare Zahlen |

### 4.3 `reference/README.md` — 1 325 → 1 504 Wörter

Drei neue Suchzeilen (Version deployen, Deploy-Statuscodes, Löschen mit Instanzen), eine Zeile „Versionskette `bpmn_process_id` → `version`" in den Quellen der Wahrheit, zwei neue Schnellverweise („Wie entsteht Version 2?", „Woran erkenne ich die Version einer Instanz?"), eine Zeile im Abschnitt „Verwandte Seiten", `related` um den Deploy-Guide erweitert. Behebt UX-10.

### 4.4 `tutorials/erster-prozess.md` — 2 747 → 2 853 Wörter

Ankündigung „später einen eigenen How-to-Guide" durch einen Link ersetzt (UX-04); Verweis im Deploy-Schritt auf den vollständigen Ablauf; „Nächster sinnvoller Schritt" um „eigenes Modell deployen" erweitert; `related` ergänzt; Voraussetzung `jq` aufgenommen.

### 4.5 `reference/bpmn-elemente.md` — 1 807 → 1 918 Wörter

Neuer Abschnitt „Zugehörige Seiten" mit fünf Fragen und Zielen (Deploy-Guide, Kompatibilität, Worker-How-to, Flattening-Erklärung, Statuscodes); Einleitung verweist auf den Deploy-Ablauf; `related` ergänzt. Behebt UX-11.

---

## 5. Offene Risiken

| # | Risiko | Auswirkung | Gegenmaßnahme |
|---|---|---|---|
| R-01 | Registry-Paket `@bpmninja/external-task-client` nicht belegt | Kopiertes `npm install` liefert kein oder ein fremdes Paket | Repository-Build ist der dokumentierte Weg (`how-to/external-task-worker-schreiben.md` Schritt 3) |
| R-02 | Bucket-Namen (`messages`, `history_instances`) nicht quellseitig bestätigt | Betriebsprüfung greift daneben; Abfrage liefert „leer", nicht „Fehler" | Warnkasten `reference/konfiguration.md` §9; Grep im Adapter |
| R-03 | Erster `cargo`-Build ohne Fortschrittsanzeige | Abbruch vor dem ersten Beleg (Annahme) | „Erste erfolgreiche Aktion" vor dem Tutorial; Prüfung im Nutzertest |
| R-04 | Kein Live-Lauf für `DELETE` und `cascade` | Ein Lösch-Beispiel kann falsch sein | Als **Offen** markiert; §7 Schritt 2 |
| R-05 | Anker nur gegen GitHub-Slugs geprüft | Ein Symptom-Link kann ins Leere springen | 57 Anker, 0 Defekte; Hinweis unter **Offen** auf jeder Quellseite |
| R-06 | Keine Nutzertests | Alle Verhaltensaussagen sind Annahmen | Test-Design in §7.3 |
| R-07 | Keine Fehlerseite (404) | Ein falscher Link endet ohne Orientierung | Vorschlag: „Diese Seite existiert nicht. Suche über **Alle Seiten auf einen Blick** oder **Hilfe bei Problemen**; melde den Pfad, der hierher geführt hat." |
| R-08 | Steuerungsdateien veralten (Content-Manifest, IA-Dateiplan) | Beitragende verlinken falsche Pfade | §7 Schritt 1 |

---

## 6. Was dieser Review nicht belegt

- **Kein Nutzertest:** TTFS, Absprungstellen und Support-Fragen sind **nicht erhoben**; alle Zielwerte sind Annahmen.
- **Kein Rendering im Zielsystem:** `verify_mermaid` prüft Syntax; das Rendering belegt nur `_qa/verification-report.md` §3 für `mmdc`.
- **Kein Live-Lauf** für die neuen Lösch-Beispiele und für `GET /api/definitions` als vollständiges Schema.
- **Keine Volltextsuche geprüft:** ob die veröffentlichende Website eine anbietet, liegt außerhalb dieses Verzeichnisses.

---

## 7. Messbarer Verbesserungsplan

### 7.1 Metriken

| Metrik | Definition | Erhebung | Baseline | Zielwert | Termin |
|---|---|---|---|---|---|
| **Time to First Success (TTFS)** | Erste Landingpage-Ansicht bis erstes `200` aus `/api/health` | Moderierter Test, Aufzeichnung, Zeitstempel | unbekannt *(Annahme: 8–12 min, davon 3–5 min Build)* | ≤ 10 min bei 4 von 5 Personen | 2026-12-15 |
| **Absprungstellen** | Seite und Abschnitt, an dem eine Sitzung ohne Abnahmekriterium endet | Testbeobachtung; später Abschnitts-Analytics | unbekannt *(Annahme: Tutorial-Schritt 2)* | ≤ 25 % Abbruch vor Schritt 4 | 2026-12-15 |
| **Support-Fragen** | Fragen, die „Hilfe bei Problemen" nicht beantwortet | Zählung je Kanal, Tag `docs-ux` | unbekannt | ≤ 2 neue Themen pro Monat | laufend ab 2026-11-01 |
| **Suchabbrüche** | Sitzungen ohne genutzte Tabelle und ohne Zielseite | Testbeobachtung; später Verweis-Analytics | unbekannt | ≤ 15 % der Sitzungen | 2026-12-15 |
| **Linkintegrität** | Anteil auflösbarer interner Links in veröffentlichten Seiten | `check_links` in CI | 247 Links, 3 Befunde, alle in `_review/style-guide.md` (Beispieltext) | 0 Befunde in veröffentlichten Seiten | erreicht 2026-10-24 |
| **Seitenabschluss** | Seiten mit Fertig-Kriterium bzw. `## Offen` | Skript über Überschriften | 4 von 4 Tutorial/How-tos; 14 von 14 mit `## Offen` | 100 % | erreicht |

### 7.2 Reihenfolge der Nachbesserung

1. **Steuerungsdateien nachziehen** (30 min): Manifest auf 14 Seiten und neue Wortzahlen; IA-Dateiplan auf „geschrieben" (UX-15, R-08).
2. **Live-Verifikation der Deploy-Seite** (45 min): `DELETE` mit und ohne `?cascade=true`, `GET /api/definitions`-Schema (R-04).
3. **Glossar-Ortsentscheidung** (15 min Entscheidung, 20 min Umbau): `reference/glossar.md` als kanonischen Ort oder öffentliche Deklaration von `_review/glossary.md` (UX-14).
4. **Navigationsblock nachrüsten** (4 × 15 min): „Zugehörige Seiten" auf `rest-api`, `service-task-api`, `cli`, `konfiguration` nach dem Muster aus `bpmn-elemente.md` (UX-12).
5. **Fehlerseite** (20 min): Text aus `R-07` in die Website-Konfiguration (UX-13).
6. **Nutzertest** (2 Wochen Vorlauf): fünf Personen; danach Baseline festschreiben (R-06).

### 7.3 Nutzertest-Design (noch nicht durchgeführt)

**Ziel:** TTFS, Absprungstellen und Suchabbrüche als Baseline. **Teilnehmer:** 5 Personen — 2 Einsteiger, je 1 Modellierer, Entwickler, Betriebsteam. **Ablauf:** 30 Minuten, Think-Aloud, Start auf `README.md`. **Aufgaben:** „Bring einen Prozess zum Laufen", „Deploy deine korrigierte Fassung als Version 2", „Prüfe, ob die Engine zu unserem Betrieb passt". **Erhoben:** Zeit bis erster erfolgreicher `curl`, letzter Abschnitt, Rücksprünge zur Landingpage, genutzte Tabellen, gestellte Fragen.

### 7.4 Abnahmekriterien dieses Reviews

| Kriterium | Beleg | Status |
|---|---|---|
| Fünf Rollen mit Einstieg auf der Landingpage | `README.md` §Zielgruppen und Einstiege | erfüllt |
| Fünf Lesepfade mit Erfolgskriterium | `README.md` §Lesepfade (A–E) | erfüllt |
| Alle Seitentitel an einer Stelle | `README.md` §Alle Seiten auf einen Blick (14 Zeilen) | erfüllt |
| Suchweg beschrieben | `README.md` §Suchen und finden (6 Wege) | erfüllt |
| Fertig-Kriterium und nächster Schritt auf jeder Handlungsseite | Tutorial + 3 How-tos | erfüllt (4 von 4) |
| Modellierer-Rolle hat eine Handlungsseite | `how-to/prozess-deployen.md` | erfüllt |
| Kein defekter Link in einer veröffentlichten Seite | `check_links` 2026-10-24: 3 Befunde, alle `_review/style-guide.md` | erfüllt |
| Barrierefreiheit für Leser sichtbar | `README.md` §Barrierefreiheit und mobile Nutzung | erfüllt |
| Metriken definiert | §7.1 (6 Metriken) | erfüllt |
| Alle Beispiele live belegt | – | **nicht erfüllt** (R-04) |
| Nutzerverhalten gemessen | – | **nicht erfüllt** (R-06) |

**Freigabeempfehlung:** Die UX-Arbeit dieses Laufs ist abgeschlossen und belegt. Die Dokumentation als Ganzes bleibt `status: draft`, bis `R-01`, `R-02` und `R-04` geschlossen sind — drei Punkte, die nicht redaktionell, sondern nur durch Quelltext- oder Live-Zugriff entschieden werden können.
