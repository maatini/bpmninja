# Revisionsbericht — Nachbesserungsrunde 1 (QA-Score 75 → Ziel 90)

**Dokument:** `_qa/revision-round-1.md`
**Owner:** Documentation Lead
**Stand:** 2026-10-24
**Entscheidung dieser Runde:** **Nachbessern — keine Freigabe.** Vier Blocker sind inhaltlich geschlossen (B-01, B-03, B-04, B-07), einer teilweise (B-09), vier bleiben offen, weil der Beleg fehlt (B-05, B-06, B-08, B-10). Ich erteile keine Freigabe, solange ein Blocker ohne dokumentierte Risikoübernahme offen ist.

---

## 1. Management Summary

| Blocker | Status nach Runde 1 | Beleglage |
|---|---|---|
| B-01 `reference/konfiguration.md` 4-Wort-Fragment | **behoben** | Datei vollständig neu geschrieben: 9 Front-Matter-Felder, 10 Abschnitte, Zustandsmatrix, Migrationswarnung, Offen |
| B-03 `service-task-api.md` bpmnError-curl abgebrochen + defekter Anker + fehlender 409 | **behoben** | Pfad `/api/service-task/{id}/bpmnError` vollständig; Anker `../explanation/konzepte.md#…` korrigiert; `409`-Zeile mit Beleg `engine-server/interfaces.md:40` ergänzt |
| B-04 `rest-api.md` letzter Absatz abgebrochen | **behoben** | Abschnitt **Offen** vollständig; Fehlerantworten-Tabelle (9 Status) + Abgrenzung; Pagination |
| B-05 `bpmn-elemente.md` und `kompatibilitaet.md` fehlen (P0) | **nicht behoben — begründet** | Element-Mapping nicht vollständig belegbar; als P0-Lücke im Manifest geführt, nicht als „geplant" getarnt |
| B-06 kein Beispiel gegen Engine/OpenAPI verifiziert | **nicht behoben — begründet** | Keine laufende Engine, kein `docs/openapi.yaml`-Abgleich in dieser Sitzung; alle betroffenen Seiten tragen jetzt einen expliziten Verifikationsvermerk |
| B-07 Worker-Beispiel falsche Client-API | **behoben** | Handler `(task, service)`, `task.variables_snapshot`, `complete({ gesendet: true })` — gegen `external-task-client/interfaces.md` belegt |
| B-08 `errorRef`-Semantik unbelegt | **nicht behoben — begründet** | `errorRef` kommt in keiner der 49 Quelldateien vor; Beispiel auf belegten Pfad umgestellt, Offen-Hinweis bleibt |
| B-09 Mermaid nur syntaktisch geprüft | **teilweise behoben** | `verify_mermaid` in dieser Runde gelaufen: 5 Diagramme, keine Syntaxbefunde. Rendering im Zielsystem offen |
| B-10 KV-Bucket-Katalog widersprüchlich | **nicht behoben — begründet, aber entschärft** | Beide Quellen zitieren unterschiedliche Namen; `explanation/architektur.md` folgt jetzt der näheren Quelle (Persistence-Modul) und nennt die Abweichung explizit |

**Kernaussage:** Die Vollständigkeits- und Nachweisblocker (B-01, B-03, B-04, B-07) sind geschlossen. Die verbleibenden (B-05, B-06, B-08, B-10) sind **Beleglücken, keine Schreibfehler**. Ich schließe sie nicht durch Formulierungen, sondern dokumentiere sie sichtbar auf der jeweiligen Seite und im Manifest. Das ist die einzige redaktionell vertretbare Entscheidung: Eine Seite, die eine API-Semantik behauptet, ohne sie belegen zu können, ist schlechter als eine Seite, die die Lücke benennt.

---

## 2. Blocker im Einzelnen: Ursache, Korrektur, Beleg

### B-01 — `reference/konfiguration.md` war ein 4-Wort-Fragment

**Ursache:** Der Redaktionslauf brach nach dem Front-Matter-Feld `title` ab. Es existierte kein Body, kein `description`, `audience`, `content_type`, `status`, `last_reviewed`, `owner`, `related`. Sechs Seiten verwiesen für Default-Werte auf diese Seite — alle sechs Verweise liefen ins Leere.

**Korrektur:** Die Seite wurde vollständig geschrieben. Sie enthält jetzt:

- vollständiges Front Matter mit allen neun Pflichtfeldern,
- zehn Abschnitte: Netzwerk/Bindung, Persistenz/Durability, Authentifizierung, CORS, Body-/Upload-Limits, Timer/Scheduler, Logging, Rhai-Sandbox, JetStream-Retention, nicht konfigurierbare Konstanten,
- eine Zustandsmatrix für das fail-closed-Auth-Gate (Keys × Bind-Adresse),
- eine Migrationswarnung zur Stream-Retention (Datenverlust bei niedrigeren Limits beim ersten Start),
- einen Abschnitt **Offen** mit fünf nicht belegten Punkten (KV-Bucket-Präfix `bpm_*` vs. Kurznamen, `BPMNINJA_STREAM_REPLICAS=0`-Warnung, Collection-Cap-Formel, `MAX_XML_BYTES`-Doppelrolle, Vollständigkeit gegen `main.rs`).

**Beleg:** `bpmninja/README.md` Env-Tabelle (Zeilen 271–278); `docs/knowledge-base/cross-cutting/shared-patterns.md` §„Environment-Driven Configuration" (Zeilen 83–94); `docs/knowledge-base/modules/engine-server/interfaces.md` §„Auth & CORS env" (Zeilen 176–185); `docs/knowledge-base/modules/persistence/gotchas.md` §„JetStream streams have explicit retention and replication" (Zeilen 46–61); `docs/knowledge-base/modules/engine-core/interfaces.md` (Rhai, Zeile 137); `docs/knowledge-base/modules/engine-server/gotchas.md` (Body-Limit, Zeile 34; Log-Buffer, Zeile 75).

**Abhängige Verweise nachgezogen:** `reference/README.md`, `reference/rest-api.md`, `reference/service-task-api.md`, `how-to/konfiguration-anpassen.md` und `explanation/architektur.md` verweisen jetzt auf eine existierende Seite. Der frühere Satz in `rest-api.md` und `service-task-api.md` („`reference/konfiguration.md` enthält nur den Front-Matter-Kopf und ist deshalb nicht verlinkt") ist entfernt — er wäre nach der Korrektur falsch.

---

### B-03 — `reference/service-task-api.md`: `bpmnError`-Beispiel brach ab, Anker defekt, 409 fehlte

**Ursache:** Drei getrennte Fehler in einer Datei:

1. Der Schreibvorgang endete bei `curl -s -X POST http://localhost:8081/api/service-task/st-0001/bpmn`. Der Pfad war unvollständig, und die Abschnitte `extendLock`, `retry`, `resolve`, Lock-Ablauf und Idempotenz fehlten vollständig — obwohl die Seite laut `description` genau diese Abschnitte verspricht.
2. Der Anker in §5 lautete `..//explanation/konzepte.md#…` (doppelter Schrägstrich).
3. Die `bpmnError`-Statusliste führte keinen `409`, obwohl `ServiceTaskNotLocked` laut `engine-server/interfaces.md:40` auf `409` abgebildet wird.

**Korrektur:**

- Der `bpmnError`-Aufruf lautet jetzt vollständig `POST /api/service-task/st-0001/bpmnError` mit Body `{"errorCode":"INVALID_ADDRESS"}` und Status-Tabelle (`200`, `401`, `404`, `409`).
- Die `409`-Zeile ist mit Beleg ergänzt: `ServiceTaskNotLocked` → `409` („Complete called on unlocked task", `engine-server/interfaces.md:40`). Zusätzlich steht unter **Offen**, dass `409` für `bpmnError` aus der `complete`-Abbildung abgeleitet ist und nicht separat spezifiziert wurde.
- Ergänzt: `extendLock` mit `newDuration` in ms und Status-Tabelle (`200`, `400`, `401`, `404`, `409`).
- Ergänzt: `retry` und `resolve` als Admin-only mit explizitem `403` für Worker-Keys.
- Ergänzt: §5 „Lock-Ablauf und Idempotenz" mit der At-least-once-Konsequenz und einer Gegenmaßnahmen-Tabelle (`lockDuration`, `autoExtendLock`, idempotenter Handler).
- Anker korrigiert auf `../explanation/konzepte.md#3-service-task-und-external-task-element-und-arbeitseinheit`.
- Ergänzt: Abschnitt **Offen** mit `errorRef`-Semantik, Default-`retries`, fehlender `fehlercodes.md`, Versionsstand und Ankerauflösung.

**Beleg:** `docs/knowledge-base/modules/engine-server/interfaces.md` §„HTTP lock duration", §„Rollenmatrix", Zeile 40 (`ServiceTaskNotLocked` → 409); `docs/knowledge-base/modules/external-task-client/gotchas.md`; `docs/knowledge-base/cross-cutting/shared-patterns.md` §„Camunda-Compatible Service Task API".

**Ursachenbehebung statt Symptom:** Die Seite ist die kanonische Lock-Dauer-Quelle (Entscheidung E7). Deshalb steht die Einheit **einmal** in §2 und wird in `rest-api.md`, `cli.md` und im How-to nur referenziert, nicht wiederholt. Damit kann die Einheit nicht erneut auseinanderlaufen.

---

### B-04 — `reference/rest-api.md`: letzter Absatz endete mitten im Satz

**Ursache:** Der Abschnitt **Offen** brach bei „Die Ereignisnamen sind belegt (`" ab. Der Satz zum SSE-Ereignisformat war unvollständig, und die Fehlerantworten-Tabelle war unvollständig.

**Korrektur:**

- Der Abschnitt **Offen** ist vollständig: OpenAPI-Abgleich, akzeptierte Medientypen für `/api/deploy`, Statuscodes für `POST /api/message` / `POST /api/timers/process` / `PUT /api/instances/{id}/variables`, SSE-Payload-Schema, fehlende `fehlercodes.md`/`metriken.md`, Versionsstand, Ankerauflösung.
- Die Fehlerantworten-Tabelle nennt jetzt neun Status (`400`, `401`, `403`, `404`, `409`, `413`, `415`, `422`, `500`) mit Bedeutung und typischem Auslöser.
- Ergänzt: Absatz „Abgrenzung der vier Ablehnungsstatus" (`400` Syntax/Validierung, `413` Größe, `415` Medientyp, `422` formal gültig aber inhaltlich unpassend).
- Ergänzt: Pagination-Hinweis für `GET /api/instances` (`limit` 1..=1000, `X-Total-Count`, Sortierung nach `started_at` absteigend).
- Präzisiert: Body-Limit-Zeile nennt jetzt „JSON-/XML-Body-Limit" statt nur „JSON-Body-Limit" (Beleg `engine-server/interfaces.md:185`: „JSON/XML body + deploy cap").

**Beleg:** `docs/knowledge-base/modules/engine-server/gotchas.md` §„Alle Fehlerantworten sind `{ "error": ... }`", §„SSE is broadcast, NOT queue"; `docs/knowledge-base/modules/engine-core/gotchas.md` (Pagination); `docs/knowledge-base/modules/engine-server/interfaces.md:185` (`MAX_XML_BYTES`).

---

### B-05 — `reference/bpmn-elemente.md` und `reference/kompatibilitaet.md` fehlen (P0)

**Ursache:** Beide Seiten sind im Dateiplan als P0 geführt und werden von `explanation/architektur.md` §2.3/§7 sowie `explanation/konzepte.md` „Offen" referenziert. Sie wurden nie geschrieben.

**Warum ich sie in dieser Runde nicht schreibe:** Eine P0-Referenzseite „XML-Tag → `BpmnElement`-Variante" braucht eine vollständige, geprüfte Elementliste. Die belegbare Grundlage ist `docs/knowledge-base/modules/bpmn-parser/interfaces.md` (Element-Mapping-Tabelle) und `bpmninja/README.md` (29 Elemente, Nicht-unterstützt-Liste). Beides zusammen deckt die Abbildung **nicht** vollständig ab: Für mehrere Elemente (u. a. `receiveTask`, `manualTask`, `businessRuleTask`, Multi-Instance, DMN) nennt das README nur „nicht unterstützt oder still ignoriert", ohne die genaue Parser-Reaktion. Eine Tabelle „Element | BPMN 2.0 | Camunda 7 | BPMNinja | Begründung | Migrationshinweis" würde an diesen Stellen raten.

**Was stattdessen geschieht:**

- Beide Seiten bleiben im Manifest als **P0-Lücke** geführt, nicht als „geplant (P0)" mit dem Anschein, sie kämen in dieser Runde.
- `reference/README.md` verlinkt sie **nicht** mehr als auflösbare Seiten, sondern nennt sie in einem eigenen Absatz „Noch nicht geschrieben (P0)" mit der Begründung.
- `explanation/architektur.md` §2.3 und `explanation/konzepte.md` verweisen auf die Lücke statt auf eine tote Seite.

**Was ich vom Team brauche, um sie zu schreiben:** Eine geprüfte Element-Mapping-Tabelle aus `bpmn-parser` (Parser-Quelle oder ein Test, der jedes Element deployt) sowie eine Entscheidung, ob „still ignoriert" pro Element dokumentiert wird oder als Klassenaussage. Ohne diese zwei Belege schreibe ich die Seiten nicht.

---

### B-06 — Kein Beispiel gegen laufende Engine oder `docs/openapi.yaml` verifiziert

**Ursache:** In dieser Sitzung standen keine laufende Engine, kein `docs/openapi.yaml`-Abgleich und kein Ausführungswerkzeug für die Beispiele zur Verfügung. Betroffen: `tutorials/erster-prozess.md`, `how-to/external-task-worker-schreiben.md`, `how-to/konfiguration-anpassen.md`, `reference/rest-api.md`, `reference/service-task-api.md`.

**Korrektur (dokumentierte Lücke statt Scheinbeleg):**

- Jede der fünf Seiten trägt im Abschnitt **Offen** einen Satz der Form: „Die Beispiele sind gegen die Knowledge Base belegt, aber nicht gegen eine laufende Engine ausgeführt; der Abgleich gegen `docs/openapi.yaml` steht aus (Audit AUD-006, AUD-018, AUD-019)."
- `reference/rest-api.md` nennt `docs/openapi.yaml` weiterhin als **einzige** Quelle der Wahrheit und behauptet keine Schemavollständigkeit.
- `how-to/external-task-worker-schreiben.md` trägt den Hinweis, dass das npm-Paket `@bpmninja/external-task-client` nicht gegen eine veröffentlichte Version ausgeführt wurde.

**Was ich nicht tue:** Ich entferne keine Beispiele, um den Score zu heben, und ich markiere sie nicht als „verifiziert". Beides wäre eine Falschaussage.

**Eskalation:** Das ist ein **Freigabe-Blocker für die Veröffentlichung**, nicht für den Redaktionsstand. Er lässt sich nur durch einen CI-Job schließen, der die curl-Beispiele gegen eine Compose-Instanz und die Tabellen gegen `docs/openapi.yaml` prüft. Ich empfehle, diesen Job vor der nächsten Runde einzurichten.

---

### B-07 — Worker-Beispiel nutzte die falsche Client-API

**Ursache:** Der Code in `how-to/external-task-worker-schreiben.md`, Schritt 4, verwendete drei nicht existierende APIs:

| Falsch (vorher) | Richtig (jetzt) | Beleg |
|---|---|---|
| `async ({ task, service }) => …` | `async (task, service) => …` | `docs/knowledge-base/modules/external-task-client/interfaces.md` (Handler-Signatur) |
| `task.variables` | `task.variables_snapshot` | `external-task-client/interfaces.md:40`; `bpmninja/README.md:424`, `:439`; `docs/architecture.md:212` |
| `service.complete({variables:{...}})` | `service.complete({ gesendet: true })` | `external-task-client/interfaces.md` (`complete(variables?)`) |

**Korrektur:** Der Code in Schritt 4 ist vollständig auf die belegte API umgestellt. Zusätzlich erklärt der Abschnitt „Wichtige Eigenschaften dieses Codes" jetzt **warum** die alte Form falsch war: `async ({ task, service })` destrukturiert zwei `undefined`-Werte und bricht beim ersten Zugriff mit `TypeError` ab; `task.variables` existiert nicht und liefert immer `undefined`, sodass der Worker jede Adresse als ungültig behandelt, ohne dass ein Fehler sichtbar wird.

**Ursachenbehebung statt Symptom:** Die Korrektur ist nicht auf Schritt 4 beschränkt. Abhängige Stellen wurden mitgezogen:

- Schritt 5 (Erfolgspfad) erwartet jetzt `gesendet: true` in der Instanzantwort — konsistent mit `complete({ gesendet: true })`.
- Schritt 6 (technischer Fehler) nutzt `service.failure(err.message, String(err.stack ?? ""), 0)` und erklärt, dass der dritte Parameter die **Engine**-Retries setzt, nicht die Client-Retries.
- Schritt 7 (fachlicher Fehler) nutzt `service.bpmnError("INVALID_ADDRESS")` und beschreibt den Boundary-Pfad.
- Die Abnahmekriterien-Tabelle am Ende prüft alle vier Wege in einem Durchlauf.

---

### B-08 — `errorRef`-Semantik unbelegt

**Ursache:** Schritt 1 des How-tos verwendet `errorRef="INVALID_ADDRESS"` als direkten String. Ob `errorRef` auf ein `<bpmn:error>`-Element verweisen muss, ist nicht belegt.

**Belegprüfung in dieser Runde:** `errorRef` kommt in **keiner** der 49 Quelldateien vor (`source_search` über `**/*.md`, keine Treffer). Belegt ist nur:

- `docs/knowledge-base/modules/bpmn-parser/interfaces.md:77`: `boundaryEvent > errorEventDefinition` → `BoundaryErrorEvent { attached_to, error_code }`
- `docs/architecture.md:247`: `BoundaryErrorEvent { attached_to, error_code: Option<String> }`
- `bpmninja/README.md:133`: „Catches BPMN errors (`errorCode`) of a ServiceTask and routes onto an alternative path."

**Korrektur:** Das Beispiel bleibt beim direkten String, weil das der einzige belegbare Pfad ist — aber es wird **nicht** als gesichert dargestellt. Der How-to trägt an der Stelle eine „Offen — `errorRef`-Semantik"-Box mit dem Fallback (explizites `<bpmn:error id="INVALID_ADDRESS" errorCode="INVALID_ADDRESS" />`), und der Punkt steht zusätzlich im Abschnitt **Offen** der Seite und in `reference/service-task-api.md`.

**Was ich nicht tue:** Ich erfinde kein `<bpmn:error>`-Element als „richtig", nur weil es in BPMN 2.0 üblich ist. Ohne Parser-Beleg bleibt es offen.

---

### B-09 — Mermaid-Diagramme nur syntaktisch geprüft

**Ursache:** `_architecture/diagram-plan.md` gibt fünf Diagramme (D1–D5) frei, hat sie aber nur manuell gegen die Mermaid-Grammatik geprüft. Ein Rendering im Zielsystem ist nicht nachgewiesen.

**Korrektur:** `verify_mermaid` ist in dieser Runde gelaufen: **5 Diagramme, keine Syntaxbefunde.** Der Diagrammplan trägt jetzt oben und in §6 den Status **„Syntax geprüft (`verify_mermaid`, 2026-10-24); Rendering im Zielsystem offen"**. Die Übersichtstabelle führt für D1–D5 die Spalte „Prüfung" mit „`verify_mermaid`: keine Befunde; Rendering offen". Die Freigabeentscheidung in §6 lautet ausdrücklich „Nachbessern, nicht freigeben" mit zwei benannten Abnahmekriterien.

**Was ich nicht tue:** Ich stelle die Diagramme nicht als „vollständig verifiziert" dar und streiche sie nicht, nur weil das Rendering-Werkzeug fehlt. Die Aussagen der Diagramme sind belegt; offen ist ausschließlich das Rendering.

---

### B-10 — Widersprüchlicher KV-Bucket-Katalog

**Ursache:** Zwei veröffentlichte Seiten nennen unterschiedliche Bucket-Namen und zitieren beide eine Quelle als Beleg:

| Seite | Namen | Zitierte Quelle |
|---|---|---|
| `explanation/architektur.md` §5.1 | `bpm_definitions`, `bpm_instances`, `bpm_user_tasks`, `bpm_service_tasks`, `bpm_timers`, `bpm_msg_catches`, `bpm_tokens`, `bpm_bpmn_xml`, `bpm_history` | `docs/architecture.md:418–426` |
| `persistence/gotchas.md:76` | `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history` | „hardcoded in `trait_impl.rs`" |

**Quellenlage (in dieser Runde geprüft):**

- `source_search` „bpm_definitions" → **ein** Treffer: `docs/architecture.md:418`.
- `source_search` „bpm_" → **neun** Treffer, alle in `docs/architecture.md:418–426`.
- `source_search` „hardcoded in" → **ein** Treffer: `persistence/gotchas.md:76` mit der Aussage „NATS KV bucket names are hardcoded in `trait_impl.rs`. … The current buckets: `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history`."
- `source_search` „message_catches" → bestätigt die Kurznamen zusätzlich in `persistence/responsibility.md:14` und `persistence/dependencies.md:63`.
- `source_search` „history_completed_instances" → **ein** Treffer: `persistence/gotchas.md:88` (Archivierungs-Absatz, nicht in der Bucket-Liste).

**Bewertung:** Die Persistence-Moduldoku ist die **nähere** Quelle (sie beschreibt die Implementierung in `trait_impl.rs` und wird von zwei weiteren Persistence-Seiten gestützt), `docs/architecture.md` ist die **entferntere** (Architekturübersicht). Der Code selbst ist in dieser Sitzung **nicht lesbar** — `trait_impl.rs` liegt nicht im Suchbereich.

**Korrektur (Entschärfung, nicht Schließung):** `explanation/architektur.md` §5.1 folgt jetzt der Persistence-Moduldoku (Kurznamen) und nennt `docs/architecture.md` §5.2 ausdrücklich als **abweichende, ältere Angabe** mit dem Hinweis, dass die verbindliche Liste vor Statuswechsel auf `reviewed` gegen `persistence-nats` abgeglichen werden muss (Audit AUD-022). `history_completed_instances` ist als **zehnter** Bucket im Archivierungs-Absatz benannt, mit dem Hinweis, dass er in keiner der beiden Listen erscheint.

**Was ich nicht tue:** Ich erkläre den Widerspruch nicht für aufgelöst. Ohne Code-Lesezugriff bleibt er offen; ich habe nur verhindert, dass zwei Seiten sich widersprechen, ohne es zu sagen.

---

## 3. Hochpriorisierte Nicht-Blocker (bearbeitet)

| Befund | Korrektur | Datei |
|---|---|---|
| `reference/README.md` verlinkte fünf nicht existierende Seiten als „geplant" | Eigener Absatz „Noch nicht geschrieben (P0/P1)" mit Begründung; keine tote Verlinkung mehr | `reference/README.md` |
| `reference/rest-api.md` verwies für Auth auf `reference/konfiguration.md` (leer) | Verweis auf `how-to/konfiguration-anpassen.md`, Schritt 3 | `reference/rest-api.md` |
| `reference/service-task-api.md` verwies auf nicht existierende `reference/fehlercodes.md` | Verweis entfernt, Lücke im Abschnitt **Offen** benannt | `reference/service-task-api.md` |
| `reference/service-task-api.md` hatte defekten Anker `..//explanation/konzepte.md#…` (doppelter Slash) | Pfad korrigiert | `reference/service-task-api.md` |
| `reference/rest-api.md` behauptete „JSON-Body-Limit 10 MiB" ohne XML-Abgrenzung | Auf „JSON- und XML-Bodies" präzisiert, Belegstelle ergänzt | `reference/rest-api.md` |
| `reference/cli.md` wiederholte widersprüchliche Testzahlen | Zahlen bewusst nicht als Zusage wiederholt, Widerspruch (AUD-008) im Abschnitt **Offen** benannt | `reference/cli.md` |
| `reference/README.md` nannte keine Endpunktzahl, aber auch keinen Grund | Begründung (sieben widersprüchliche Angaben, AUD-006) steht jetzt als Blockquote | `reference/README.md` |

---

## 4. Nicht behobene Punkte und der Grund

| Punkt | Grund | Nächster Schritt | Verantwortlich |
|---|---|---|---|
| **B-05** `bpmn-elemente.md`, `kompatibilitaet.md` | Element-Mapping nicht vollständig belegbar; „still ignoriert" pro Element unbelegt | Geprüfte Mapping-Tabelle aus `bpmn-parser` (Quelle oder Deploy-Test je Element) | Technical Writer + Parser-Owner |
| **B-06** Beispielverifikation | Keine laufende Engine, kein OpenAPI-Abgleich in dieser Sitzung | CI-Job: curl-Beispiele gegen Compose-Instanz, Tabellen gegen `docs/openapi.yaml` | Plattform/CI |
| **B-08** `errorRef`-Semantik | `errorRef` in keiner der 49 Quelldateien belegt | Parser-Quelle lesen oder Deploy-Test mit/ohne `<bpmn:error>` | Parser-Owner |
| **B-09** Mermaid-Rendering | `verify_mermaid` gelaufen (keine Syntaxbefunde); Rendering im Zielsystem nicht verfügbar | Rendering in bpmn-js/GitHub/IDE prüfen, Ergebnis im Diagrammplan eintragen | Documentation Lead |
| **B-10** KV-Bucket-Präfix `bpm_*` vs. Kurznamen | Widerspruch zwischen `docs/architecture.md` §5.2 und Persistence-Moduldoku; Code nicht lesbar | `trait_impl.rs` prüfen | Persistence-Owner |
| **AUD-021** `WorkflowPersistence`-Trait-Liste unvollständig | Kein Code-Lesezugriff in dieser Sitzung | Trait-Definition lesen | Persistence-Owner |
| **AUD-016/017** `async-nats`-Abhängigkeit, `persistence-memory` als direkte Abhängigkeit | Kein Code-Lesezugriff | `Cargo.toml` prüfen | Architecture-Owner |
| **AUD-008** Test- und LoC-Zahlen widersprüchlich | Kein Zugriff auf CI-Artefakte | CI-Report als Quelle | CI |

---

## 5. Geänderte Dateien

| Datei | Art der Änderung | Blocker |
|---|---|---|
| `reference/konfiguration.md` | **vollständig neu geschrieben** (Front Matter + 10 Abschnitte + Offen) | B-01 |
| `reference/service-task-api.md` | `bpmnError`-Pfad vollständig; `409`-Zeile mit Beleg; `extendLock`, `retry`, `resolve`, §5 Lock-Ablauf/Idempotenz, Offen ergänzt; defekter Anker korrigiert; `fehlercodes.md`-Verweis entfernt | B-03, B-08 |
| `reference/rest-api.md` | Abschnitt **Offen** vollständig; Fehlerantworten-Tabelle (9 Status) + Abgrenzung; Pagination; Auth-Verweis auf How-to; Body-Limit präzisiert | B-04, B-06 |
| `how-to/external-task-worker-schreiben.md` | Schritt 4 auf belegte Client-API umgestellt; abhängige Schritte 5–7 und Abnahmekriterien konsistent; `errorRef`-Offen-Box | B-07, B-08 |
| `reference/README.md` | Abschnitt „Noch nicht geschrieben (P0/P1)" statt toter Verlinkung; Endpunktzahl-Begründung | B-05 |
| `explanation/architektur.md` | §5.1 folgt der Persistence-Moduldoku (Kurznamen); `docs/architecture.md` als abweichende Angabe benannt; `history_completed_instances` als zehnter Bucket eingeordnet | B-10 |
| `_architecture/diagram-plan.md` | Status „Syntax geprüft (`verify_mermaid`, 2026-10-24); Rendering offen"; Prüfspalte je Diagramm | B-09 |
| `_qa/content-manifest.md` | `konfiguration.md` von „Fragment" auf „draft" umgestellt; P0-Lücken `bpmn-elemente.md`/`kompatibilitaet.md` als Lücke geführt; Verifikationsvermerke je Seite | B-01, B-05, B-06 |
| `_qa/revision-round-1.md` | **neu** — dieser Bericht | — |

**Nicht geändert:** `tutorials/erster-prozess.md`, `how-to/konfiguration-anpassen.md`, `explanation/konzepte.md`, `reference/cli.md` — sie tragen bereits korrekte Offen-Vermerke; eine Änderung hätte nur den Score, nicht die Beleglage verbessert.

---

## 6. Prüfungen nach jeder Änderung

| Prüfung | Ergebnis |
|---|---|
| **Interne Links (`check_links`)** | 4 defekte von 170 vor der Runde; nach der Runde **0 defekte** in veröffentlichten Seiten. Die zwei verbleibenden Verweise auf `bpmn-elemente.md`/`kompatibilitaet.md` sind als Text, nicht als Link formuliert. |
| **Aussagen gegen Quellen** | Jede geänderte Aussage trägt eine Belegstelle (`bpmninja/README.md`, `docs/knowledge-base/modules/*/interfaces.md`, `gotchas.md`, `shared-patterns.md`). Nicht belegbare Aussagen stehen unter **Offen**. |
| **Front Matter** | Alle neun Pflichtfelder in `reference/konfiguration.md` vorhanden; alle anderen geänderten Seiten unverändert vollständig. |
| **Terminologie** | „Service Task" (BPMN-Element) vs. „External Task" (Arbeitseinheit) durchgängig getrennt; `lockDuration`/`newDuration` ausschließlich in ms; `bpmn_process_id` mit `bpmn_id` als Alias. |
| **Diátaxis-Zuordnung** | `konfiguration.md` bleibt **reference** (Verträge, Defaults, Grenzwerte); die Anleitung steht in `how-to/konfiguration-anpassen.md` und wird von dort verlinkt. Keine Vermischung. |
| **Content-Manifest** | Aktualisiert: `konfiguration.md` jetzt „draft" mit Wortzahl; P0-Lücken explizit; Verifikationsvermerke je Seite. |

---

## 7. Verbleibende Risiken

1. **Veröffentlichungsrisiko durch B-06 (hoch).** Fünf Seiten enthalten Beispiele, die nicht gegen eine laufende Engine ausgeführt wurden. Ein falscher Statuscode oder ein falsches Feld würde Nutzer in die Irre führen. Das ist das größte verbleibende Risiko und lässt sich nur durch CI schließen.
2. **P0-Lücke B-05 (hoch).** Ohne `bpmn-elemente.md` und `kompatibilitaet.md` fehlt Modellierern die konsolidierte Abweichungsreferenz. Nutzer, die von Camunda kommen, treffen falsche Annahmen über Sub-Prozesse, Multi-Instance und DMN.
3. **`errorRef`-Semantik B-08 (mittel).** Das How-to-Beispiel könnte bei einem Deploy mit `InvalidDefinition` abbrechen. Der Fallback ist dokumentiert, aber nicht verifiziert.
4. **Mermaid-Rendering B-09 (mittel).** Fünf Diagramme könnten im Zielsystem fehlerhaft rendern. Die Aussagen sind belegt; nur die Darstellung ist offen.
5. **KV-Bucket-Namenskonflikt B-10 (mittel).** `reference/konfiguration.md` und `explanation/architektur.md` folgen der Persistence-Moduldoku (Kurznamen), `docs/architecture.md` nennt `bpm_*`. Ein Betriebsteam, das Buckets direkt inspiziert, könnte den falschen Namen verwenden.
6. **Versionsstand (niedrig).** Keine Seite nennt „gültig ab". Bei einem Minor-Release ist unklar, welche Aussage noch gilt.

---

## 8. Freigabeentscheidung

**Status: „Nachbessern" — keine Freigabe.**

Ich erteile die Freigabe **nicht**, weil drei Abnahmekriterien nicht belegt sind:

1. **Beispiele sind nicht ausgeführt** (B-06). Ein Beispiel, das nicht läuft, ist keine Dokumentation, sondern eine Behauptung.
2. **Zwei P0-Seiten fehlen** (B-05). Die Abweichungsreferenz ist der
