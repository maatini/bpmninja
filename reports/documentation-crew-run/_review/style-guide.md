---
title: Style Guide — BPMNinja-Dokumentation
description: Anwendbare Schreib- und Formatierungsregeln für alle veröffentlichten Seiten — Terminologie, Stimme, Überschriften, Formatierung, Front Matter, Links, Barrierefreiheit, Sprachqualität und Diátaxis; 47 Regeln, jede mit Beispiel, bewusst knapp.
audience:
  - Beitragende und Maintainer
  - Entwickler und Integratoren
prerequisites:
  - Kenntnis des Glossars unter _review/glossary.md
content_type: reference
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - _review/glossary.md
  - _review/style-report.md
  - _architecture/information-architecture.md
---

# Style Guide — BPMNinja-Dokumentation

**Geltung:** alles unter `tutorials/`, `how-to/`, `reference/`, `explanation/` und `README.md`.
**Rangordnung:** Glossar → dieser Guide → Formulierung der Einzelseite. Der Guide beschreibt den Ist-Stand; er erfindet keine Regel, die keine Seite erfüllt.
**Grundhaltung:** sachlich, direkt, respektvoll. Der Leser ist kompetent, aber neu in diesem System. Wir erklären, wir werben nicht.
**Wenn du nur drei Regeln mitnimmst:** R1.1, R2.3, R5.1.
**Beispiele** stehen in Backticks oder als Zitat; sie sind Illustration, kein Verweis.

## 1. Terminologie

### R1.1 — Ein Konzept, ein Begriff
Genau der Begriff aus dem Glossar, keine stillen Synonyme.
✅ „Die Process Instance bleibt im Zustand `WaitingOnServiceTask`.“ · ❌ „Der Workflow-Lauf bleibt im Zustand `WaitingOnServiceTask`.“

### R1.2 — Produktbegriffe sind unveränderlich
`Service Task`, `External Task`, `Topic`, `Incident`, `Token`, `Wait State`, `Process Definition`, `Process Instance` werden weder übersetzt noch abgekürzt.
✅ „Ein External Task ist die Arbeitseinheit in der Fetch-and-Lock-Queue.“ · ❌ „Ein ext. Task ist die Arbeitseinheit in der Queue.“

### R1.3 — Code-Bezeichner in Backticks
Feldnamen, Variablen, Endpunkte, Zustände, Statuscodes.
✅ „Setze `BIND_ADDR=127.0.0.1`.“ · ❌ „Setze BIND_ADDR auf 127.0.0.1.“

### R1.4 — Zeitwerte immer mit Einheit
An der HTTP-Grenze `ms` oder „Millisekunden“, nie eine nackte Zahl.
✅ „`lockDuration: 30_000` — Millisekunden, nicht `30`.“ · ❌ „`lockDuration: 30` für 30 Sekunden.“

### R1.5 — Sperrliste beachten
Die Liste in `_review/glossary.md` §6 ist verbindlich.
✅ „Die Engine persistiert den Zustand in NATS JetStream.“ · ❌ „Die leistungsstarke Engine persistiert den Zustand nahtlos in NATS JetStream.“

### R1.6 — Produktname „BPMNinja“
„Engine“ ist nach Erstnennung zulässig; „Workflow Engine“ ist gesperrt.
✅ „BPMNinja ist eine in Rust geschriebene Engine, die BPMN-2.0-Prozesse token-basiert ausführt.“ · ❌ „BPMNinja ist eine Workflow-Engine.“

### R1.7 — Feldnamen und Repository-Pfade exakt
Kein Alias, keine Umbenennung „zur Lesbarkeit“. Der TypeScript-Client heißt im Repository `bpmn-ninja-external-task-client`; ein Registry-Paketname ist nicht belegt.
✅ `task.variables_snapshot?.email` · ❌ `task.variables?.email`

## 2. Stimme und Ton

### R2.1 — Ansprache je Bereich
Tutorial und How-to duzen; Referenz und Explanation bleiben unpersönlich.
✅ „Lege eine Datei an.“ (How-to) · ✅ „`GET /api/instances` liefert das vollständige Array.“ (Referenz) · ❌ „Du kannst hier alle Instanzen abrufen.“ (Referenz)

### R2.2 — Keine Wir-Form für das Produkt
✅ „Die Engine dekrementiert `retries`.“ · ❌ „Wir dekrementieren `retries`.“

### R2.3 — Keine unbelegte Aussage
Jede Aussage über Verhalten, Grenzen oder Sicherheit braucht einen Beleg oder eine Markierung als offen.
✅ „Ein fehlender `Content-Type` wird mit **415** beantwortet. **Offen:** die vollständige Liste der Medientypen.“ · ❌ „Der Endpunkt akzeptiert alle üblichen Medientypen.“

### R2.4 — Grenzen benennen, nicht beschönigen
✅ „Das ist kein Fehler der Engine, sondern eine Folge von `lockDuration` < Bearbeitungsdauer.“ · ❌ „In seltenen Fällen kann es zu Überschneidungen kommen.“

### R2.5 — Keine Ausrufezeichen, keine Drohungen
✅ „Ohne gesetzten Key antworten alle `/api`-Routen mit **401**.“ · ❌ „Achtung! Ohne Key geht gar nichts!“

### R2.6 — Eine Anredeform pro Seite
Durchgängig du **oder** durchgängig Sie. Der gesamte Bestand duzt.
✅ „Lege eine Datei an.“ … „Du hast einen Prozess ausgeführt.“ · ❌ „Legen Sie eine Datei an.“ … „Du hast einen Prozess ausgeführt.“

### R2.7 — Anglizismen nur mit Glossareintrag oder Erstnennungserklärung
✅ „Long-Polling (der Client hält die Anfrage offen, bis ein Task eintrifft oder `asyncResponseTimeout` abläuft)“ · ❌ „Der Client pollt long.“

## 3. Überschriften und Struktur

### R3.1 — Genau eine H1, identisch mit `title`
✅ `# External-Task-Worker schreiben und testen` · ❌ Zwei H1 oder eine H1, die vom `title` abweicht.

### R3.2 — Keine Hierarchiesprünge
✅ `##` → `###` → `####` · ❌ `##` → `####`

### R3.3 — Überschriften benennen den Inhalt
Keine Fragen, keine Stichwortketten. Zwei Ausnahmen: die Standardabschnitte aus R3.5 (einwortig erlaubt) und **eine** Einstiegsfrage auf einer Landingpage.
✅ `### Fehler 2: complete antwortet mit 409` · ✅ `## Was möchtest du tun?` auf `README.md` · ❌ `### Probleme?` in einem How-to

### R3.4 — Nummerierung nur für Reihenfolgen und stabile Anker
Tutorial-Schritte und Fehlerlisten werden nummeriert. Referenzseiten dürfen nummerieren, wenn andere Seiten auf die nummerierten Anker verweisen — dann lückenlos ab 1. Abschnitte außerhalb des Referenzkörpers (`## Offen`, `## Prüfung nach dem Start`) bleiben unnummeriert.
✅ `## Schritt 4: Worker-Code schreiben` · ✅ `## 9. Persistierte Strukturen (NATS JetStream)` · ❌ Nummerierte und unnummerierte Abschnitte im selben Referenzkörper

### R3.5 — Standardabschnitte in fester Reihenfolge
**How-to:** `## Aufgabe` → `## Voraussetzungen` → `## Erwartetes Ergebnis` → Schritte → `## Typische Fehler` → `## Nächster sinnvoller Schritt` → `## Offen`.
**Tutorial:** `## Lernziele` → `## Voraussetzungen` → `## Geschätzte Dauer` → optional `## Begriffe, die du in diesem Tutorial brauchst` → Schritte → `## Fehlerbehebung` → `## Aufräumen` → `## Nächster sinnvoller Schritt` → `## Offen`.
Jede Seite endet mit `## Offen`.
✅ `## Voraussetzungen` vor dem ersten Schritt · ❌ Ein How-to, das mit Schritt 1 beginnt und `## Offen` weglässt

### R3.6 — Abnahmekriterien unter `## Woran du erkennst, dass du fertig bist`
Zulässiger Zusatz zwischen Schritten und `## Typische Fehler` bzw. `## Fehlerbehebung`, als Tabelle.
✅ `## Woran du erkennst, dass du fertig bist` mit `| Kriterium | Prüfung | Erwartet |` · ❌ Abnahmekriterien als Fließtext am Seitenende

### R3.7 — Keine erklärenden Zwischenabschnitte in der Handlungsfolge
Ein Abschnitt, der eine Einheit erklärt, steht vor `## Erwartetes Ergebnis` oder als Blockquote im betroffenen Schritt.
✅ Blockquote in Schritt 4: „**Millisekunden:** `lockDuration` ist in ms anzugeben.“ · ❌ Ein eigener H2 über Millisekunden zwischen `## Erwartetes Ergebnis` und `## Schritt 1`

## 4. Formatierung

### R4.1 — Code-Fences immer mit Sprachkennung
✅ ` ```bash `, ` ```json `, ` ```javascript `, ` ```xml `, ` ```text `, ` ```mermaid ` · ❌ ` ``` ` ohne Kennung

### R4.2 — Erwartete Ausgabe in eigenem Block mit Ankündigung
✅ „Erwartete Ausgabe (HTTP 200):“ gefolgt von ` ```json ` · ❌ Ausgabe als Fließtext im Satz

### R4.3 — Tabellen nur für gleichartige Datensätze
Kopfzeile Pflicht, je Spalte eine Bedeutung, keine Absätze in Zellen.
✅ `| Status | Auslöser |` · ❌ Eine Zelle mit drei Absätzen

### R4.4 — Nummerierte Liste nur bei Reihenfolge
✅ `1. Prüfe, ob der NATS-Container läuft.` · ❌ Eine nummerierte Liste ohne Reihenfolge

### R4.5 — Hervorhebung sparsam
`**fett**` für den ersten Auftritt eines Fachbegriffs und für Warnungen, `*kursiv*` für Betonung, `` `Code` `` für Bezeichner. Nie Fettschrift für ganze Sätze.
✅ „**`autoExtendLock: true`** in der Client-Konfiguration aktivieren.“ · ❌ „**Wichtig: Du musst unbedingt den Key setzen.**“

### R4.6 — Blockquotes beginnen mit einer fetten Einleitung
Die Einleitung sagt, worum es geht. Feste Etiketten: `**Hinweis:**`, `**Wichtig:**`, `**Warnung:**`, `**Offen:**`, `**Auth-Hinweis:**`, `**Sicherheitshinweis:**`, `**Migrationswarnung:**`. Eine seitenbezogene Einleitung wie `**Wo du gerade einsteigst:**` ist zulässig, wenn sie das Thema benennt.
✅ „> **Warnung:** Die Engine bindet standardmäßig an `0.0.0.0`.“ · ✅ „> **Wo du gerade einsteigst:** Diese Seite ist Schritt 2 des Lesepfads B.“ · ❌ „> Das ist ein wichtiger Punkt.“

### R4.7 — Zahlen und Einheiten
Tausendertrennung im Fließtext mit schmalem Leerzeichen, Einheit mit Leerzeichen.
✅ „`MAX_XML_BYTES` (Default `10485760`, also 10 MiB)“ · ❌ „`MAX_XML_BYTES` (Default 10485760, also 10MB)“

### R4.8 — Drei Zahlendarstellungen, drei Kontexte
Code-Literal (JS/TS) mit `_` (`lockDuration: 30_000`) · JSON-Body ohne Trennung (`{"lockDuration":30000}`) · Fließtext mit schmalem Leerzeichen („1 000 bis 3 600 000 ms“) · Grenzwert in Backticks in Maschinenform (`` `1000` ``).
✅ „`lockDuration: 30_000` im Code entspricht `"lockDuration":30000` im Body.“ · ❌ `30000` und `30_000` im selben Codeblock

### R4.9 — Statuscodes tragen ein Textäquivalent
✅ „**401** — fehlender oder falscher API-Key“ · ❌ Eine Spalte „Ergebnis“ mit dem einzigen Zellinhalt `401`

## 5. Front Matter

### R5.1 — Neun Pflichtfelder, immer in dieser Reihenfolge
`title`, `description`, `audience`, `prerequisites`, `content_type`, `status`, `last_reviewed`, `owner`, `related`.
✅ Vollständiger Kopf wie in `tutorials/erster-prozess.md` · ❌ Ein Kopf, der nach `title` abbricht

### R5.2 — `description` nennt Nutzen und Umfang in ein bis zwei Sätzen
✅ „Deploye ein minimales BPMN-2.0-Modell, starte eine Instanz und prüfe den Zustand `Completed`.“ · ❌ „Diese Seite beschreibt das Tutorial.“

### R5.3 — `last_reviewed` im Format `YYYY-MM-DD`
✅ `last_reviewed: 2026-10-24` · ❌ `last_reviewed: 24.10.2026`

### R5.4 — `content_type` aus der geschlossenen Werteliste
`tutorial`, `how-to`, `reference`, `explanation`. Die Landingpages sind als `reference` deklariert; ein Typ `landing` ist eine offene Entscheidung.
✅ `content_type: how-to` · ❌ `content_type: guide`

### R5.5 — `status` nur `draft` oder `reviewed`
✅ `status: draft` · ❌ `status: in Arbeit`

### R5.6 — `related` enthält nur existierende Pfade
Geplante Seiten werden im Body als „geplant (P0/P1)“ genannt, nicht in `related` geführt.
✅ `related:` / `  - reference/rest-api.md` · ❌ `related:` / `  - reference/fehlercodes.md`

### R5.7 — `audience` aus der geschlossenen Rollenliste
Nur: „Einsteiger und Lernende“, „Prozessmodellierer“, „Entwickler und Integratoren“, „Betriebsteams und Plattform-Administratoren“, „Stakeholder und Entscheider“, „Beitragende und Maintainer“.
✅ `  - Prozessmodellierer` · ❌ `  - Fortgeschrittene Nutzer, Admins`

## 6. Links

### R6.1 — Beschreibender Linktext, nie „hier“
✅ „Details: [`reference/konfiguration.md`](../reference/konfiguration.md).“ · ❌ „Details [hier](…).“

### R6.2 — Linktext nennt das Ziel, nicht die Aktion
✅ „→ **[`reference/service-task-api.md`](../reference/service-task-api.md)** — kanonische Definition von `lockDuration`.“ · ❌ „→ **Klicke hier für die API**.“

### R6.3 — Relative Pfade für interne Seiten
✅ `[`tutorials/erster-prozess.md`](../tutorials/erster-prozess.md)` · ❌ `https://docs.bpmninja.dev/tutorials/erster-prozess`

### R6.4 — Anker nur auf existierende Überschriften
✅ `[Rollenmatrix](../reference/rest-api.md#rollenmatrix-für-service-task-routen)` · ❌ Ein Anker auf eine Überschrift, die es nicht gibt

### R6.5 — Kein Link auf eine unvollständige Seite ohne Hinweis
Ist eine Zielseite ein Fragment oder als unvollständig markiert, nennt der Link das im selben Absatz.
✅ „Die kanonische Referenz steht in [`reference/konfiguration.md`](../reference/konfiguration.md); Abschnitt 9 ist quellseitig noch nicht bestätigt.“ · ❌ Ein nackter Link auf eine Seite, die nur aus `title` besteht

### R6.6 — Kein Link auf eine nicht existierende Seite
✅ „Die Abweichungsliste steht in `reference/kompatibilitaet.md` (**geplant, P0**).“ · ❌ Ein klickbarer Link auf eine fehlende Datei

### R6.7 — Anker auf Überschriften mit Umlauten sind ungeprüft
Solche Anker sind zulässig, gelten aber erst als belegt, wenn die Auflösung im Zielrenderer einmal geprüft wurde. Bis dahin steht ein Hinweis unter `## Offen` der Quellseite. Der Linkprüfer prüft Anker nicht.
✅ `## Offen` → „**Ankerlinks:** `#fehler-3-…` zeigt auf eine Überschrift mit Umlaut; Auflösung prüfen.“ · ❌ Ein Umlaut-Anker ohne diesen Hinweis

## 7. Barrierefreiheit

### R7.1 — Jedes Bild und jedes Diagramm hat einen Alt-Text
Er beschreibt den Inhalt, nicht die Bildart. Mermaid-Blöcke tragen ihn als HTML-Kommentar direkt über dem Block.
✅ `![Ablauf: Worker A holt Task, Sperre läuft ab, Worker B holt denselben Task, Worker A erhält 409](/assets/ablauf.svg)` · ✅ `<!-- Alt-Text: Sequenzdiagramm mit zwei Workern und der Engine … -->` über einem Mermaid-Block · ❌ `![Diagramm](/assets/ablauf.svg)`

### R7.2 — Farbe und Symbol sind nie der einzige Bedeutungsträger
✅ `| Route | Admin | Worker |` / `| retry | erlaubt | **403** (Rolle nicht zugelassen) |` · ❌ `| Route | Admin | Worker |` / `| retry | ✅ | ❌ |`

### R7.3 — Tabellen haben eine Kopfzeile, die erste Spalte benennt den Datensatz
✅ `| Status | Bedeutung | Auslöser |` · ❌ Eine Tabelle, deren erste Zeile Daten enthält

### R7.4 — Keine Richtungsangaben als Verweis
✅ „Siehe `## Authentifizierung`.“ · ❌ „Siehe oben.“

## 8. Sprachqualität

### R8.1 — Satzlänge: 25 Wörter im Regelfall, 35 im Ausnahmefall
✅ „Die Engine prüft den Key. Fehlt er und bindet der Server nicht an Loopback, antwortet jede `/api`-Route mit **401**.“ · ❌ Ein Satz, der Key-Prüfung, Bind-Adresse, Ausnahmeliste und Statuscode in einem Atemzug nennt

### R8.2 — Aktiv statt Passiv, wenn der Handelnde bekannt ist
✅ „Die Engine setzt `retries` auf 0.“ · ❌ „`retries` wird auf 0 gesetzt.“

### R8.3 — Anglizismen einheitlich schreiben
Substantivisch groß, mit Bindestrich, wo das Glossar es so führt: „Long-Polling“, „Backoff“, „At-least-once“, „fail-closed“ (klein).
✅ „At-least-once, nicht Exactly-once.“ · ❌ „at least once … exactly-once … At Least Once“

### R8.4 — Rechtschreibung nach Duden, Fachbegriffe nach Glossar
✅ „Die Sperre (`lockDuration`) läuft ab.“ · ❌ „Die Sperre (lockDuration) laeuft ab.“

### R8.5 — Abkürzungen bei Erstnennung auflösen
SSE, BPMN, NATS, UUID — sofern die Zielgruppe sie nicht voraussetzt.
✅ „Server-Sent Events (SSE)“ · ❌ „SSE“ im ersten Satz einer Einsteigerseite

## 9. Diátaxis-Konsistenz

### R9.1 — Jede Seite erfüllt genau einen Diátaxis-Zweck
✅ Ein How-to verweist für Default-Werte auf die Referenz · ❌ Ein How-to mit vollständiger Konfigurationstabelle

### R9.2 — Referenzdaten stehen in der Referenz
✅ „Alle Variablen: [`reference/konfiguration.md`](../reference/konfiguration.md).“ · ❌ Eine 20-zeilige Variablentabelle in `how-to/konfiguration-anpassen.md`

### R9.3 — Fehlerbehebung im Tutorial bleibt auf den Tutorialpfad begrenzt
✅ „Für alle anderen Symptome: [Hilfe bei Problemen](../README.md#hilfe-bei-problemen).“ · ❌ Fehlerbilder mit Ursachenanalyse außerhalb des Tutorialpfads

### R9.4 — Explanation enthält keinen Code als Anleitung
✅ „Vereinfacht: `token → serviceTask → waitState`.“ · ❌ Ein vollständiges Worker-Skript in `explanation/architektur.md`

## Offen

- **R5.4 `content_type` für Landingpages:** Die Werteliste kennt keinen Typ `landing`. **Offen:** fünften Wert einführen oder Zuordnung beibehalten.
- **R6.7 Ankerauflösung:** `check_links` prüft keine Fragmente. **Offen:** einmalig prüfen, danach R6.7 entschärfen.
- **R7.1 Bildbeispiel:** `/assets/ablauf.svg` existiert im Ausgabewurzel nicht; das Beispiel zeigt die Alt-Text-Form, kein vorhandenes Asset.
- **Regelumfang:** 47 Regeln in 9 Abschnitten (R1.1–R1.7, R2.1–R2.7, R3.1–R3.7, R4.1–R4.9, R5.1–R5.7, R6.1–R6.7, R7.1–R7.4, R8.1–R8.5, R9.1–R9.4).
