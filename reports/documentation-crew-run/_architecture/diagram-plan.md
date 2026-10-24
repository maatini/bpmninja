---
title: Diagrammplan — BPMNinja-Dokumentation
description: Welche Mermaid-Diagramme die Dokumentation enthält, welche eine einzige Frage jedes Diagramm beantwortet, worauf es sich belegt, auf welcher Seite es steht, welchen Alternativtext es trägt und welche Kandidaten mit Begründung verworfen wurden.
audience:
  - Beitragende und Maintainer
  - Stakeholder und Entscheider
prerequisites:
  - Kenntnis der Informationsarchitektur unter _architecture/information-architecture.md
  - Kenntnis der Diátaxis-Bereiche (tutorial, how-to, reference, explanation)
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _architecture/information-architecture.md
  - _qa/content-manifest.md
  - _qa/verification-report.md
  - explanation/architektur.md
  - explanation/konzepte.md
  - reference/service-task-api.md
---

# Diagrammplan — BPMNinja-Dokumentation

**Stand:** 2026-10-24 · **Owner:** Documentation Lead · **Prüfung:** `verify_mermaid`, 6 Diagramme in veröffentlichten Seiten, keine Syntaxbefunde.

Dieser Plan ist die Redaktionsakte der Diagrammarbeit. Er beantwortet für jedes Diagramm vier Fragen: **Welche eine Frage beantwortet es**, **worauf ist es belegt**, **wo steht es** und **welchen Alternativtext trägt es**. Er enthält bewusst **keine Kopie der Diagrammquellen** — die Quelle der Wahrheit ist die Seite selbst. Eine Kopie im Plan würde bei jeder Textänderung auseinanderlaufen; die frühere Fassung dieses Plans enthielt fünf solcher Doppelungen, sie sind entfernt.

## 1. Aufnahmeregeln

Ein Diagramm kommt nur in eine Seite, wenn alle fünf Bedingungen erfüllt sind:

1. **Eine Frage.** Das Diagramm beantwortet genau eine Frage, die der umgebende Text stellt, aber nicht in einem Bild auflöst.
2. **Neue Beziehung.** Es zeigt eine Beziehung, die der Text nicht schon als nummerierte Liste trägt. Eine Schrittfolge, die im Text numeriert ist, wird nicht nachgezeichnet.
3. **Belegt.** Jeder Knoten und jede Kante ist durch Text, Quelle oder Live-Lauf gedeckt. Nicht belegte Semantik wird weggelassen, nicht geraten.
4. **Lesbar ohne Farbe.** Bedeutung liegt in Beschriftung, Strichart und Leserichtung — nie allein in der Farbe.
5. **Alt-Text und Erklärungssatz.** Direkt beim Block steht ein erklärender Satz und ein Alt-Text als HTML-Kommentar.

## 2. Übersicht

| ID | Zielseite und Abschnitt | Typ | Die eine Frage | Primaere Quelle | Alternativtext (Kurzfassung) | Prüfstand |
|---|---|---|---|---|---|---|
| **D1** | `explanation/architektur.md` §1 | `flowchart LR` | Welche Komponente kennt welche — und wo verläuft die Port-Grenze? | ADR-001; Komponententabelle §3; `engine-core/dependencies.md` | „Blockdiagramm: Clients sprechen HTTP mit engine-server; engine-server ruft engine-core; engine-core definiert den Port WorkflowPersistence; persistence-nats und persistence-memory implementieren ihn; keine Kante engine-core zu NATS." | `verify_mermaid` ohne Befund; Rendering belegt (`_qa/verification-report.md` §3) |
| **D2** | `explanation/architektur.md` §2.2 | `stateDiagram-v2` | In welchen Zuständen kann eine Process Instance sein und wodurch wechselt sie? | `NextAction`-Werte §2.2; `history_instances` §5.2; `Suspended { previous_state }` | „Zustandsdiagramm: Running wechselt in fünf Wait States und über die passende Auflösung zurück; Completed wechselt in den Bucket history_instances; Suspended merkt previous_state." | `verify_mermaid` ohne Befund; Rendering belegt |
| **D3** | `reference/service-task-api.md` §5 | `sequenceDiagram` | Warum garantiert Fetch-and-Lock nur At-least-once? | Lock-Ablauf §5; `409 ServiceTaskNotLocked`; Live-Lauf 2026-10-24 §2.3 | „Sequenzdiagramm: Worker A holt den Task mit Sperre; die Sperre läuft ab; Worker B holt denselben Task; Worker A erhält 409; Worker B schließt ab." | `verify_mermaid` ohne Befund; Rendering belegt |
| **D4** | `explanation/architektur.md` §4.4 | `sequenceDiagram` | In welcher Reihenfolge stellt die Engine beim Start Zustand wieder her — und warum ist `/api/ready` so lange 503? | Restore-Reihenfolge §4.4; `data-flows.md` §6/§7 | „Sequenzdiagramm: NATS-Verbindung, dann Restore in sechs Stufen mit XML-Fallback für Definitions, danach antwortet /api/ready mit 200; vorher 503." | `verify_mermaid` ohne Befund; Rendering belegt |
| **D5** | `explanation/konzepte.md` §6 | `flowchart TD` | Wann ist es eine Migration und wann eine Modification? | §5 Migration; §6 Modification; `rest-api.md` (422) | „Entscheidungsfluss: Versionswechsel? Nein führt zu Modification über move-token; Ja führt zur Frage nach dem Zielknoten; Nein führt zu 422 OrphanedToken; Ja führt zu Migration." | `verify_mermaid` ohne Befund; Rendering belegt |
| **D6** | `explanation/konzepte.md` §1 | `erDiagram` | Was hält den Instanzzustand — und warum ist ein Pending-Objekt ein Zeiger und keine Kopie? | `architecture.md` §2.1 (`tokens: HashMap<Uuid, Token>`, Pending hält nur `token_id`); `engine-core/interfaces.md`; `retries: 3` aus dem Live-Lauf | „Entity-Relationship-Diagramm: Eine Process Instance hält mehrere Tokens und mehrere Pending-Objekte; jedes Pending-Objekt verweist über token_id auf genau ein Token; die Position im Graph steht nur im Token." | `verify_mermaid` ohne Befund; **Rendering offen** |

**Summe:** sechs Diagramme in vier Seiten. Verteilung: `explanation/architektur.md` 3, `explanation/konzepte.md` 2, `reference/service-task-api.md` 1.

## 3. Einzelbegründungen

### D1 — Komponenten und Port-Grenze

- **Zweck:** Die Ports-and-Adapters-Form ist im Text als Entscheidung beschrieben. Der Satz, der regelmäßig überlesen wird, ist die *fehlende* Kante: `engine-core` kennt NATS nicht. Ein Diagramm kann eine Nicht-Beziehung zeigen, Fließtext nicht.
- **Leserichtung:** links (Clients) nach rechts (Broker) — die Richtung des Aufrufs.
- **Farbe nicht der einzige Träger:** Die Port-Grenze trägt dreifach — gestrichelte Linie, Knotenbeschriftung „Port: WorkflowPersistence", Kantenbeschriftung „implementiert von".
- **Beleg:** `docs/knowledge-base/architecture/decisions.md` (ADR-001), `docs/knowledge-base/architecture/components.md`, `docs/knowledge-base/modules/engine-core/dependencies.md` („engine-core does NOT depend on: async-nats").
- **Bewusste Auslassung:** keine Kante `bpmn-parser → engine-server`; der Parser wird von der Deploy-Route aufgerufen, ist aber keine Laufzeitabhängigkeit der Ausführung.

### D2 — Zustände einer Process Instance

- **Zweck:** `NextAction` ist im Text als „eigentlicher Motor" bezeichnet. Das Zustandsdiagramm macht aus der Enum-Beschreibung eine lesbare Menge von Wait States samt Rückwegen.
- **Leserichtung:** links nach rechts über die Wait States, Rückwege nach oben — Zustand links, Auflösung als Kante zurück.
- **Farbe nicht der einzige Träger:** Jede Kante trägt die auslösende Bedingung als Text („Task abgeschlossen", „suspend").
- **Beleg:** `explanation/architektur.md` §2.2, `InstanceState` aus `GET /api/instances/{id}` (`_qa/verification-report.md` §2.2 zeigt `{"WaitingOnServiceTask":{…}}`), Bucket-Wechsel nach `history_instances` aus §5.2.
- **Bewusste Auslassung:** `CompletedWithError` und der Incident-Zustand fehlen. Die Zuordnung „Incident ⇒ welcher `InstanceState`" ist in den Quellen nicht als Enum-Wert belegt; sie wird nicht geraten.

### D3 — Fetch-and-Lock und die At-least-once-Kante

- **Zweck:** Die Referenz nennt `lockDuration` als Zahl mit Bereichsgrenzen. Was sie nicht in einem Bild zeigt, ist die Folge: Der zweite `fetchAndLock` ist definiertes Verhalten, kein Fehler.
- **Leserichtung:** zeitlich von oben nach unten; zwei Worker als parallele Spuren.
- **Farbe nicht der einzige Träger:** Jede Nachricht trägt ihren Statuscode als Text; der Konflikt steht als `409 ServiceTaskNotLocked` an der Kante.
- **Beleg:** `reference/service-task-api.md` §5, `explanation/konzepte.md` §3, Live-Lauf 2026-10-24 (`complete` auf ungesperrtem Task → `409`, `_qa/verification-report.md` §2.3).
- **Bewusste Auslassung:** kein konkreter Timeout-Wert und kein Default-`retries` im Bild — beides steht in der Tabelle daneben und wäre im Diagramm eine zweite, driftende Quelle.

### D4 — Startup-Restore-Reihenfolge

- **Zweck:** Die Reihenfolge ist der Grund, warum ein Load Balancer eine noch nicht wiederhergestellte Engine nicht anspricht. Der Text nennt sechs Stufen; das Diagramm zeigt die Stufen **und** die Readiness-Kante.
- **Leserichtung:** zeitlich von oben nach unten; der Load Balancer kommt als eigener Teilnehmer erst am Ende ins Bild.
- **Farbe nicht der einzige Träger:** Der `alt`-Block für den XML-Fallback trägt seine Bedingung als Text.
- **Beleg:** `explanation/architektur.md` §4.4 (Reihenfolge wörtlich), `docs/knowledge-base/architecture/data-flows.md` §6/§7 (`/api/health` immer `200`, `/api/ready` `503` ohne Persistenz).
- **Bewusste Auslassung:** keine Dauerangaben — sie sind nicht belegt.

### D5 — Migration oder Modification

- **Zweck:** Zwei Operationen, die im Alltag verwechselt werden, hängen an einer einzigen Frage. Das Diagramm macht die Frage zur Verzweigung und die Ablehnung `422 OrphanedToken` zum sichtbaren Zweig.
- **Leserichtung:** von oben (Ausgangslage) nach unten (Ergebnis); die Rückkante ist als didaktische Schleife beschriftet („Modell oder Zielversion anpassen") und nicht als API-Aussage.
- **Farbe nicht der einzige Träger:** Die Zweige tragen „nein"/„ja" als Text; das Ergebnis trägt den Statuscode.
- **Beleg:** `explanation/konzepte.md` §5 und §6, `reference/rest-api.md` (Statuscodes), `reference/bpmn-elemente.md` (Deploy-Regeln).
- **Bewusste Auslassung:** keine Aussage über Teil-Migrationen — die Migration ist laut Quelle atomar, also gibt es keinen Zwischenzustand zu zeichnen.

### D6 — Datenmodell des Instanzzustands

- **Zweck:** Die wichtigste Invariante der Engine ist eine Nicht-Beziehung: Pending-Objekte halten **keine** Kopie des Tokens. Das ist der Punkt, an dem Einsteiger den Fehler an der falschen Stelle suchen (`explanation/konzepte.md` §1) und der einzige Fall im Bestand, in dem ein Datenmodell eine Verwechslung auflöst.
- **Leserichtung:** links Container (`ProcessInstance`), rechts Zustand (`TOKEN`) und Wartemarken (`PENDING_*`); die Kanten laufen von der Wartemarke zum Token, weil nur diese Richtung existiert.
- **Farbe nicht der einzige Träger:** Es gibt keine farbliche Kodierung. Bedeutung liegt in den Beziehungsgraden (`||--o{` = hält mehrere, `}o--||` = verweist auf genau eines) und in den Kantenbeschriftungen.
- **Beleg:** `explanation/architektur.md` §2.1 („Tokens liegen zentral in `ProcessInstance.tokens: HashMap<Uuid, Token>`"; Pending-Objekte „halten **nur** eine `token_id: Uuid`"), `docs/knowledge-base/modules/engine-core/interfaces.md`, `int retries "Startwert 3"` aus dem Live-Lauf (`_qa/verification-report.md` §2.3).
- **Bewusste Auslassung:** keine Attributliste von `Token` außer dem Schlüssel. Die Feldnamen des Tokens sind in den gelesenen Quellen nicht vollständig belegt (Audit AUD-021, `WorkflowPersistence`-Trait-Liste unvollständig); ein Diagramm hätte die Lücke als Vollständigkeit getarnt.

## 4. Verworfene Kandidaten

Diese Kandidaten wurden geprüft und **nicht** aufgenommen. Die Begründung ist Teil des Plans, damit sie nicht ohne neue Beleglage wieder aufgegriffen werden.

| Kandidat | Zielseite | Grund der Ablehnung |
|---|---|---|
| Drei Ergebniswege eines External Task als `flowchart` | `how-to/external-task-worker-schreiben.md` Schritte 5–7 | Die Seite führt die drei Wege als numerierte Schritte mit erwarteter Ausgabe und Statuscode; ein Diagramm würde die Liste nachzeichnen, ohne eine neue Beziehung zu zeigen. Regel 2. |
| Tutorial-Ablauf als `flowchart` | `tutorials/erster-prozess.md` | Ein Tutorial ist eine geordnete Prozedur; die Schrittnummerierung **ist** das Diagramm. |
| Konfigurationsreihenfolge als `flowchart` | `how-to/konfiguration-anpassen.md` | Die Abhängigkeit „Retention vor dem ersten Start" steht als Warnkasten und als Prüfschritt; ein Diagramm zeigt sie nicht präziser. |
| Verwechslungspaare als `classDiagram` | `_review/glossary.md` | Die Tabelle „Verwechslungsgefahr mit" leistet genau das — in Textform, durchsuchbar und ohne zweite Pflegestelle. |
| Incident-Lebenszyklus als `stateDiagram-v2` | `explanation/konzepte.md` §4 | Die Zustandsmenge (`offen`/`aufgelöst`) ist belegt, die Übergänge bei `retries > 0` gegenüber `retries = 0` sind es nicht. Ein Diagramm würde Semantik behaupten. Voraussetzung für eine Neuprüfung: Beleg aus `engine-core`-Gotchas oder ein Live-Lauf. |
| KV-Buckets als `erDiagram` | `reference/konfiguration.md` §9 | Die Buckets stehen dort als Tabelle. Zwischen ihnen besteht keine Beziehung — sie sind eine Liste, kein Modell. Ein `erDiagram` würde eine Struktur suggerieren, die es nicht gibt. |
| Entscheidungsbaum über REST-Endpunkte | `reference/rest-api.md` | Die Rollenmatrix und die Endpunkttabelle sind Nachschlagewerk; ein Baum wäre ein zweiter, schlechterer Index. |

## 5. Was dieser Plan nicht leistet

- **Kein Rendering-Nachweis für D6.** `verify_mermaid` prüft Grammatik und Diagrammtyp, nicht die Darstellung. Für D1–D5 liegt der Rendering-Nachweis aus `_qa/verification-report.md` §3 vor (Mermaid CLI 11.x); für D6 steht er aus und ist in `explanation/konzepte.md` unter **Offen** vermerkt.
- **Keine Barrierefreiheits-Prüfung im Zielrenderer.** Die Alternativtexte sind im Markdown hinterlegt und inhaltlich geprüft; ob der Renderer sie als `Alt`-Text ausgibt, ist eine Eigenschaft der veröffentlichenden Website.
- **Keine Nutzertests.** Ob die Diagramme die Verständlichkeit tatsächlich erhöhen, ist nicht gemessen. Der UX-Review (§7.3) beschreibt das Test-Design, in dem Diagrammnutzung mit erhoben werden sollte.

## 6. Pflege

| Änderung | Was zu tun ist |
|---|---|
| Neues Diagramm | Erst alle fünf Aufnahmeregeln prüfen; dann Zeile in §2 ergänzen, Einzelbegründung in §3, Alternativtext direkt beim Block. |
| Textänderung, die ein Diagramm berührt | Diagramm und Erklärungssatz im selben Commit anpassen; die Quelle der Wahrheit ist die Seite, nicht dieser Plan. |
| Diagramm entfernt | Zeile aus §2 löschen und in §4 mit Begründung verschieben — nicht stillschweigend löschen. |
| Quelle wird widerlegt | Diagramm entfernen, nicht „vorsichtig umformulieren". Ein Diagramm, das eine unbelegte Semantik braucht, ist das falsche Diagramm. |

## Offen

- **Rendering D6:** einmal mit dem Zielrenderer rendern und Ergebnis hier eintragen.
- **Anzahl der Diagramme in `_qa/content-manifest.md`:** Das Manifest nennt den alten Stand (fünf Diagramme, zehn Blöcke inklusive der Doppelungen). **Offen:** Manifest auf sechs Diagramme in Seiten und null Blöcke im Plan nachziehen.
- **`_qa/verification-report.md` §3:** Die Zählung „10 Blöcke / 10 SVG" bezieht sich auf den Stand mit den Doppelungen im Plan. Nach dem Entfernen sind es sechs Blöcke in Seiten. **Offen:** Zählung im Verifikationsbericht nachführen, ohne den Rendering-Nachweis selbst zu entwerten.
