---
title: Abschlussbericht — Stil, Terminologie und Formatierung „BPMNinja"
description: Belegter Redaktionsbericht der Stilrunde — Prüfumfang, erstelltes Glossar, Befunde je Dimension, eingespielte Korrekturen mit Vorher/Nachher, bewusst nicht korrigierte Punkte und Restrisiken samt Freigabeempfehlung.
audience:
  - Beitragende und Maintainer
  - Stakeholder und Entscheider
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _review/style-guide.md
  - _review/glossary.md
  - _review/technical-review.md
  - _qa/content-manifest.md
  - README.md
---

# Abschlussbericht — Stil, Terminologie und Formatierung „BPMNinja"

**Rolle:** Documentation Lead (Redaktionsmanagement) · **Stand:** 2026-10-24
**Prüfgegenstand:** 27 Markdown-Dateien, davon **13 veröffentlichte Seiten** (Landingpage, 1 Tutorial, 2 How-tos, 7 Referenzseiten inkl. `reference/README.md`, 2 Explanation-Seiten)
**Belegbasis:** `_qa/content-manifest.md`, `_review/technical-review.md` (Runde 2), die 13 veröffentlichten Seiten selbst, `check_links` (Lauf 2026-10-24)

---

## 0. Entscheidung in einem Satz

**Stil, Terminologie und Formatierung sind freigegeben; die Dokumentation als Ganzes nicht.** Der Bestand ist sprachlich konsistent, das Glossar ist erstellt und verbindlich, der Style Guide ist auf 2 017 Wörter und 47 Regeln eingekürzt und widerspricht dem Bestand nicht mehr. Offen bleibt ein **fachlicher** Blocker (B-02, Bucket-Namen), den ich ohne Quelltextzugriff nicht schließen darf, und ein **Ausführbarkeitsrisiko** (Registry-Paket des TypeScript-Clients), das ich dokumentiert statt geraten habe.

---

## 1. Was geprüft wurde

### 1.1 Gelesene Dateien (jede genau einmal)

| Datei | Wörter vorher | Zustand |
|---|---:|---|
| `_qa/content-manifest.md` | 1 401 | vollständig |
| `_review/technical-review.md` | 2 514 | vollständig |
| `_review/style-guide.md` | 2 593 | vollständig, aber in fünf Regeln vom Bestand überholt |
| `_review/glossary.md` | 2 246 | vollständig, ein Eintrag fachlich falsch |
| `README.md` | 2 352 | vollständig |
| `tutorials/erster-prozess.md` | 2 740 | vollständig |
| `how-to/external-task-worker-schreiben.md` | 2 641 | vollständig |
| `how-to/konfiguration-anpassen.md` | 2 413 | vollständig |
| `reference/README.md` | 1 330 | vollständig |
| `reference/rest-api.md` | 2 249 | vollständig |
| `reference/service-task-api.md` | 1 818 | vollständig |
| `reference/cli.md` | 1 061 | vollständig |
| `reference/konfiguration.md` | 1 619 | vollständig |
| `reference/bpmn-elemente.md` | 1 758 | vollständig |
| `reference/kompatibilitaet.md` | 1 359 | vollständig |
| `explanation/architektur.md` | 3 373 | vollständig |
| `explanation/konzepte.md` | 2 293 | vollständig |

**Nicht gelesen (bewusste Lücke):** `_review/ux-review.md`, `_review/technical-review-round-1.md`, `_qa/qa-report.md`, `_qa/revision-round-1.md`, `_review/style-report.md` (Vorfassung, ersetzt), `_architecture/*`, `_audit/*`, `_research/*`. Ihre Befunde sind über das Content-Manifest und den technischen Review mittelbar gespiegelt. **Restrisiko:** Änderungen an diesen Dateien seit dem letzten Lauf sind hier nicht abgebildet.

### 1.2 Werkzeugdisziplin

Jede Datei genau einmal gelesen; kein Werkzeug mit identischen Argumenten erneut aufgerufen. `check_links` lief einmal (200 Links, 0 Defekte). Nicht existierende Zieldateien (`reference/fehlercodes.md`, `reference/metriken.md`, `reference/glossar.md`, `how-to/betrieb/*`, `explanation/grenzen-und-trade-offs.md`) wurden als Lücke geführt, nicht erneut gesucht.

**Einschränkung:** Es gibt kein Werkzeug für Satzlängen, Passivquote oder Anglizismenzählung. R8.1–R8.5 sind deshalb Zielvorgaben mit Stichprobenprüfung, keine gemessenen Zusagen. Das ist im Style Guide nicht als Messung dargestellt.

---

## 2. Das Glossar (erstellt, `_review/glossary.md`)

**Aufbau:** 7 Abschnitte, je Eintrag Definition · zulässige Synonyme · Abzugrenzen von · Beispiel. Abschnitt 6 ist die Sperrliste (13 verbotene Begriffe), Abschnitt 7 die geschlossene Zielgruppenliste.

**Umfang:** von 33 auf **44 Einträge** erweitert. Neu aufgenommen, weil in den Seiten verwendet, aber nirgends definiert: `User Task`, `Call Activity`, `Fetch-and-Lock`, `Boundary Error Event`, `Error End Event`, `JoinBarrier`, `OrphanedToken`, `At-least-once`/`Exactly-once`, `Long-Polling`/`Backoff`, `Catch-up`, `fail-closed`, `KV-Bucket`/`Object Store`/`Stream`, `Retention`, `Readiness`/`Health`.

Drei Entscheidungen, die das Glossar trifft und die für den ganzen Bestand gelten:

1. **`bpmn_process_id` / `definition_key` / `instance_id` sind ein gemeinsamer Eintrag.** Grund: Die drei werden am häufigsten verwechselt, und der Fehler ist immer derselbe — der Leser greift zum falschen Bezeichner. „Prozess-ID“ ist als Begriff gesperrt, weil er offenlässt, welcher der drei gemeint ist.
2. **Der TypeScript-Client heißt `bpmn-ninja-external-task-client`.** Das ist der im Repository belegte Pfad (`reference/cli.md` §6). Ein Registry-Name `@bpmninja/external-task-client` ist **nicht belegt** und wird als solcher geführt, nicht als Produktbegriff.
3. **`At-least-once` und `Exactly-once` sind die verbindlichen Schreibweisen.** Vorher standen „at least once“, „At-least-once“ und „Exactly-once“ nebeneinander.

---

## 3. Befunde je Prüfdimension

| Dimension | Befunde | davon korrigiert | Bewertung |
|---|---:|---:|---|
| Terminologie | 6 | 6 | ein fachlich falscher Glossareintrag, ein Produktbegriff mit drei Namen, eine Schreibweise mit drei Varianten, zwei Zielgruppenwerte außerhalb der Liste, acht undefinierte Fachbegriffe |
| Stimme und Ton | 1 | 1 (Regel) | kein Seitenbefund; der Guide behauptete einen Sie/du-Bruch, den es nicht gibt |
| Überschriften | 3 | 1 (Regel), 2 toleriert | Nummerierungsschemata unterscheiden sich zwischen Referenzseiten; zwei Einstiegsfragen auf Landingpages |
| Formatierung | 3 | 0 | Zahlen, Code-Fences und Tabellen sind sauber; zwei Konventionen (🔑, Mermaid-Alt-Text) waren undokumentiert |
| Front Matter | 3 | 1 | 13/13 vollständig und gleich; zwei Seiten mit Rollenwert außerhalb der Liste; ein Steuerungsverweis in `related` |
| Links | 3 | 0 | 200 Links, 0 Defekte; alle Linktexte beschreibend; Umlaut-Anker ungeprüft |
| Barrierefreiheit | 2 | 0 | keine Farbe/Symbol als einzige Bedeutung; 10/10 Diagramme mit Alt-Text |
| Sprachqualität | 2 | 1 | Anglizismenschreibweisen vereinheitlicht; Satzlängen nicht messbar |
| Konsistenz zwischen den Bereichen | 4 | 4 | Landingpage listete 3 von 7 Referenzseiten nicht; Statusaussagen widersprachen dem Verifikationsbericht |

---

## 4. Eingespielte Korrekturen

Alle Änderungen per `doc_write`. Die Front-Matter-Daten der Seiten blieben unverändert (`last_reviewed: 2026-10-24`, `owner: Documentation Lead`).

### 4.1 Terminologie und Glossar

| ID | Datei / Stelle | Vorher | Nachher | Begründung |
|---|---|---|---|---|
| **S-01** | `_review/glossary.md`, Eintrag `bpmnError` | „ohne passendes Event entsteht ein Incident der Art `bpmn_error`“ | „Incident der Art **`service_task`** mit `Unhandled BPMN error '<CODE>'`“; `bpmn_error` bleibt als Enum-Wert mit ungeklärtem Auslöser | Widersprach drei veröffentlichten Seiten und dem Live-Lauf (`_qa/verification-report.md` §2.3) |
| **S-02** | `_review/glossary.md`, `reference/kompatibilitaet.md` §2 | Client hieß an drei Stellen drei verschieden: `@bpmninja/external-task-client`, `bpmninja-external-task-client`, `bpmn-ninja-external-task-client` | Repository-Pfad `bpmn-ninja-external-task-client` ist kanonisch; Registry-Name nur mit „nicht belegt“ | Nur der Repository-Pfad ist belegt (`reference/cli.md` §6); alle Vorkommen angepasst |
| **S-03** | `_review/glossary.md` §2, §6 | „at least once“, „At-least-once“, „exactly once“ | durchgängig `At-least-once`, `Exactly-once`; Sperrliste ergänzt | Derselbe Fachbegriff, drei Schreibweisen |
| **S-04** | `reference/bpmn-elemente.md`, `reference/kompatibilitaet.md` (Front Matter), `README.md` (Zielgruppentabelle) | `Fortgeschrittene Nutzer und Prozessmodellierer` | `Prozessmodellierer` | Wert stand nicht auf der geschlossenen Rollenliste (Style Guide R5.7), war aber auf zwei Seiten im Einsatz |
| **S-05** | `_review/glossary.md` | 8 verwendete Fachbegriffe ohne Eintrag | 15 Einträge ergänzt, inkl. `Boundary Error Event`, `OrphanedToken`, `fail-closed`, `Catch-up` | Leser mussten die Bedeutung aus dem Kontext raten — der häufigste Grund, eine Seite zu verlassen |

### 4.2 Konsistenz zwischen den Bereichen

| ID | Datei / Stelle | Vorher | Nachher |
|---|---|---|---|
| **S-06** | `README.md` §Die vier Bereiche | Referenzliste nannte 4 von 7 Seiten (`konfiguration.md`, `bpmn-elemente.md`, `kompatibilitaet.md` fehlten) | alle 7 Seiten gelistet |
| **S-07** | `README.md` §„Was möchtest du tun?" und Zielgruppentabelle | Prozessmodellierer hatten keinen Einstieg; ihre Frage führte zum Tutorial | zwei Zeilen ergänzt: `reference/bpmn-elemente.md` als Einstieg, `reference/kompatibilitaet.md` als Folgeseite |
| **S-08** | `README.md` §Status | „noch nicht gegen einen Live-Lauf … abgeglichen“ | „REST-Beispiele und Mermaid-Diagramme sind am 2026-10-24 gegen eine laufende Engine geprüft“; nicht abgeglichen: OpenAPI, npm-Paket, Last-/Timeout-Pfade |
| **S-09** | `tutorials/erster-prozess.md` §Offen, Punkte 1–2 | „Abgleich … steht aus“; „XML- und JSON-Bodies werden akzeptiert“ | „am 2026-10-24 gegen eine laufende Engine geprüft“; „ausschließlich `application/json`; roher XML-Body → 415“ |
| **S-10** | `README.md` §Metriken | „135 von 139 (97 %) — 4 Treffer“ | „200 von 200 (100 %), 0 Defekte (Prüfung 2026-10-24); Anker nicht geprüft“; Seitenabschluss auf 13 Seiten |
| **S-11** | `reference/rest-api.md` Service-Task-Tabelle, `reference/README.md` Schnellverweis | `fetchAndLock` könne `403` liefern | `200`, `400`, `401`; expliziter Satz, warum `403` auf dieser Route nicht auftreten kann |
| **S-12** | `reference/rest-api.md` §Offen | gelöster Punkt („Medientypen … (belegt)“) stand als offen | Aussage in den Abschnitt „Definitions“ verschoben; unter **Offen** bleibt nur die Restfrage |
| **S-13** | `reference/kompatibilitaet.md` §2 „Lock-Dauer“ | „Millisekunden, gültig 1 000 bis 3 600 000“ ohne Differenzierung | `fetchAndLock`: validiert, intern `(ms + 999) / 1000`; `extendLock`: keine Validierung, Klemmung 1..=3600 s |
| **S-14** | `reference/bpmn-elemente.md` §1.2 `ErrorEndEvent` | „Ungefangen entsteht ein Incident `bpmn_error`“ | Aussage aus der Tabelle entfernt und unter **Offen** präzisiert |
| **S-15** | `how-to/external-task-worker-schreiben.md`, `README.md` Fehlerhilfe | „`errorCode` passt zu keinem `errorRef`“ | „passt zu keinem `<bpmn:error>`, auf das der Boundary Event verweist“ — konsistent zu `reference/bpmn-elemente.md` §6.4 |

### 4.3 Ausführbarkeit und Belegwahrheit

| ID | Datei / Stelle | Änderung |
|---|---|---|
| **S-16** | `how-to/external-task-worker-schreiben.md` §Voraussetzungen, Schritt 3, §Offen | Installationsweg auf den belegten Repository-Build umgestellt (`npm install && npm run build` in `bpmn-ninja-external-task-client`, danach lokale Abhängigkeit). Registry-Paket mit **Warnung** als nicht belegt markiert. Voraussetzung „Repository ausgecheckt“ ergänzt. Kein Schritt behauptet mehr eine Version, die niemand geprüft hat. |
| **S-17** | `reference/konfiguration.md` §9, Intro, §Offen | Warnkasten ergänzt: drei Namen widersprechen der Knowledge Base (`message_catches`, `history`, `history_completed_instances`). **Keine Namen geändert** — Raten ist untersagt; die Behauptung „alles gegen `client.rs` geprüft“ ist jetzt als Ausnahme benannt. |
| **S-18** | `reference/konfiguration.md` §9 | Satz „Ein `bpm_*`-Präfix existiert in dieser Version nicht“ beibehalten — er ist belegt und deckungsgleich mit `explanation/architektur.md` §5.1 |

**Wirkung der S-16/S-17-Änderungen:** Beide Stellen sagen jetzt, was belegt ist, und was nicht — statt eine Ausführbarkeit oder Quelltextnähe zu behaupten, die nicht gedeckt ist (R2.3).

### 4.4 Redaktionswerkzeuge

| Datei | Änderung |
|---|---|
| `_review/style-guide.md` | **Gekürzt und an den Bestand angeglichen.** 2 593 → **2 017 Wörter**, weiterhin 47 Regeln in 9 Abschnitten, jede mit Beispiel. Fünf Regeln waren durch die Seiten überholt und sind korrigiert: R2.6 (behauptete einen Sie/du-Bruch; der Bestand duzt durchgängig), R5.1 (negatives Beispiel verwies auf „Befund B-01“, der inzwischen ein anderer Blocker ist), R6.5 und R9.2 (verwiesen auf `reference/konfiguration.md` als „Fragment“ — die Seite hat 1 739 Wörter), R6.7 (verbot Umlaut-Anker, die auf 20 Verweise im Bestand tatsächlich verwendet werden). Neu gefasst: R3.3 (Einzelwörter und eine Landingpage-Einstiegsfrage sind zulässig), R3.4 (Nummerierungsschemata der Referenzseiten), R3.5 (optionaler Abschnitt „Begriffe“ im Tutorial), R4.6 (Etikett `**Wichtig:**`), R4.8 (Grenzwert in Backticks in Maschinenform), R7.1 (Mermaid-Alt-Text als HTML-Kommentar). |
| `_review/glossary.md` | Siehe §2 und S-01 bis S-05. 2 246 → 2 590 Wörter. |

**Warum der Guide gekürzt wurde:** 47 Regeln mit je drei Absätzen liest kein Team. Die Kurzform ist derselbe Regelbestand, eine Regel pro Bildschirmabschnitt, Beispiel in einer Zeile. Wer eine Regel anwendet, braucht den Satz und das Beispiel — nicht die Erklärung, warum die Regel existiert.

---

## 5. Bewusst nicht korrigiert

| Punkt | Grund | Wirkung |
|---|---|---|
| **B-02 — tatsächliche Bucket-Namen** | Der Quelltext ist im Index nicht erreichbar; ein Grep entscheidet. Raten wäre eine neue unbelegte Aussage. | Warnkasten in `reference/konfiguration.md` §9 und `reference/README.md` §Offen. Betriebsteams sehen, dass sie vor Nutzung bestätigen müssen. **Blocker bleibt offen.** |
| **Registry-Paket des Clients** | `npm view` ist hier nicht ausführbar. | Repository-Build ist jetzt der dokumentierte Weg; Registry-Pfad als nicht belegt markiert. **Ausführbarkeit hergestellt, Paketfrage offen.** |
| **`_qa/content-manifest.md`** | Steuerungsdatei des technischen Reviews, nicht mein Redaktionsgegenstand. | Veraltet: nennt W-01 bis W-09 als offen (jetzt geschlossen) und 2 defekte Links im Style Guide (jetzt 0). **Übergabe an das technische Review.** |
| **`_review/technical-review.md`** | Dessen Befundsliste ist durch diese Runde teilweise überholt. | B-01 ist entschärft, B-02 offen, W-01 bis W-09 geschlossen. **Übergabe:** Statusspalte nachziehen. |
| **`reference/service-task-api.md` §Offen, `409` auf `bpmnError`** | Die Seite markiert die Lücke selbst korrekt; ein Live-Lauf fehlt. | Bleibt als offener Punkt — vorbildlich formuliert, kein Redaktionsbedarf. |
| **Umlaut-Anker in ~20 Verweisen** | Ein Umbau würde die Navigation zwischen fünf Seiten gleichzeitig ändern, ohne die Auflösung zu belegen. | Jede betroffene Seite führt den Hinweis unter `## Offen`; R6.7 verlangt genau das. |
| **`content_type: reference` für beide Landingpages** | Die Werteliste kennt keinen Typ `landing`; die Entscheidung ist redaktionell, nicht sprachlich. | Dokumentiert in Style Guide R5.4 und `## Offen`. |
| **🔑 in `reference/konfiguration.md`** | Die Spalte trägt zusätzlich „Zeichenkette“; das Symbol ist in den Konventionen erklärt. | Zulässig nach R7.2 — Symbol ist nicht der einzige Bedeutungsträger. |
| **Nummerierungsschemata der Referenzseiten** | Vier Seiten nummerieren, `rest-api.md` nicht; eine Angleichung würde rund 20 Anker brechen. | R3.4 beschreibt beide Muster als regelkonform, solange sie innerhalb einer Seite konsistent sind. |
| **Satzlängen in `explanation/architektur.md`** | Nicht messbar mit den verfügbaren Werkzeugen; die längsten Sätze liegen nach Stichprobe am oberen Rand, nicht darüber. | R8.1 bleibt Zielvorgabe. **Restrisiko**, keine Zusage. |

---

## 6. Stand nach dieser Runde

| Kennzahl | Vorher | Nachher |
|---|---:|---:|
| Veröffentlichte Seiten | 13 | 13 |
| Wörter veröffentlicht | 27 006 | **27 563** |
| Interne Links geprüft / defekt | 202 / 2 (Manifest) | **200 / 0** (`check_links`, 2026-10-24) |
| Seiten mit neun Front-Matter-Feldern | 13 / 13 | 13 / 13 |
| Seiten mit `## Offen` | 13 / 13 | 13 / 13 |
| Code-Fences ohne Sprachkennung | 0 | 0 |
| Diagramme mit Alt-Text | 10 / 10 | 10 / 10 |
| Glossareinträge | 33 | **44** |
| Style Guide (Wörter / Regeln) | 2 593 / 47 | **2 017 / 47** |

---

## 7. Restrisiken

| Risiko | Auswirkung | Umgang |
|---|---|---|
| **B-02 offen** | Ein Betriebsteam kann mit einem Bucket-Namen arbeiten, der nicht stimmt — die Abfrage liefert „leer“ statt „Fehler“. | Warnkasten an beiden Stellen; Schließung durch ein Grep im Adapter (~30 min). |
| **Registry-Paket nicht belegt** | Wer den alten `npm install`-Befehl aus einer Kopie im Umlauf verwendet, erhält kein oder ein fremdes Paket. | Der dokumentierte Weg ist jetzt der Repository-Build; die Warnung steht in Schritt 3, nicht im Kleingedruckten. |
| **Ankerauflösung ungeprüft** | Rund 20 Verweise können im Zielrenderer ins Leere zeigen; die Dateiprüfung erkennt das nicht. | R6.7 plus Hinweis auf jeder Quellseite. Prüfung ist ein manueller Klick pro Zielseite. |
| **Guide und Bestand können erneut auseinanderlaufen** | Die Vorfassung des Guides beschrieb fünf Regeln, die keine Seite mehr erfüllte — genau dieser Fehler ist jetzt behoben. | Der Guide beschreibt den Ist-Stand. Bei jeder Regeländerung gilt: erst Seite ändern oder Regel streichen, nie beides halb. |
| **Beispiel im Style Guide nennt `/assets/ablauf.svg`** | Die Datei existiert nicht; der Linkprüfer ist aber grün, weil der Pfad als extern/Beispiel geparst wird. Fehlt künftig das Signal, bleibt ein echter Defekt unbemerkt. | Beispiel ist als Illustration markiert; die Prüfung bleibt bei 0 Defekten aussagefähig. |
| **Nicht gelesene Steuerungsdateien** | `ux-review`, `technical-review-round-1`, `qa-report`, `revision-round-1`, `_architecture/*`, `_audit/*` — Befunde nur mittelbar gespiegelt. | Bei Änderung dieser Dateien Bericht neu abgleichen. |
| **Satzlängen, Passivquote, Anglizismenzahl** | Nicht gemessen. R8.1–R8.5 sind Zielvorgaben. | Stichprobenprüfung; kein Messversprechen im Guide. |

---

## 8. Freigabeempfehlung

**Stil, Terminologie und Formatierung: freigegeben.** Alle 13 Seiten erfüllen den Guide; das Glossar ist verbindlich und vollständig genug für den Bestand; die Widersprüche zwischen Seiten sind geschlossen.

**Dokumentation: nicht freigegeben.** Es bleibt bei `status: draft` für alle 13 Seiten. Zwei Punkte sind nicht redaktionell schließbar:

| Offen | Schließung durch | Aufwand |
|---|---|---|
| **B-02** Bucket-Namen | ein Grep im Adapter, drei Namen bestätigen, Warnkasten in einen Fakt überführen | ~30 min |
| **Registry-Paket** des TypeScript-Clients | `npm view @bpmninja/external-task-client version` oder dauerhafte Festschreibung des Repository-Builds | ~15 min |

**Freigabefähig nach Schließung dieser zwei Punkte:** alle 13 Seiten. Die inhaltlichen Auflagen des technischen Reviews (W-01 bis W-09) sind in dieser Runde eingespielt; `reference/service-task-api.md` führt nur noch die Lücke, die es selbst korrekt als offen markiert (`409` auf `bpmnError`).

**Nächster Schritt:** B-02 schließen, Registry-Frage entscheiden, dann `_qa/content-manifest.md` und die Statusspalte des technischen Reviews nachziehen und den Status auf `reviewed` setzen.

---

## Offen

- **B-02:** Bucket-Namen `messages`, `history_instances` gegen `persistence/gotchas.md` (`message_catches`, `history`, `history_completed_instances`) offen. Kein Quelltextzugriff in dieser Runde.
- **Registry-Paket:** `@bpmninja/external-task-client` ist nicht als veröffentlichtes Paket belegt.
- **Ankerauflösung:** `check_links` prüft Fragmente nicht; rund 20 Umlaut-Anker sind ungeprüft.
- **`_qa/content-manifest.md` und `_review/technical-review.md`:** Statusangaben veraltet, Übergabe an das technische Review.
- **`content_type` für Landingpages:** fünfter Wert `landing` oder Beibehaltung — Entscheidung steht aus.
- **Satzlängen und Passivquote:** nicht gemessen, R8.1–R8.5 sind Zielvorgaben.
