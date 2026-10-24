---
title: Content-Manifest — BPMNinja-Dokumentation
description: Redaktionelles Manifest aller Markdown-Dateien mit Titel, Content-Typ, Zielgruppe, Wortzahl, Quellen, Status und offenen Punkten; ergänzt um Diátaxis-Verteilung, Front-Matter-Prüfung, Link- und Rendering-Befunde, die verbleibenden Lücken und das Korrekturprotokoll des technischen Reviews Runde 2.
audience:
  - Beitragende und Maintainer
  - Stakeholder und Entscheider
prerequisites:
  - Kenntnis der Informationsarchitektur unter _architecture/information-architecture.md
  - Kenntnis der Diátaxis-Bereiche (tutorial, how-to, reference, explanation)
content_type: reference
status: reviewed
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _architecture/information-architecture.md
  - _audit/audit-report.md
  - _qa/verification-report.md
  - _review/technical-review.md
  - README.md
---

# Content-Manifest — BPMNinja-Dokumentation

Stand: **2026-10-24**, nach der technischen Prüfung Runde 2. Grundlage ist der tatsächliche Dateibestand unter `/Volumes/SSD2TB/work/bpmninja/reports/documentation-crew-run` (27 Markdown-Dateien). Die Wortzahlen sind über `doc_list` neu erhoben; die Spalten „Titel", „Content-Typ", „Zielgruppe" und „Status" stammen aus dem Front Matter. Gegenüber der Vorversion sind die Wortzahlen der vier im Review korrigierten Seiten aktuell (11 061 statt 11 066 in `reference/cli.md` usw., siehe §7).

## 1. Veröffentlichte Seiten (13)

| Datei | Titel | Content-Typ | Zielgruppe | Wörter | Quellen | Status | Offene Punkte |
|---|---|---|---|---:|---|---|---|
| `README.md` | BPMNinja — Dokumentation | reference | Einsteiger; Entwickler und Integratoren; Betriebsteams; Stakeholder und Entscheider | 2352 | Alle veröffentlichten Seiten | draft | Status-Absatz behauptet „noch nicht live abgeglichen" (Review W-01); Linkintegritäts-Metrik veraltet (W-07); Modellierer-Einstieg ohne `bpmn-elemente.md` (W-09) |
| `explanation/architektur.md` | Architektur verstehen | explanation | Einsteiger; Stakeholder; Beitragende; Betriebsteams | 3373 | `architecture/*`; `persistence-nats` | draft | KV-Bucket-Namen nicht quellseitig bestätigbar (Blocker B-02); „Single Writer" ohne ADR |
| `explanation/konzepte.md` | Zentrale Konzepte und Trade-offs | explanation | Einsteiger; Entwickler; Stakeholder; Prozessmodellierer | 2293 | `architecture/decisions.md`; `engine-core` | draft | **korrigiert (K-04)**: ungefangener `bpmnError` → `service_task`; Incident-Art `bpmn_error` als unbelegt markiert |
| `how-to/external-task-worker-schreiben.md` | External-Task-Worker schreiben und testen | how-to | Entwickler; Betriebsteams | 2641 | `external-task-client/interfaces.md`; Live-Lauf 2026-10-24 | draft | **korrigiert (K-03)**; **Blocker B-01**: npm-Paket nicht belegt, Schritte 3–7 nicht ausführbar |
| `how-to/konfiguration-anpassen.md` | Konfiguration anpassen | how-to | Betriebsteams; Entwickler; Beitragende | 2413 | `engine-server/src/**`; `persistence-nats/src/client.rs` | draft | CORS-/Key-Verhalten aus Quelltext belegt, nicht mit gesetzten Keys live geprüft |
| `reference/README.md` | Referenz | reference | Alle Zielgruppen | 1330 | Referenzseiten | draft | `403` bei `fetchAndLock` ohne Beleg (Review W-03); Endpunktzahl bewusst offen |
| `reference/bpmn-elemente.md` | BPMN-Elemente und ihre Abbildung | reference | Prozessmodellierer; Entwickler; Beitragende | 1758 | `bpmn-parser/**`; `engine-core/.../helpers.rs` | draft | Incident-Art bei Error End Event unbelegt (W-06); kein Live-Deploy je Elementtyp |
| `reference/cli.md` | CLI und Build-Befehle | reference | Beitragende; Betriebsteams; Entwickler | 1061 | `README.md`; `devbox.json` | draft | **korrigiert (K-05)**: Verweis auf `reference/konfiguration.md`; Fuzz-Targets unbelegt |
| `reference/kompatibilitaet.md` | Kompatibilität mit BPMN 2.0 und Camunda 7 | reference | Prozessmodellierer; Entwickler; Betriebsteams | 1359 | `bpmn-parser`; `engine-server`; Live-Lauf | draft | Lock-Dauer-Zeile ohne `extendLock`-Differenzierung (W-05) |
| `reference/konfiguration.md` | Konfiguration und Umgebungsvariablen | reference | Betriebsteams; Entwickler; Beitragende | 1619 | `engine-server/src/**`; `persistence-nats/src/client.rs` | draft | **Blocker B-02**: Bucket-Namen (`messages`/`message_catches`, `history_instances`/`history`/`history_completed_instances`, `bpmn_xml` KV vs. Object Store) |
| `reference/rest-api.md` | REST-API | reference | Entwickler; Betriebsteams | 2249 | `docs/openapi.yaml`; `engine-server/src/server/mod.rs`; Live-Lauf | draft | `403` bei `fetchAndLock` (W-03); gelöster Punkt steht unter „Offen" (W-04) |
| `reference/service-task-api.md` | Service-Task-API | reference | Entwickler; Betriebsteams | 1818 | `engine-server/src/server/tasks.rs`; Live-Lauf | draft | **korrigiert (K-01, K-02)**: `extendLock` ohne 400-Pfad, `retries: 3` belegt; `409` auf `bpmnError` noch nicht live belegt |
| `tutorials/erster-prozess.md` | Ersten Prozess ausführen | tutorial | Einsteiger; Entwickler; Betriebsteams; Stakeholder | 2740 | `docs/openapi.yaml`; Live-Lauf 2026-10-24 | draft | **W-02**: §Offen behauptet ausstehenden Live-Abgleich und akzeptierte XML-Bodies |

**Summe veröffentlicht:** 13 Dateien, **27 006 Wörter**.

## 2. Steuerungs-, Review- und QA-Artefakte (14)

| Datei | Titel | Wörter | Status | Offene Punkte |
|---|---|---:|---|---|
| `_architecture/diagram-plan.md` | Abschlussentscheidung — Mermaid-Diagramme | 2390 | – | D1–D5 eingefügt und mit `mmdc` gerendert (10/10) |
| `_architecture/information-architecture.md` | Informationsarchitektur | 3207 | – | Dateiplan weicht in Einzelfällen von den realen Pfaden ab |
| `_audit/audit-report.md` | Audit-Bericht | 3407 | – | Befunde in den Zielseiten abgearbeitet, sofern dort vermerkt |
| `_qa/content-manifest.md` | Content-Manifest | 1399 | reviewed | Dieses Manifest |
| `_qa/qa-report.md` | QA-Bericht, Iteration 0 | 3205 | draft | Wird je Lauf neu erzeugt |
| `_qa/revision-round-1.md` | Revisionsbericht Runde 1 | 3453 | – | Historischer Stand |
| `_qa/verification-report.md` | Verifikationsbericht — Live-Lauf und Rendering | 916 | reviewed | Belegt B-06, B-09 und die Vertragskorrekturen; Client-Paket ausdrücklich ausgenommen |
| `_research/domain-research.md` | Forschungsbericht | 3406 | – | Belegbasis der Explanation-Seiten |
| `_review/glossary.md` | Glossar | 2246 | draft | Keine |
| `_review/style-guide.md` | Style Guide | 2587 | draft | 2 defekte Bildverweise `ablauf.svg` (Review W-08) |
| `_review/style-report.md` | Abschlussbericht Stil | 3080 | draft | Nicht Teil der veröffentlichten Seiten |
| `_review/technical-review-round-1.md` | Technischer Nachreview Runde 1 | 3283 | – | Historischer Stand |
| `_review/technical-review.md` | Technischer Review Runde 2 | 2514 | reviewed | 2 Blocker, 9 offene Nacharbeiten, 5 eingespielte Korrekturen |
| `_review/ux-review.md` | UX-Review | 3496 | – | Nicht Teil der veröffentlichten Seiten |

**Summe Steuerung:** 14 Dateien, rund **38 600 Wörter**. **Gesamt:** 27 Dateien, rund **65 600 Wörter**.

## 3. Diátaxis-Verteilung

| Bereich | Dateien | Anzahl | Wörter |
|---|---|---:|---:|
| Tutorial | `tutorials/erster-prozess.md` | 1 | 2740 |
| How-to | `how-to/external-task-worker-schreiben.md`, `how-to/konfiguration-anpassen.md` | 2 | 5054 |
| Reference | `reference/README.md`, `reference/bpmn-elemente.md`, `reference/cli.md`, `reference/kompatibilitaet.md`, `reference/konfiguration.md`, `reference/rest-api.md`, `reference/service-task-api.md` | 7 | 11194 |
| Explanation | `explanation/architektur.md`, `explanation/konzepte.md` | 2 | 5666 |
| Landingpage | `README.md` | 1 | 2352 |
| **Gesamt** | | **13** | **27006** |

Die Verteilung ist funktional: ein durchgehendes Tutorial, zwei aufgabenorientierte How-tos, sieben Referenzseiten und zwei Explanation-Seiten. Die frühere Lücke — fehlende Konfigurations-, BPMN-Elemente- und Kompatibilitätsreferenz — ist geschlossen.

## 4. Front Matter

Alle 13 veröffentlichten Seiten tragen die neun Pflichtfelder `title`, `description`, `audience`, `prerequisites`, `content_type`, `status`, `last_reviewed`, `owner` und `related`. Alle stehen auf `status: draft`; die Freigabe erfolgt über die Blocker-Abarbeitung aus `_review/technical-review.md`, nicht über einen Statuswechsel in diesem Lauf.

## 5. Link-, Anker- und Rendering-Befunde

| Prüfung | Ergebnis | Beleg |
|---|---|---|
| Relative Datei- und Bildlinks (`check_links`) | **202 Links, 2 defekt** — beide in `_review/style-guide.md` (`ablauf.svg`), keine veröffentlichte Seite betroffen | `check_links`, Lauf 2026-10-24 |
| Anker | nicht geprüft (Prüfwerkzeug unterstützt keine Fragmente); Seiten führen das unter **Offen** | `check_links`-Hinweis; `README.md` §Offen |
| Mermaid-Syntax (`verify_mermaid`) | 10 Diagramme, keine Befunde | `_qa/verification-report.md` §3 |
| Mermaid-Rendering (`mmdc`) | **10/10 SVG** | `_qa/verification-report.md` §3 |
| REST-Beispiele | gegen laufende Engine geprüft (Deploy, Start, Instances, Tasks, Service Tasks, Incidents, Body-Limit) | `_qa/verification-report.md` §2 |

Die in der Vorversion genannten Zahlen („57 Anker, 0 Defekte", „135 von 139 Links") sind damit überholt; die Abweichung ist als Review-Befund W-07 dokumentiert.

## 6. Verbleibende Lücken

1. **Client-Paket (Blocker B-01).** `@bpmninja/external-task-client` ist nicht als Registry-Paket nachgewiesen; die Schritte 3–7 des Worker-How-tos sind ohne Repository-Build nicht ausführbar.
2. **Persistierte Strukturnamen (Blocker B-02).** KV-Buckets, Object Stores und Stream-Namen sind zwischen `reference/konfiguration.md`, `persistence/gotchas.md` und `.agent/rules/NATS_MESSAGE_BROKER.md` in drei Punkten widersprüchlich; ein Grep im Adapter entscheidet.
3. **OpenAPI-Drift.** `docs/openapi.yaml` nennt für `bpmnError` `400`; die Engine liefert `204` und legt einen Incident an. Die Dokumentation folgt der Engine; der CI-Abgleich fehlt.
4. **Nicht abgedeckte Betriebsthemen.** Keine Seiten für Fehlercodes, Metriken, Betriebs-Runbooks und Grenzen/Trade-offs. Diese Lücken werden in den Landingpages nicht als vorhanden suggeriert.

## 7. Korrekturprotokoll aus dem technischen Review Runde 2

| ID | Datei | Änderung | Status |
|---|---|---|---|
| K-01 | `reference/service-task-api.md` | `newDuration` hat **keinen** `400`-Pfad; Grenzen gelten nur für `fetchAndLock` (vorher widersprüchlich zu §4, zu `rest-api.md` und zum Live-Lauf) | behoben |
| K-02 | `reference/service-task-api.md` | Default-`retries: 3` als belegt geführt, nicht mehr als „nicht belegt" | behoben |
| K-03 | `how-to/external-task-worker-schreiben.md` | Ungefangener `bpmnError` → Incident `service_task` (vorher `bpmn_error`); Abgleich gegen `errorCode` statt `errorRef`; Admin-only-Hinweis für `GET /api/service-tasks` | behoben |
| K-04 | `explanation/konzepte.md` | Incident-Arten konsistent zu K-03; `bpmn_error` als unbelegt markiert; `resolve` löst Incident nicht automatisch auf | behoben |
| K-05 | `reference/cli.md` | Verweis auf `reference/konfiguration.md` statt „noch fehlende Konfigurationsreferenz"; `related` ergänzt | behoben |

Offen und nicht Teil dieses Protokolls: B-01, B-02 (Blocker) sowie W-01 bis W-09 (Nacharbeiten) — Details, Belege und Korrekturtexte in `_review/technical-review.md`.
