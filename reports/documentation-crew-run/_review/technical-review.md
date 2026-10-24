---
title: Technischer Review — BPMNinja-Dokumentation (Runde 2, konsolidiert)
description: Technische Prüfung aller 13 veröffentlichten Seiten gegen Knowledge Base, Repository-Regeln und Live-Verifikationsbericht; zwei offene Blocker, fünf eingespielte Korrekturen, neun offene Nacharbeiten und eine Freigabeempfehlung.
audience:
  - Beitragende und Maintainer
  - Stakeholder und Entscheider
content_type: reference
status: reviewed
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _qa/content-manifest.md
  - _qa/verification-report.md
  - _qa/qa-report.md
  - _architecture/information-architecture.md
  - _audit/audit-report.md
---

# Technischer Review — BPMNinja-Dokumentation (Runde 2)

**Prüfgegenstand:** 27 Markdown-Dateien im Ausgabewurzel, davon **13 veröffentlichte Seiten** (Landingpage, 1 Tutorial, 2 How-tos, 7 Referenzseiten inkl. `reference/README.md`, 2 Explanation-Seiten).
**Stand:** 2026-10-24 · **Prüfer:** Documentation Lead (Redaktionsmanagement).
**Belegbasis:** `_qa/content-manifest.md`, `_qa/verification-report.md` (Live-Lauf, Status `reviewed`), `_architecture/information-architecture.md`, `_audit/audit-report.md`, Moduldokumentation unter `docs/knowledge-base/`, `bpmninja/README.md`, `docs/architecture.md`, `.agent/rules/NATS_MESSAGE_BROKER.md`.

## 0. Methode und Grenzen

Jede der 13 veröffentlichten Seiten wurde vollständig gelesen und Satz für Satz gegen die Belege geprüft: HTTP-Pfade, Statuscodes, Feldnamen, Konfigurationsvariablen mit Defaults und Klemmverhalten, Fehlerformat, Rollenlogik, Grenzwerte, Betriebsrisiken, Ausführbarkeit der Beispiele — zusätzlich **gegeneinander** (Statuscodes, Incident-Arten, Bucket-Namen, Verifikationsstatus).

**Was dieser Review nicht belegen kann:**

1. **Kein Zugriff auf den Quelltext.** Der Repository-Index umfasst 300 Dateien, aber keine `src/`-Dateien der Crates: `code_search` nach `message_catches`, `history_completed_instances` und `create_key_value` liefert **null Treffer**; `instance_files` ist nur in `.agent/rules/NATS_MESSAGE_BROKER.md` belegt. Aussagen, die ausschließlich mit `persistence-nats/src/client.rs` oder `parser.rs` belegt werden, sind **nicht nachprüfbar** (→ B-02).
2. **Kein laufender Server in diesem Durchlauf.** Der Live-Beleg stammt aus `_qa/verification-report.md`; ich habe ihn ausgewertet, nicht wiederholt.
3. **Anker werden nicht geprüft.** `check_links` prüft relative Datei- und Bildpfade, keine Fragmente. Die wiederholten Anker-Warnungen auf den Seiten bleiben gültig.

Jede Datei wurde höchstens einmal gelesen; nicht existierende Ziele wurden übersprungen und als Lücke dokumentiert (Abschnitt 6).

## 1. Befundübersicht

| Kategorie | Anzahl | Blocker |
|---|---:|---:|
| Offene Blocker (B) | 2 | 2 |
| Eingespielte Korrekturen (K) | 5 | – |
| Offene Nacharbeiten hoch (W) | 2 | – |
| Offene Nacharbeiten mittel/niedrig (W) | 7 | – |
| **Summe** | **16** | **2** |

**Kernaussage.** Die Dokumentation ist gegenüber der Vorrunde deutlich gereift: alle sieben Referenzseiten existieren, keine Seite ist abgeschnitten, der Live-Lauf ist als `reviewed` belegt und die zuvor gemeldeten API-Fehler im Worker-Beispiel sind behoben. Der verbliebene Schaden liegt an **zwei Stellen**: einem How-to, dessen Schritte 3–7 ohne veröffentlichtes npm-Paket nicht ausführbar sind (B-01), und einer Betriebsliste persistierter Strukturnamen, die drei Quellen widersprüchlich beschreiben und die ich nicht gegen den Quelltext prüfen kann (B-02). Fünf technische Fehler sind korrigiert (Abschnitt 3), zwei Statusaussagen zum Verifikationsstand sind falsch (W-01, W-02).

## 2. Blocker

### B-01 — Das How-to ist ohne veröffentlichtes Client-Paket nicht ausführbar

| Feld | Inhalt |
|---|---|
| **ID** | B-01 |
| **Datei** | `how-to/external-task-worker-schreiben.md`, Schritt 3–7 und §Offen |
| **Schweregrad** | **Blocker** (für diese Seite) |
| **Problem** | Der Handlungsteil ab Schritt 3 hängt an `npm install @bpmninja/external-task-client`. Existenz und Export-Signatur dieses Pakets sind nicht belegt, obwohl die Seite die Schritte 5–7 als „belegt\" formuliert. |
| **Beleg** | `_qa/verification-report.md` §5: „Den TypeScript-Client `@bpmninja/external-task-client` — kein veröffentlichtes Paket installiert; der How-to-Code folgt der Knowledge-Base-API.\" `reference/cli.md` §6 nennt als Pfad `bpmn-ninja-external-task-client` — also ein **Repository-Paket**, nicht notwendig ein Registry-Paket. |
| **Warum Blocker** | Ein Leser kopiert den Install-Befehl und erhält im schlechteren Fall ein fremdes oder kein Paket. Der Guide ist die einzige Anleitung für den wichtigsten Integrationspfad. |
| **Korrekturvorschlag** | Schritt 3 auf den belegten Weg umstellen: `cd bpmn-ninja-external-task-client && npm install && npm run build`, danach `npm install ../bpmn-ninja-external-task-client` im Worker-Projekt — **oder** einen Registry-Nachweis (`npm view … version`) erbringen. Bis dahin Warnbox: „Der Client wird aus dem Repository gebaut; ein Registry-Paket ist nicht belegt.\" |
| **Status** | **offen — Blocker** |

### B-02 — Persistierte Strukturnamen: drei Quellen, drei Listen

| Feld | Inhalt |
|---|---|
| **ID** | B-02 |
| **Dateien** | `reference/konfiguration.md` §9, `explanation/architektur.md` §5.1/§5.2, `reference/kompatibilitaet.md` §8 |
| **Schweregrad** | **Blocker** (Betriebsvertrag) |
| **Problem** | Veröffentlicht sind **acht** KV-Buckets (`definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `messages`, `tokens`, `history_instances`) und **zwei** Object Stores (`bpmn_xml`, `instance_files`). `reference/konfiguration.md` §9 nennt das „Vertrag für Betriebsprüfungen\" und versichert, alles sei „gegen `persistence-nats/src/client.rs` geprüft\". Die zugänglichen Quellen widersprechen in drei Punkten. |
| **Beleg** | (a) `persistence/gotchas.md` §„KV bucket names cannot change\" nennt neun Buckets, darunter **`message_catches`**, **`bpmn_xml`** (als KV) und **`history`**. (b) Dieselbe Datei §„Completed instances are archived\" nennt das Archiv **`history_completed_instances`**. (c) `.agent/rules/NATS_MESSAGE_BROKER.md` §2 nennt als KV **`messages`** und als Object Stores `bpmn_xml` und `instance_files`. Zwei Quellen stützen damit `messages` und die Object-Store-Rolle von `bpmn_xml`; **keine** Quelle außer der Seite selbst stützt `history_instances`. |
| **Warum Blocker** | Ein Betriebsteam, das Buckets für Backup, Migration oder Fehlersuche inspiziert, arbeitet mit einem Namen, der falsch sein kann — und die Seite behauptet dafür Quelltextnähe. Ein falscher Bucket-Name ist still: Die Abfrage liefert „leer\", nicht „Fehler\". |
| **Korrekturvorschlag** | Ein Grep im Adapter (`rg -n 'create_key_value\|create_object_store' persistence-nats/src/`) entscheidet die drei offenen Punkte. Bis dahin Kasten in `reference/konfiguration.md` §9 und `explanation/architektur.md` §5.1: „`messages` und die Object-Store-Rolle von `bpmn_xml` sind durch die Repository-Regel gedeckt; `history_instances` ist nur über `client.rs` belegt — die Knowledge Base nennt `history` bzw. `history_completed_instances`. Vor Betriebsnutzung quellseitig bestätigen.\" |
| **Status** | **offen — Blocker** (nicht korrigiert: Raten ist untersagt, der Quelltext ist im Index nicht erreichbar) |

## 3. Eingespielte Korrekturen

Alle fünf sind per `doc_write` umgesetzt; die Front-Matter-Daten der Seiten blieben unverändert.

| ID | Datei / Stelle | Schweregrad | Problem (vorher) | Beleg | Korrektur (nachher) | Status |
|---|---|---|---|---|---|---|
| **K-01** | `reference/service-task-api.md` §2 | **hoch** | Tabelle „Lock-Dauer — kanonisch\" führte `newDuration` mit Bereich `1_000..=3_600_000` und „Überschreitung: **400**\" — im Widerspruch zu §4 derselben Seite, zu `rest-api.md` und zum Live-Lauf | `_qa/verification-report.md` §2.3 („`newDuration: 30` → `204` — keine 400-Validierung\"); `engine-server/interfaces.md` §HTTP lock duration (Validierung nur für `fetchAndLock`) | `newDuration`: „**wird nicht validiert**\", „keine Ablehnung; `engine-core` klemmt die umgerechneten Sekunden auf `1..=3600`\"; Satz ergänzt: „Die Grenzen gelten ausschließlich für `fetchAndLock`\" | **behoben** |
| **K-02** | `reference/service-task-api.md` §Offen | mittel | §3 dokumentiert `retries: 3`, §Offen behauptete „Default-`retries` … **nicht belegt**\" | `_qa/verification-report.md` §2.3 | Punkt lautet jetzt: frischer Task startet mit `retries: 3`, nach `retry` wieder `3`; „belegt, aber nicht als Konfigurationsvariable einstellbar\" | **behoben** |
| **K-03** | `how-to/external-task-worker-schreiben.md` §Wichtige Eigenschaften, Fehler 3 | **hoch** | Punkt behauptete, ein ungefangener `bpmnError` erzeuge einen Incident der Art `bpmn_error` — derselbe Guide nennt in „Fehler 3\" `service_task` | `_qa/verification-report.md` §2.3 (`kind: \"service_task\"`, `message: \"Unhandled BPMN error '…'\"`) | Punkt nennt `service_task` samt Meldung und Beleg; „Fehler 3\" präzisiert: Abgleich gegen `errorCode` des referenzierten `<bpmn:error>` (vorher `errorRef`); Hinweis ergänzt, dass `GET /api/service-tasks` Admin-only ist (`403` mit Worker-Key) | **behoben** |
| **K-04** | `explanation/konzepte.md` §4 | mittel | „Die vier Arten\" erklärte `bpmn_error` mit dem ungefangenen `bpmnError` — nach Live-Beleg `service_task`. Damit hätten zwei veröffentlichte Seiten widersprechend geantwortet | `_qa/verification-report.md` §2.3; `engine-server/interfaces.md` §Incident-API (Enum kennt `bpmn_error`) | `service_task` trägt beide Auslöser; `bpmn_error` bleibt als Enum-Wert genannt, mit Hinweis „auslösender Fall nicht belegt\" plus **Offen**-Eintrag. Ergänzt: `resolve` löst den Incident nicht automatisch auf | **behoben** |
| **K-05** | `reference/cli.md` §3 | mittel | „Alle Variablen …: `how-to/konfiguration-anpassen.md` (Ersatz für die noch fehlende Konfigurationsreferenz)\" — die Referenz existiert, und der eigene **Offen**-Abschnitt verweist korrekt dorthin | `doc_list` (`reference/konfiguration.md`, 1 619 Wörter) | Verweis zeigt auf `reference/konfiguration.md`, How-to bleibt als aufgabenbezogener Einstieg; `related` ergänzt | **behoben** |

## 4. Offene Befunde

### 4.1 Hoch

| ID | Datei / Stelle | Problem | Beleg | Korrigierte Fassung | Status |
|---|---|---|---|---|---|
| **W-01** | `README.md` §„Status dieser Dokumentation\" | „… aber **noch nicht gegen einen Live-Lauf und gegen `docs/openapi.yaml` abgeglichen**.\" Das ist nach dem Live-Lauf falsch und widerspricht fünf Seiten, die „Live-Lauf 2026-10-24\" zitieren | `_qa/verification-report.md` (Status `reviewed`); `reference/rest-api.md`; `tutorials/erster-prozess.md`; `reference/kompatibilitaet.md` | „Der Stand ist **draft**: inhaltlich vollständig. REST-Beispiele und Mermaid-Diagramme sind am 2026-10-24 gegen eine laufende Engine geprüft (`_qa/verification-report.md`). **Nicht** abgeglichen: `docs/openapi.yaml` (bekannte Abweichung bei `bpmnError`), das npm-Paket des Clients, Last-/Timeout-Pfade.\" | offen |
| **W-02** | `tutorials/erster-prozess.md` §Offen, Punkte 1–2 | (a) „Ein Abgleich der Beispielantworten gegen einen Live-Lauf … steht aus\" — er ist erfolgt. (b) „**XML- und JSON-Bodies** werden akzeptiert\" — **falsch**: Rohes XML wird mit `415` abgelehnt, so steht es im Fließtext von Schritt 4 derselben Seite | `_qa/verification-report.md` §2.1 („Rohes XML (`application/xml`) → `415`\"); Tutorial Schritt 4 | (a) „Die Beispielantworten sind am 2026-10-24 gegen eine laufende Engine geprüft.\" (b) „`/api/deploy` akzeptiert ausschließlich `application/json` mit `name` und `xml`; ein roher XML-Body wird mit `415` abgelehnt.\" | offen |

### 4.2 Mittel und niedrig

| ID | Datei / Stelle | Grad | Problem | Beleg | Korrigierte Fassung | Status |
|---|---|---|---|---|---|---|
| **W-03** | `reference/rest-api.md` Service-Task-Tabelle; `reference/README.md` Schnellverweis | mittel | Beide nennen `403` als möglichen Status von `fetchAndLock`. `403` ist laut Rollenmatrix „Worker-Key auf **Admin**-Route\"; `fetchAndLock` ist für Worker zugelassen. Nach K-01/§3 der kanonischen Seite widersprechen sich die Seiten | `engine-server/interfaces.md` §Rollenmatrix; `reference/service-task-api.md` §3 | `403` entfernen oder als „nicht erreichbar\" kennzeichnen | offen |
| **W-04** | `reference/rest-api.md` §Offen, Punkt 2 | mittel | Ein **gelöster** Punkt steht als „Offen\": „Akzeptierte Medientypen für `/api/deploy` **(belegt)**\" — die Seite wirkt dadurch unfertiger als sie ist | `_qa/verification-report.md` §2.1 | Punkt in den Fließtext von „Definitions\" verschieben (dort steht die Aussage bereits); unter **Offen** nur die Restfrage nach der vollständigen Liste | offen |
| **W-05** | `reference/kompatibilitaet.md` §2, Zeile „Lock-Dauer\" | mittel | „Millisekunden, gültig 1 000 bis 3 600 000\" ohne Differenzierung. Für `extendLock` gilt die Grenze nicht (K-01) | `reference/service-task-api.md` §2; `_qa/verification-report.md` §2.3 | „`fetchAndLock`: 1 000–3 600 000 ms, intern `(ms + 999) / 1000`. `extendLock`: keine Validierung, Klemmung 1–3600 s.\" | offen |
| **W-06** | `reference/bpmn-elemente.md` §1.2, `ErrorEndEvent` | mittel | „Ungefangen entsteht ein Incident `bpmn_error`.\" Der Enum-Wert existiert, der Auslöser ist nicht belegt; die Aussage steht nicht unter **Offen** | `engine-server/interfaces.md` §Incident-API; `_qa/verification-report.md` §2.3 (nur Service-Task-Fall belegt) | Aussage nach **Offen** verschieben: „Ob ein ungefangenes Error End Event `bpmn_error` erzeugt, ist nicht belegt.\" | offen |
| **W-07** | `README.md` §Metriken, Zeile „Linkintegrität\" | niedrig | Zeile nennt „135 von 139 (97 %) — 4 Treffer\". `check_links` misst **202 relative Links, 2 defekt**; Bezugsgröße wechselt (Anker vs. Dateilinks) | `check_links` (2026-10-24); `_qa/content-manifest.md` §5 („57 Anker, 0 Defekte\") | „2 von 202 relativen Dateilinks defekt (beide in `_review/style-guide.md`, nicht veröffentlicht); Anker nicht geprüft.\" | offen |
| **W-08** | `_review/style-guide.md` Zeilen 346–347 | niedrig | Zwei Bildverweise auf `ablauf.svg` laufen ins Leere | `check_links` | Pfad korrigieren oder auf den Mermaid-Block verweisen | offen |
| **W-09** | `README.md` Zielgruppentabelle; `_qa/content-manifest.md` §5 | niedrig | Zählungen noch „10 Seiten mit `## Offen`\" / „57 Anker\"; die Zielgruppe „Prozessmodellierer\" verweist nicht auf `reference/bpmn-elemente.md` und `reference/kompatibilitaet.md`, die genau ihre Fragen beantworten | `doc_list` (13 veröffentlichte Seiten) | Zählungen auf 13 umstellen; Modellierer-Einstieg auf `reference/bpmn-elemente.md` setzen, `kompatibilitaet.md` als Folgeseite | offen |

## 5. Geprüft und bestätigt

| Bereich | Bestätigte Aussage | Beleg |
|---|---|---|
| Deploy | `POST /api/deploy` nur `application/json` mit `name` und `xml`; roher XML-Body → `415`; Antwort `{definition_key, version}` | `_qa/verification-report.md` §2.1; `reference/rest-api.md`; Tutorial Schritt 4 |
| Start | `POST /api/start` liefert **nur** `instance_id`; Zustand über `GET /api/instances/{id}` (`{\"WaitingOnUserTask\":{\"task_id\":…}}`) | `_qa/verification-report.md` §2.2; `reference/rest-api.md`; Tutorial Schritt 5 |
| User Task | `POST /api/complete/{id}` → **`204`**, leerer Body | `_qa/verification-report.md` §2.2; `reference/rest-api.md` |
| `fetchAndLock` | `topics` 1..=32, `lockDuration` 1 000..=3 600 000 ms, `maxTasks` 1..=100 → sonst `400` vor dem Long-Poll; `lockDuration: 30` → `400` | `engine-server/interfaces.md` §HTTP lock duration; `_qa/verification-report.md` §2.3 |
| Service-Task-Ergebnisse | `complete`/`failure`/`bpmnError`/`extendLock`/`retry`/**`resolve`** → `204`; ungesperrt → `409` („is not locked\"); `failure(...,0)` → Incident `service_task`, Instanz bleibt `WaitingOnServiceTask` | `_qa/verification-report.md` §2.3; `reference/service-task-api.md` |
| Body-Limit | 10 MiB → `413` mit `Failed to buffer the request body: length limit exceeded` | `_qa/verification-report.md` §2.4 |
| Auth | fail-closed ohne Key bei Nicht-Loopback-Bind → `401`; Worker auf Admin-Route → `403`; Health/Ready öffentlich | `engine-server/interfaces.md` §Auth-Modi; `reference/konfiguration.md` §2 |
| Defaults | `PORT=8081`, `BIND_ADDR=0.0.0.0`, `REQUIRE_NATS=true`, `MAX_XML_BYTES=10485760`, `MAX_UPLOAD_BYTES=5242880`, `PERSISTENCE_RETRY_QUEUE_CAPACITY=10000`, `TIMER_INTERVAL_MS=1000`, `RHAI_MAX_OPERATIONS=50000`, Retention `7`/`30` Tage, `BPMNINJA_STREAM_REPLICAS=1` | `bpmninja/README.md` Env-Tabelle; `persistence/gotchas.md` §R7; `engine-server/dependencies.md` |
| Diagramme | 10 Mermaid-Blöcke, 10/10 als SVG gerendert | `_qa/verification-report.md` §3 |
| Client-API | `TaskHandler = (task, service) => Promise<void>`; `Task.variables_snapshot`; `complete(variables?)`, `failure(msg, details?, retries?)`, `extendLock(ms)`, `bpmnError(code)`; `maxRetries` 3 global pro Client; `lockDuration` 30 000 ms; `autoExtendLock` bei 80 % | `external-task-client/interfaces.md`; `external-task-client/gotchas.md` |

## 6. Nicht prüfbar in diesem Lauf

1. **Bucket- und Stream-Namen** gegen den Adapter (B-02) — Quelltext nicht im Index.
2. **Parser-Aussagen**, die nur mit `bpmn-parser/src/parser.rs` belegt sind: `errorRef`-Lookup, Ablehnungsfälle, Multi-Instance-Grenzen, Cron-Timer. Sie sind in sich konsistent und deckungsgleich mit `bpmn-parser/gotchas.md`, aber nicht quellseitig bestätigt.
3. **TypeScript-Client** (B-01) — kein Paket, kein Lauf.
4. **Last-, Timeout- und Fehlerpfade** jenseits der im Verifikationsbericht genannten Statuscodes.
5. **`docs/openapi.yaml`** — vier bekannte Abweichungen (u. a. `bpmnError`: OpenAPI `400`, Engine `204`); der CI-Abgleich (Audit AUD-006/018/019) fehlt weiterhin.

## 7. Freigabeempfehlung

**Keine Freigabe.** Alle 13 Seiten bleiben auf `status: draft`. Zwei Blocker sind offen, beide sind in einem Arbeitstag mechanisch schließbar — aber nicht durch Textänderung allein:

| Blocker | Schließung durch | Aufwand |
|---|---|---|
| **B-01** Client-Paket | Installationsweg auf Repository-Build umstellen **oder** Registry-Nachweis; danach Schritt 5–7 einmal ausführen | ~1 h |
| **B-02** Strukturnamen | ein Grep im Adapter, drei Namen bestätigen, betroffene Absätze nachziehen | ~30 min |

**Pflicht vor Veröffentlichung (keine Blocker, aber sichtbare Falschaussagen):** W-01 und W-02. Beide Seiten behaupten, es habe keinen Live-Abgleich gegeben — das untergräbt den `reviewed`-Status des Verifikationsberichts und ist für Leser eine falsche Statusauskunft.

**Freigabefähig nach Schließung von B-01/B-02 und W-01/W-02:** `reference/service-task-api.md` (nach K-01/K-02), `reference/konfiguration.md` (nach B-02), `reference/cli.md` (nach K-05), `reference/rest-api.md` (nach W-03/W-04), `reference/bpmn-elemente.md` (nach W-06), `reference/kompatibilitaet.md` (nach W-05), `explanation/architektur.md` (nach B-02), `explanation/konzepte.md` (nach K-04), `tutorials/erster-prozess.md` (nach W-02), `how-to/konfiguration-anpassen.md` (ohne Auflage), `how-to/external-task-worker-schreiben.md` (nach B-01), `README.md`/`reference/README.md` (nach W-01/W-03/W-07/W-09).

**Empfohlene Reihenfolge:** B-02 (Vertrag) → B-01 (Ausführbarkeit) → W-01/W-02 (Statuswahrheit) → W-03 bis W-09 (Konsistenz) → erneuter Live-Lauf nur für das geänderte How-to → Freigabe durch den Documentation Lead.
