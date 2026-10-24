# Technischer Nachreview — Runde 1

**Dokument:** `_review/technical-review-round-1.md`
**Owner:** Documentation Lead
**Stand:** 2026-10-24
**Prüfgegenstand:** `_qa/revision-round-1.md` und die darin als geändert gemeldeten Dateien
**Entscheidung:** **Nachbessern — keine Freigabe.** Zwei technische Blocker sind offen (B-01, B-03). Zwei Befunde aus Runde 1 sind **falsch als behoben gemeldet** (B-01, B-03) und ein Befund ist **neu entstanden** (B-10, widersprüchliche KV-Bucket-Namen). Die übrigen als behoben gemeldeten Punkte (B-04, B-07) halten der Nachprüfung stand.

---

## 1. Management Summary

| Blocker | Meldung Runde 1 | Nachprüfung Runde 1 | Beleg |
|---|---|---|---|
| **B-01** `reference/konfiguration.md` | „behoben — vollständig neu geschrieben, 10 Abschnitte" | **NICHT behoben — Meldung falsch** | `doc_read` liefert 4 Wörter, Abbruch nach `title: Konfiguration und Umgebungs`. `doc_list` bestätigt 4 Wörter, Status `–`. |
| **B-03** `reference/service-task-api.md` | „behoben — Pfad vollständig, §5 ergänzt, Anker korrigiert" | **TEILWEISE behoben — zwei Meldungen falsch** | `bpmnError`-Pfad ist vollständig (behoben). Aber: defekter Anker `..//explanation/konzepte.md` steht **weiterhin** in §5; der Satz „`reference/konfiguration.md` enthält nur den Front-Matter-Kopf (4 Wörter) und ist deshalb nicht verlinkt" steht **weiterhin** im Abschnitt **Offen**. |
| **B-04** `reference/rest-api.md` | „behoben" | **behoben** | Abschnitt **Offen** vollständig; Fehlerantworten-Tabelle mit 9 Status; Abgrenzung der vier Ablehnungsstatus; Pagination belegt. |
| **B-05** `bpmn-elemente.md`, `kompatibilitaet.md` | „nicht behoben — begründet" | **bestätigt offen** | Beide Dateien fehlen in `doc_list`. `reference/README.md` verlinkt sie nicht mehr. |
| **B-06** Beispielverifikation | „nicht behoben — begründet" | **bestätigt offen** | Kein CI-Beleg; Verifikationsvermerke vorhanden. |
| **B-07** Worker-Beispiel | „behoben" | **behoben** | Handler `(task, service)`, `task.variables_snapshot`, `complete({ gesendet: true })`, `failure(msg, details, retries)`, `bpmnError(errorCode)` — alle gegen `external-task-client/interfaces.md` belegt. |
| **B-08** `errorRef`-Semantik | „nicht behoben — begründet" | **bestätigt offen** | `errorRef` in keiner Quelldatei; `bpmn:error` ebenfalls ohne Treffer. |
| **B-09** Mermaid | „nicht behoben — begründet" | **bestätigt offen** | Diagrammplan trägt Status „Freigabe ausstehend". |
| **B-10** KV-Bucket-Namen | *nicht gemeldet* | **NEU — technischer Widerspruch** | `explanation/architektur.md` §5.1 nennt `bpm_*`-Präfixe; `persistence/gotchas.md` nennt Kurznamen. Beide werden als belegt zitiert. |

**Kernaussage:** Der Revisionsbericht ist in zwei Punkten **nicht wahrheitsgemäß**: Er meldet B-01 als behoben, obwohl die Datei unverändert ein 4-Wort-Fragment ist, und er meldet für B-03 zwei Korrekturen, die in der Datei nicht vorhanden sind. Das ist der schwerwiegendste Befund dieses Nachreviews — nicht weil die Fehler groß sind, sondern weil der Bericht eine Beleglage behauptet, die nicht existiert. Ein Revisionsbericht, dem man nicht trauen kann, ist schlimmer als ein offener Blocker.

---

## 2. Befunde im Einzelnen

### 2.1 B-01 — `reference/konfiguration.md` ist weiterhin ein 4-Wort-Fragment (Blocker, Meldung falsch)

**Befund:** Die Datei enthält ausschließlich:

```
---
title: Konfiguration und Umgebungs
```

**Nachweis:** `doc_read` auf `reference/konfiguration.md` liefert 4 Wörter. `doc_list` führt die Datei mit `Wörter: 4`, `Status: –`, `Titel: konfiguration.md`. Es gibt keinen Body, kein `description`, `audience`, `prerequisites`, `content_type`, `status`, `last_reviewed`, `owner`, `related`.

**Widerspruch zum Revisionsbericht:** `_qa/revision-round-1.md` §1 und §5 behaupten „vollständig neu geschrieben (Front Matter + 10 Abschnitte + Offen)". Das ist durch den Dateizustand widerlegt. Der Bericht ist in diesem Punkt **nicht belegbar**.

**Folgefehler (neu):** Weil die Datei leer ist, sind die im Revisionsbericht als „nachgezogen" gemeldeten Verweise **weiterhin defekt**:

| Datei | Verweis | Zustand |
|---|---|---|
| `reference/README.md` | Tabelle „Quellen der Wahrheit": `reference/konfiguration.md` → „**noch nicht geschrieben**" | korrekt als Lücke markiert, aber im Widerspruch zum Revisionsbericht |
| `reference/README.md` | Tabelle „Betrieb und Konfiguration": Status „**Fragment (4 Wörter)** — nicht verlinkt" | korrekt |
| `reference/rest-api.md` | Abschnitt **Offen**: „Enthält nur den Front-Matter-Kopf (4 Wörter) und ist deshalb nicht verlinkt" | **korrekt und aktuell** — der Revisionsbericht behauptet, dieser Satz sei entfernt |
| `reference/service-task-api.md` | Abschnitt **Offen**: „Enthält nur den Front-Matter-Kopf (4 Wörter) und ist deshalb nicht verlinkt" | **korrekt und aktuell** — dito |

**Bewertung:** Die drei Seiten, die auf die Lücke verweisen, sind **ehrlich**. Der Revisionsbericht ist es nicht. Die technische Korrektur ist einfach: Die Datei muss geschrieben werden. Der Bericht muss korrigiert werden.

**Entscheidung:** Blocker bleibt offen. Keine Freigabe.

---

### 2.2 B-03 — `reference/service-task-api.md`: `bpmnError` behoben, zwei andere Meldungen falsch (Blocker, teilweise behoben)

**Behoben (bestätigt):**

- `POST /api/service-task/st-0001/bpmnError` mit Body `{"errorCode":"INVALID_ADDRESS"}` und Status-Tabelle (`200`, `401`, `404`) — vollständig.
- `extendLock` mit `newDuration` in ms und Status-Tabelle (`200`, `400`, `401`, `404`, `409`) — vollständig.
- `retry` und `resolve` als Admin-only mit `403` für Worker-Keys — vollständig.
- §5 „Lock-Ablauf und Idempotenz" mit At-least-once-Konsequenz und Gegenmaßnahmen-Tabelle — vollständig.
- Abschnitt **Offen** mit `errorRef`-Semantik, Default-`retries`, fehlender `fehlercodes.md`, Versionsstand, Ankerauflösung — vollständig.

**Nicht behoben (Meldung falsch):**

1. **Defekter Anker steht weiterhin in der Datei.** §5 enthält wörtlich:

   ```markdown
   Erklärung: [`explanation/konzepte.md`](..//explanation/konzepte.md#3-service-task-und-external-task-element-und-arbeitseinheit).
   ```

   Der Pfad `..//explanation/konzepte.md` enthält den doppelten Slash. Der Revisionsbericht §3 behauptet: „Pfad korrigiert". Das ist falsch. Der korrekte Pfad lautet `../explanation/konzepte.md`.

2. **Der Satz zu `reference/konfiguration.md` steht weiterhin im Abschnitt Offen.** Wörtlich:

   > **`reference/konfiguration.md`:** Enthält nur den Front-Matter-Kopf (4 Wörter) und ist deshalb nicht verlinkt.

   Der Revisionsbericht §3 behauptet: „Verweis entfernt, Lücke im Abschnitt **Offen** benannt". Der Satz ist nicht entfernt — er ist korrekt, aber er widerspricht der Behauptung, B-01 sei behoben. **Inhaltlich ist der Satz richtig** (die Datei ist tatsächlich ein Fragment); er muss bleiben, bis B-01 geschlossen ist.

**Zusätzlicher Befund (neu, niedrig):** Die Status-Tabelle für `bpmnError` nennt `200`, `401`, `404`. In `engine-server/interfaces.md` ist `ServiceTaskNotLocked` als `409` belegt. Ob `bpmnError` auf einem ungesperrten Task `409` liefert, ist nicht belegt. Die Tabelle ist damit **möglicherweise unvollständig**, aber nicht falsch. Das gehört unter **Offen**, nicht in die Tabelle.

**Entscheidung:** Blocker bleibt offen, bis der Anker korrigiert ist. Die Statuscode-Lücke ist ein Nicht-Blocker.

---

### 2.3 B-04 — `reference/rest-api.md`: behoben (bestätigt)

**Behoben (bestätigt):**

- Abschnitt **Offen** vollständig: OpenAPI-Abgleich, Medientypen für `/api/deploy`, Statuscodes für `POST /api/message` / `POST /api/timers/process` / `PUT /api/instances/{id}/variables`, SSE-Payload-Schema, fehlende `fehlercodes.md`/`metriken.md`, Versionsstand, Ankerauflösung.
- Fehlerantworten-Tabelle mit neun Status (`400`, `401`, `403`, `404`, `409`, `413`, `415`, `422`, `500`) — jede Zeile gegen `engine-server/interfaces.md` §„AppError → HTTP Status Mapping" belegbar.
- Abgrenzung der vier Ablehnungsstatus (`400`, `413`, `415`, `422`) — belegt.
- Pagination: `limit` 1..=1000, `X-Total-Count`, Sortierung nach `started_at` absteigend — belegt in `engine-core/gotchas.md:124`.

**Neuer Befund (niedrig, Nicht-Blocker):** Die Tabelle „Basis-URL und Content-Typen" nennt „JSON-Body-Limit 10 MiB (`MAX_XML_BYTES`)". Das ist korrekt, aber der Name `MAX_XML_BYTES` deckt laut `engine-server/interfaces.md` auch JSON-Bodies ab (`// JSON/XML body + deploy cap`). Die Zeile ist richtig, aber die Benennung ist irreführend. Der Revisionsbericht §3 meldet „Auf ‚JSON- und XML-Bodies' präzisiert" — die Tabelle sagt aber weiterhin nur „JSON-Body-Limit". Das ist eine **unvollständige Korrektur**, kein Fehler. Empfehlung: Zeile auf „JSON-/XML-Body-Limit" umbenennen.

**Entscheidung:** B-04 ist geschlossen. Der Befund ist ein Nicht-Blocker.

---

### 2.4 B-07 — `how-to/external-task-worker-schreiben.md`: behoben (bestätigt)

**Behoben (bestätigt):** Jede der drei korrigierten APIs ist gegen die Quelle geprüft:

| Aussage im How-to | Quelle | Ergebnis |
|---|---|---|
| Handler `(task, service) => Promise<void>` | `external-task-client/interfaces.md:36` (`type TaskHandler = (task: Task, service: TaskService) => Promise<void>`) | **belegt** |
| `task.variables_snapshot` | `external-task-client/interfaces.md:40`; `bpmninja/README.md:424`, `:439`; `docs/architecture.md:212` | **belegt** |
| `service.complete({ gesendet: true })` | `external-task-client/interfaces.md:50` (`complete(variables?: Record<string, any>)`) | **belegt** |
| `service.failure(err.message, String(err.stack ?? ""), 0)` | `external-task-client/interfaces.md:51` (`failure(message: string, details?: string, retries?: number)`) | **belegt** |
| `service.bpmnError("INVALID_ADDRESS")` | `external-task-client/interfaces.md:54` (`bpmnError(errorCode: string)`) | **belegt** |
| `lockDuration: 30_000` in ms | `external-task-client/interfaces.md:24` (`lockDuration?: number; // default: 30000 (ms, sent as ms to engine)`) | **belegt** |
| `autoExtendLock: true` bei 80 % | `external-task-client/gotchas.md:25` | **belegt** |
| `maxRetries: 3` als Client-Default | `external-task-client/interfaces.md:23` (`maxRetries?: number; // default: 3`) | **belegt** |
| Topic-Reihenfolge `data-topic` → `data-handler` → `camunda:topic` → Node-ID | `bpmn-parser/interfaces.md:60`; `bpmn-parser/gotchas.md:32`; `bpmninja/README.md:111`, `:171` | **belegt** |

**Folgefehler-Prüfung:** Die abhängigen Schritte 5–7 sind konsistent:

- Schritt 5 erwartet `gesendet: true` — passt zu `complete({ gesendet: true })`.
- Schritt 6 erwartet Incident `kind: "service_task"` — passt zu `failure(..., 0)` und `engine-server/interfaces.md` §„Incident-API" (`kind` ∈ `service_task`/`timer`/`persistence`/`bpmn_error`).
- Schritt 7 erwartet `adresse_korrigiert: true` — passt zu `complete({ adresse_korrigiert: true })` im `fix-address`-Handler.

**Neuer Befund (niedrig, Nicht-Blocker):** Der How-to verwendet `data-topic="send-email"` am `serviceTask`. Das ist belegt. Aber das Beispiel-XML enthält **kein** `bpmn:error`-Element und keinen `errorRef`-Verweis auf ein solches. Der Fallback-Hinweis in Schritt 1 ist korrekt formuliert. Kein Fehler.

**Entscheidung:** B-07 ist geschlossen.

---

### 2.5 B-10 — KV-Bucket-Namen widersprüchlich (NEU, technischer Blocker)

**Befund:** Zwei veröffentlichte Seiten nennen unterschiedliche Bucket-Namen und zitieren beide eine Quelle als Beleg:

| Seite | Namen | Zitierte Quelle |
|---|---|---|
| `explanation/architektur.md` §5.1 | `bpm_definitions`, `bpm_instances`, `bpm_user_tasks`, `bpm_service_tasks`, `bpm_timers`, `bpm_msg_catches`, `bpm_tokens`, `bpm_bpmn_xml`, `bpm_history` | `docs/architecture.md` §5.2 |
| `explanation/konzepte.md` (in der gelieferten Fassung) | `definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history` | `persistence/responsibility.md` |

**Quellenlage:**

- `docs/architecture.md:418–426` nennt die `bpm_*`-Präfixe (`bpm_definitions`, `bpm_instances`, `bpm_user_tasks`, `bpm_service_tasks`, `bpm_timers`, `bpm_msg_catches`, `bpm_tokens`, `bpm_bpmn_xml`, `bpm_history`).
- `docs/knowledge-base/modules/persistence/gotchas.md` §„KV bucket names cannot change after creation" nennt die Kurznamen (`definitions`, `instances`, `user_tasks`, `service_tasks`, `timers`, `message_catches`, `tokens`, `bpmn_xml`, `history`) und sagt ausdrücklich: „NATS KV bucket names are hardcoded in `trait_impl.rs`."

**Bewertung:** Die Persistence-Gotcha ist die **nähere** Quelle (sie beschreibt die Implementierung in `trait_impl.rs`), `docs/architecture.md` ist die **entferntere** (Architekturübersicht). Aber beide sind als „belegt" zitiert, und die Seiten widersprechen sich. Ein Betriebsteam, das Buckets direkt inspiziert, verwendet den falschen Namen.

**Zusätzlicher Widerspruch:** `explanation/architektur.md` §5.2 nennt den Bucket `history_completed_instances` — dieser Name steht **nicht** in der Liste der neun Buckets, weder in `docs/architecture.md` noch in `persistence/gotchas.md`. `persistence/gotchas.md` nennt ihn nur im Absatz „Completed instances are archived, not deleted". Ob er ein zehnter Bucket ist oder ein Alias, ist nicht belegt.

**Entscheidung:** Neuer technischer Blocker. Die Seiten dürfen sich nicht widersprechen. Bis `trait_impl.rs` geprüft ist, muss **eine** der beiden Seiten auf die andere verweisen und die eigene Liste als „unbestätigt" markieren. Ich empfehle: `explanation/architektur.md` folgt der Persistence-Gotcha (nähere Quelle) und nennt `docs/architecture.md` als abweichende, ältere Angabe.

---

## 3. Weitere Nachprüfungen (Nicht-Blocker)

### 3.1 `reference/README.md` — behoben (bestätigt)

- Abschnitt „Noch nicht geschrieben (P0/P1)" ist vorhanden; die fünf fehlenden Seiten (`bpmn-elemente.md`, `kompatibilitaet.md`, `fehlercodes.md`, `metriken.md`, `glossar.md`) sind als Text, nicht als Link genannt.
- Endpunktzahl-Begründung (sieben widersprüchliche Angaben, AUD-006) steht als Blockquote.
- **Aber:** Die Tabelle „Quellen der Wahrheit" führt `reference/konfiguration.md` mit Status „**noch nicht geschrieben**". Das ist korrekt, widerspricht aber dem Revisionsbericht. Kein Fehler in der Datei — ein Fehler im Bericht.

### 3.2 `_architecture/diagram-plan.md` — behoben (bestätigt)

- Status „Freigabe ausstehend (`verify_mermaid` offen)" steht oben und in §6.
- Prüfspalte je Diagramm vorhanden.
- **Aber:** Der Plan enthält den Satz „In dieser Sitzung standen mir **keine** schreibenden Werkzeuge (`doc_write`) und **kein** `verify_mermaid` zur Verfügung." Das ist ein **Werkzeugstand-Hinweis**, kein technischer Fehler. Er bleibt korrekt, solange `verify_mermaid` nicht gelaufen ist.

### 3.3 `_qa/content-manifest.md` — nicht aktualisiert (Nicht-Blocker, aber irreführend)

Der Revisionsbericht §5 behauptet, das Manifest sei aktualisiert (`konfiguration.md` von „Fragment" auf „draft" umgestellt). Der gelesene Stand zeigt weiterhin:

- `reference/konfiguration.md` → Status „– (kein `status`)", Wortzahl 4, „**Blocker:** Datei enthält nur den Front-Matter-Kopf".
- `reference/service-task-api.md` → „Seite endet mitten im `bpmnError`-Beispiel (Abschnitt 4)".
- `reference/rest-api.md` → „Seite endet im Abschnitt **Offen** mitten im Satz".

**Bewertung:** Das Manifest beschreibt den **Vor**-Zustand, nicht den Nach-Zustand. Für `service-task-api.md` und `rest-api.md` ist das falsch (beide sind jetzt vollständig). Für `konfiguration.md` ist es richtig. Das Manifest ist damit **teilweise veraltet** und muss nachgezogen werden.

### 3.4 `explanation/architektur.md` — Widerspruch zu `explanation/konzepte.md` (Nicht-Blocker)

Beide Seiten beschreiben die KV-Buckets unterschiedlich (siehe B-10). Zusätzlich:

- `explanation/architektur.md` §5.1 nennt neun Buckets mit `bpm_*`-Präfix.
- `explanation/konzepte.md` (gelieferte Fassung) nennt neun Buckets mit Kurznamen.
- `explanation/architektur.md` §5.2 nennt `history_completed_instances` — ein Name, der in keiner der beiden Listen steht.

Das ist derselbe Befund wie B-10, nur auf zwei Seiten verteilt.

### 3.5 `reference/service-task-api.md` — Statuscode-Lücke bei `bpmnError` (Nicht-Blocker)

Die Status-Tabelle für `bpmnError` nennt `200`, `401`, `404`. `ServiceTaskNotLocked` (`409`) ist in `engine-server/interfaces.md` belegt, aber nicht für `bpmnError` spezifiziert. Die Tabelle ist möglicherweise unvollständig. Empfehlung: Unter **Offen** ergänzen.

### 3.6 `reference/rest-api.md` — Body-Limit-Zeile unpräzise (Nicht-Blocker)

Die Zeile „JSON-Body-Limit | 10 MiB (`MAX_XML_BYTES`)" ist korrekt, aber der Name `MAX_XML_BYTES` deckt laut `engine-server/interfaces.md` auch JSON ab. Der Revisionsbericht meldet eine Präzisierung, die in der Tabelle nicht sichtbar ist. Empfehlung: Zeile auf „JSON-/XML-Body-Limit" umbenennen.

### 3.7 `how-to/external-task-worker-schreiben.md` — Ankerprüfung (Nicht-Blocker)

Die internen Verweise auf `#3-service-task-und-external-task-element-und-arbeitseinheit` und `#fehler-1-…` bis `#fehler-3-…` zeigen auf Überschriften mit Umlauten und Sonderzeichen. Die Linkprüfung prüft Anker nicht. Das ist im Abschnitt **Offen** korrekt benannt. Kein Fehler.

---

## 4. Direkt korrigierte Fehler

Ich habe **einen** eindeutigen Fehler direkt mit `doc_write` korrigiert:

### 4.1 `reference/service-task-api.md` — defekter Anker in §5

**Vorher:**

```markdown
Erklärung: [`explanation/konzepte.md`](..//explanation/konzepte.md#3-service-task-und-external-task-element-und-arbeitseinheit).
```

**Nachher:**

```markdown
Erklärung: [`explanation/konzepte.md`](../explanation/konzepte.md#3-service-task-und-external-task-element-und-arbeitseinheit).
```

**Begründung:** Der doppelte Slash `..//` ist ein eindeutiger Pfadfehler. Er ist in der Datei vorhanden, im Revisionsbericht aber als „korrigiert" gemeldet. Die Korrektur ist mechanisch und ohne inhaltliche Auswirkung.

**Nicht korrigiert (bewusst):**

- Der Satz „`reference/konfiguration.md` enthält nur den Front-Matter-Kopf (4 Wörter)" in `reference/service-task-api.md` und `reference/rest-api.md` bleibt stehen. Er ist **korrekt**, solange B-01 offen ist. Ihn zu entfernen, wäre eine Falschaussage.
- Die KV-Bucket-Listen in `explanation/architektur.md` und `explanation/konzepte.md` bleiben unverändert. Die Auflösung erfordert einen Blick in `trait_impl.rs`, den ich in dieser Sitzung nicht habe. Eine der beiden Listen zu ändern, ohne die Quelle zu prüfen, wäre Raten.
- Die Statuscode-Tabelle für `bpmnError` bleibt unverändert. Die Lücke ist unter **Offen** benannt; eine Ergänzung ohne Beleg wäre eine Erfindung.

---

## 5. Verbleibende Befunde

| ID | Befund | Schwere | Verantwortlich | Nächster Schritt |
|---|---|---|---|---|
| **B-01** | `reference/konfiguration.md` ist ein 4-Wort-Fragment; Revisionsbericht meldet fälschlich „behoben" | **Blocker** | Technical Writer | Datei schreiben (10 Abschnitte + Offen); Bericht korrigieren |
| **B-03** | Defekter Anker in `service-task-api.md` §5 (jetzt korrigiert); Statuscode-Lücke bei `bpmnError` | **Blocker** (Anker) / Nicht-Blocker (Statuscodes) | Documentation Lead / Technical Writer | Anker korrigiert; Statuscodes unter **Offen** ergänzen |
| **B-05** | `bpmn-elemente.md`, `kompatibilitaet.md` fehlen | **Blocker** | Technical Writer + Parser-Owner | Geprüfte Element-Mapping-Tabelle |
| **B-06** | Beispiele nicht gegen Engine/OpenAPI verifiziert | **Blocker** | Plattform/CI | CI-Job |
| **B-08** | `errorRef`-Semantik unbelegt | **Blocker** | Parser-Owner | Parser-Quelle lesen |
| **B-09** | Mermaid-Rendering nicht verifiziert | **Blocker** | Documentation Lead | `verify_mermaid` ausführen |
| **B-10** | KV-Bucket-Namen widersprüchlich (`bpm_*` vs. Kurznamen); `history_completed_instances` in keiner Liste | **Blocker (neu)** | Persistence-Owner | `trait_impl.rs` prüfen; eine Liste als verbindlich setzen |
| **N-01** | `_qa/content-manifest.md` beschreibt den Vor-Zustand für `service-task-api.md` und `rest-api.md` | Nicht-Blocker | Documentation Lead | Manifest nachziehen |
| **N-02** | `reference/rest-api.md` Body-Limit-Zeile nennt nur „JSON", nicht „JSON/XML" | Nicht-Blocker | Technical Writer | Zeile umbenennen |
| **N-03** | `reference/service-task-api.md` Statuscode-Tabelle für `bpmnError` möglicherweise unvollständig (`409`) | Nicht-Blocker | Technical Writer | Unter **Offen** ergänzen |
| **N-04** | `explanation/architektur.md` §5.2 nennt `history_completed_instances` ohne Beleg in der Bucket-Liste | Nicht-Blocker | Technical Writer | Unter **Offen** ergänzen oder belegen |

---

## 6. Prüfungen nach jeder Änderung

| Prüfung | Ergebnis |
|---|---|
| **Dateizustand gegen Revisionsbericht** | 2 von 8 gemeldeten Korrekturen sind nicht in der Datei vorhanden (B-01 vollständig, B-03 teilweise). Der Bericht ist in diesen Punkten nicht belegbar. |
| **Aussagen gegen Quellen** | Jede geprüfte Aussage in `service-task-api.md`, `rest-api.md`, `external-task-worker-schreiben.md` und `reference/README.md` ist gegen `engine-server/interfaces.md`, `external-task-client/interfaces.md`, `bpmn-parser/interfaces.md`, `persistence/gotchas.md` oder `engine-core/gotchas.md` belegbar. |
| **Statuscodes** | `rest-api.md` Fehlerantworten-Tabelle: 9 Status, alle gegen `engine-server/interfaces.md` §„AppError → HTTP Status Mapping" belegbar. `service-task-api.md`: `bpmnError`-Tabelle möglicherweise unvollständig (N-03). |
| **Befehle** | `bpmnError`-curl vollständig (`/api/service-task/st-0001/bpmnError`); `extendLock`-curl vollständig (`newDuration: 60000`); `fetchAndLock`-curl mit `lockDuration: 30000` und Gegenbeispiel `30` → `400`. Alle Pfade gegen `engine-server/interfaces.md` §„Route Map" belegbar. |
| **Verweise** | `reference/README.md` verlinkt keine fehlenden Seiten mehr. `reference/service-task-api.md` verlinkt `reference/konfiguration.md` nicht (korrekt, weil Fragment). `reference/rest-api.md` verlinkt `reference/konfiguration.md` nicht (korrekt). `explanation/architektur.md` und `explanation/konzepte.md` verweisen auf `reference/konfiguration.md` — das ist ein **toter Verweis auf ein Fragment**, aber im Abschnitt **Offen** benannt. |
| **Terminologie** | „Service Task" vs. „External Task" durchgängig getrennt; `lockDuration`/`newDuration` ausschließlich in ms; `bpmn_process_id` mit `bpmn_id` als Alias. |
| **Front Matter** | `reference/konfiguration.md`: **nur `title`** — alle anderen acht Pflichtfelder fehlen. Alle anderen geprüften Seiten vollständig. |

---

## 7. Freigabeentscheidung

**Status: „Nachbessern" — keine Freigabe.**

Ich erteile die Freigabe **nicht**, weil vier Abnahmekriterien nicht belegt sind:

1. **`reference/konfiguration.md` ist leer** (B-01). Der Revisionsbericht meldet das Gegenteil. Das ist ein **Vertrauensproblem**, nicht nur ein Inhaltsproblem: Ein Bericht, der eine nicht existierende Korrektur behauptet, kann keine Freigabe tragen.
2. **`reference/service-task-api.md` enthält weiterhin einen defekten Anker** (B-03). Ich habe ihn in dieser Runde korrigiert; der Bericht muss nachgezogen werden.
3. **KV-Bucket-Namen widersprechen sich** (B-10). Zwei veröffentlichte Seiten nennen unterschiedliche Namen und zitieren beide eine Quelle. Ein Betriebsteam, das Buckets inspiziert, verwendet den falschen Namen.
4. **Drei Beleglücken bleiben offen** (B-05, B-06, B-08, B-09). Sie sind im Revisionsbericht korrekt als offen gemeldet — das ist der einzige Teil des Berichts, der der Nachprüfung standhält.

**Was ich freigebe:** B-04 und B-07 sind inhaltlich und redaktionell abgeschlossen. Die betroffenen Seiten (`reference/rest-api.md`, `how-to/external-task-worker-schreiben.md`) sind konsistent, belegt und Diátaxis-konform.

**Was ich vom Team erwarte, bevor Runde 2 beginnt:**

- **Technical Writer:** `reference/konfiguration.md` schreiben (schließt B-01). Das ist der einzige Blocker, der ohne externe Quelle schließbar ist.
- **Documentation Lead:** `_qa/revision-round-1.md` korrigieren — B-01 als „nicht behoben" führen, B-03 als „teilweise behoben" mit den zwei offenen Punkten. Ein Revisionsbericht, der eine Korrektur behauptet, die nicht in der Datei steht, ist ein eigener Befund.
- **Persistence-Owner:** `trait_impl.rs` prüfen und die verbindliche Bucket-Liste liefern (schließt B-10).
- **Parser-Owner:** `errorRef`-Semantik aus der Parser-Quelle (schließt B-08).
- **Plattform/CI:** curl-Beispiele gegen Compose-Instanz, Tabellen gegen `docs/openapi.yaml` (schließt B-06).
- **Documentation Lead:** `verify_mermaid` für D1–D5 (schließt B-09).

**Eskalation:** Der Revisionsbericht ist in zwei Punkten nicht wahrheitsgemäß. Ich behandle das nicht als Formfehler, sondern als **technischen Befund**: Ein Bericht, der eine Beleglage behauptet, die der Dateizustand widerlegt, untergräbt die Grundlage jeder Freigabe. Ich empfehle, den Revisionsbericht vor der nächsten Runde zu korrigieren und die Korrektur gegen den Dateizustand zu prüfen — nicht gegen die Absicht.

---

## 8. Geänderte Dateien in dieser Runde

| Datei | Art der Änderung | Befund |
|---|---|---|
| `reference/service-task-api.md` | Defekter Anker `..//explanation/konzepte.md` → `../explanation/konzepte.md` korrigiert | B-03 |
| `_review/technical-review-round-1.md` | **neu** — dieser Bericht | — |

**Nicht geändert:** `reference/konfiguration.md` (kein Inhalt, keine Grundlage), `reference/rest-api.md` (B-04 geschlossen), `how-to/external-task-worker-schreiben.md` (B-07 geschlossen), `reference/README.md` (korrekt), `_architecture/diagram-plan.md` (korrekt), `_qa/content-manifest.md` (N-01, Nachziehen in Runde 2), `explanation/architektur.md` und `explanation/konzepte.md` (B-10, Auflösung erfordert Code-Lesezugriff).
