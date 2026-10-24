---
title: Konfiguration anpassen
description: Die BPMNinja-Engine über Umgebungsvariablen für den eigenen Betrieb konfigurieren — Bind-Adresse, Auth-Keys, NATS-Durability, Body-Limits, CORS, Logging, Retry-Queue und Stream-Retention — mit Prüfschritten, einer Abnahmeliste und den Fehlkonfigurationen, die zu 401, 503 oder Datenverlust führen.
audience:
  - Betriebsteams und Plattform-Administratoren
  - Entwickler und Integratoren
  - Beitragende und Maintainer
prerequisites:
  - Laufende BPMNinja-Engine (cargo run -p engine-server oder Container)
  - Laufender NATS-JetStream-Broker
  - curl installiert
  - Zugriff auf die Umgebung, in der die Engine gestartet wird (Shell, systemd-Unit, Compose-Datei oder Kubernetes-Manifest)
content_type: how-to
status: draft
last_reviewed: 2026-10-24
owner: Documentation Lead
related:
  - reference/cli.md
  - reference/rest-api.md
  - explanation/architektur.md
  - how-to/external-task-worker-schreiben.md
  - tutorials/erster-prozess.md
  - _review/glossary.md
---

# Konfiguration anpassen

## Aufgabe

Du willst die BPMNinja-Engine für deine Umgebung konfigurieren, ohne in die drei Fallen zu laufen, die im Betrieb am teuersten sind: **fail-closed-Auth** (alle `/api`-Routen antworten 401), **fail-closed-Durability** (die Engine startet nicht oder `/api/ready` bleibt 503) und **Stream-Retention** (Daten werden gelöscht, weil eine Retention-Variable zu spät gesetzt wurde).

Dieser Guide führt dich durch die Variablen, die du bewusst setzen musst, zeigt nach jedem Block eine Prüfung und nennt die typischen Fehlkonfigurationen. Die vollständigen Tabellen mit Default, Gültigkeitsbereich und Wirkung stehen in [`reference/konfiguration.md`](../reference/konfiguration.md); dieser Guide nennt nur die Werte, die du für die Aufgabe brauchst.

> **Wo du gerade einsteigst:** Diese Seite ist Schritt 1 des Lesepfads C („Ich will es produktiv betreiben"). Danach folgen [`reference/rest-api.md`](../reference/rest-api.md), [`explanation/architektur.md`](../explanation/architektur.md) und [`reference/cli.md`](../reference/cli.md). Alle Pfade: [Dokumentationslandingpage](../README.md#lesepfade).

## Voraussetzungen

- Du kennst die Umgebung, in der die Engine läuft, und kannst sie neu starten.
- NATS JetStream ist erreichbar (`docker compose ps nats` zeigt `running`).
- Du hast ein Terminal für `curl`-Prüfungen.
- Du weißt, ob die Engine hinter einem Load Balancer, in Kubernetes oder lokal läuft — davon hängen `BIND_ADDR` und die Readiness-Prüfung ab.
- Du hast die Retention-Entscheidung noch **nicht** durch einen ersten Start der neuen Version getroffen (siehe Schritt 7).

## Erwartetes Ergebnis

Am Ende hast du eine Konfiguration, die:

1. **nur dort lauscht, wo sie soll** (`BIND_ADDR` bewusst gesetzt),
2. **Auth fail-closed betreibt** (Admin- und Worker-Key getrennt, kein versehentlicher Opt-out),
3. **Durability fail-closed betreibt** (`REQUIRE_NATS=true`, `/api/ready` liefert 200),
4. **Body-Limits und CORS bewusst gesetzt** hat (kein `*` in Produktion),
5. **Logging und Retry-Queue** so eingestellt hat, dass die Log-Datei nicht unbegrenzt wächst und Queue-Drops sichtbar sind,
6. **Stream-Retention vor dem ersten Start** gesetzt hat, damit keine Daten unerwartet gelöscht werden.

---

## Schritt 1: Ausgangszustand prüfen

Bevor du etwas änderst, prüfe, was die Engine aktuell meldet:

```bash
curl -s http://localhost:8081/api/health
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/ready
```

**Erwartetes Ergebnis:**

- `/api/health` liefert **`200` mit leerem Body** (Liveness — der Prozess lebt).
- `/api/ready` liefert `200` (Readiness — die Engine kann Zustand sicher halten).

Wenn `/api/ready` `503` liefert, ist die Persistenz nicht in Ordnung. Behebe das **vor** jeder weiteren Konfigurationsänderung — sonst kannst du die Wirkung deiner Änderungen nicht von einem bestehenden Fehler unterscheiden.

---

## Schritt 2: Bind-Adresse bewusst setzen

`BIND_ADDR` (Alias `BIND_ADDRESS`) steuert, auf welcher Adresse die Engine lauscht. Der Default ist `0.0.0.0` — für Container nötig, lokal ohne Key gefährlich.

| Umgebung | Empfohlener Wert | Begründung |
|---|---|---|
| Lokal ohne Key | `127.0.0.1` | Nur Loopback; das fail-closed-Auth-Gate greift dann nicht. |
| Container / Kubernetes | `0.0.0.0` | Der Container muss von außen erreichbar sein. **Dann ist ein Key Pflicht.** |
| Hinter Reverse Proxy | `127.0.0.1` | Der Proxy terminiert TLS und reicht weiter. |

```bash
BIND_ADDR=127.0.0.1 cargo run -p engine-server
```

**Erwartetes Ergebnis:** Die Engine lauscht nur auf der gesetzten Adresse.

```bash
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:8081/api/health
```

Erwartet: `200`. Ein Aufruf über eine andere lokale IP sollte fehlschlagen (Verbindung abgelehnt).

> **Wichtig:** Wenn du `BIND_ADDR` auf `0.0.0.0` lässt und **keinen** Key setzt, antworten alle `/api`-Routen mit **401** (fail-closed). Das ist kein Bug, sondern der Schutz gegen versehentlich offene Engines.

---

## Schritt 3: Auth-Keys setzen und Rollen trennen

Die Engine kennt zwei Keys:

| Variable | Rolle | Darf |
|---|---|---|
| `BPMNINJA_API_KEY` | Admin | alles, inkl. `retry`, `resolve`, `/metrics`, Deploy, Instanzen |
| `BPMNINJA_WORKER_API_KEY` | Worker | nur `fetchAndLock`, `complete`, `failure`, `extendLock`, `bpmnError` |

Setze beide in Produktion:

```bash
export BPMNINJA_API_KEY="$(openssl rand -hex 32)"
export BPMNINJA_WORKER_API_KEY="$(openssl rand -hex 32)"
BIND_ADDR=0.0.0.0 cargo run -p engine-server
```

**Erwartetes Ergebnis:** Ohne Key antwortet jede `/api`-Route außer `/api/health` und `/api/ready` mit **401**:

```bash
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/definitions
```

Erwartet: `401`. Mit Admin-Key:

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  http://localhost:8081/api/definitions
```

Erwartet: `200`. Mit Worker-Key auf einer Admin-Route:

```bash
curl -s -o /dev/null -w "%{http_code}\n" \
  -H "X-API-Key: $BPMNINJA_WORKER_API_KEY" \
  http://localhost:8081/api/definitions
```

Erwartet: `403`.

> **`BPMNINJA_ALLOW_UNAUTHENTICATED=true`** ist der einzige bewusste Opt-out aus dem fail-closed-Gate. Er ist **nur für lokale Entwicklung** gedacht. Setze ihn nie in einer Umgebung, die von außen erreichbar ist.

---

## Schritt 4: Durability fail-closed betreiben

`REQUIRE_NATS` steuert, ob die Engine ohne NATS startet. Der Default ist `true`: Ohne NATS bricht der Start ab, statt stillschweigend in den In-Memory-Modus zu wechseln.

| Wert | Verhalten | Wann |
|---|---|---|
| `true` (Default) | Start bricht ab, wenn NATS nicht erreichbar ist. `/api/ready` spiegelt die Persistenz. | Produktion, jede Umgebung mit Daten, die einen Neustart überleben müssen. |
| `false` | In-Memory-Fallback mit Warnung. **Alle Daten gehen bei Neustart verloren.** `/api/ready` bleibt 503. | Nur ephemere lokale Läufe. |

```bash
export REQUIRE_NATS=true
export NATS_URL="nats://bpmninja:bpmninja@nats:4222"
```

**Erwartetes Ergebnis:** `/api/ready` liefert `200`, solange NATS erreichbar ist. Stoppe NATS testweise und prüfe:

```bash
docker compose stop nats
curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/ready
```

Erwartet: `503`. Starte NATS wieder (`docker compose start nats`) und prüfe erneut — `200`.

> **Nicht** `REQUIRE_NATS=false` setzen, nur um einen Startfehler loszuwerden. Das schaltet die Persistenz ab und ist der häufigste Weg zu Datenverlust. Warum die Engine hier bewusst fail-closed ist: [`explanation/architektur.md`](../explanation/architektur.md).

---

## Schritt 5: Body-Limits und CORS setzen

**Body-Limits:**

| Variable | Default | Wirkung |
|---|---|---|
| `MAX_XML_BYTES` | `10485760` (10 MiB) | JSON/XML-Body und Deploy-Cap. Überschreitung → **413**. |
| `MAX_UPLOAD_BYTES` | `5242880` (5 MiB) | Multipart-Datei-Uploads an Instanzen. Überschreitung → **413**. |

**CORS:** `CORS_ORIGINS` ist eine kommagetrennte Origin-Liste. Ohne Wert gilt eine Allowlist für Tauri und lokale Entwicklung (`http://localhost:1420`, `http://127.0.0.1:1420`, `tauri://localhost`, `https://tauri.localhost`). `CORS_ORIGINS=*` ist der einzige Weg, jeden Origin zu erlauben — **nur für Entwicklung**.

```bash
export CORS_ORIGINS="https://bpmn.example.com"
export MAX_XML_BYTES=10485760
export MAX_UPLOAD_BYTES=5242880
```

**Erwartetes Ergebnis:** Ein Deploy mit einer Datei über 10 MiB antwortet mit **413**:

```bash
head -c 11000000 /dev/zero | tr '\0' ' ' > zu-gross.xml
jq -Rs '{name:"zu-gross", xml:.}' zu-gross.xml > zu-gross.json
curl -s -o /dev/null -w "%{http_code}\n" -X POST http://localhost:8081/api/deploy \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $BPMNINJA_API_KEY" \
  --data-binary @zu-gross.json
```

Erwartet: `413`.

---

## Schritt 6: Logging und Retry-Queue einstellen

**Logging:**

| Variable | Default | Wirkung |
|---|---|---|
| `RUST_LOG` | `info` | `tracing`-Filterausdruck, z. B. `debug`, `warn`, `engine_server=trace`. |
| `LOG_FORMAT` | `text` | `text` oder `json` (strukturierte Zeilen für Log-Aggregation). |
| `LOG_FILE` | `engine_logs.jsonl` | Datei-Fallback für den Log-Buffer. `off` deaktiviert die Dateipersistenz. |

**Retry-Queue:**

| Variable | Default | Wirkung |
|---|---|---|
| `PERSISTENCE_RETRY_QUEUE_CAPACITY` | `10000` | Größe der begrenzten Background-Retry-Queue. Ist sie voll, werden neue Jobs **verworfen** und als `bpmn_persistence_retry_dropped_total` gezählt. |

```bash
export RUST_LOG=info
export LOG_FORMAT=json
export LOG_FILE=/var/log/bpmninja/engine_logs.jsonl
export PERSISTENCE_RETRY_QUEUE_CAPACITY=10000
```

**Erwartetes Ergebnis:** Die Engine schreibt strukturierte Zeilen in die angegebene Datei, und der Log-Buffer ist abfragbar:

```bash
curl -s -H "X-API-Key: $BPMNINJA_API_KEY" \
  "http://localhost:8081/api/logs?level=warn&limit=5"
```

Erwartet: `200` mit höchstens fünf Einträgen. Prüfe zusätzlich, dass die Datei existiert und wächst (`ls -l /var/log/bpmninja/engine_logs.jsonl`).

> **Log-Rotation ist deine Aufgabe.** Die Datei wächst unbegrenzt, solange `LOG_FILE` gesetzt ist. Richte `logrotate` oder ein Äquivalent ein, oder setze `LOG_FILE=off` und verlasse dich auf den In-Memory-Puffer (5 000 Einträge) und den NATS-Stream `ENGINE_LOGS` (50 000 Einträge).

---

## Schritt 7: Stream-Retention **vor** dem ersten Start setzen

`WORKFLOW_EVENTS` und `WORKFLOW_HISTORY` werden bei **jedem** `connect` mit begrenzten Limits erstellt **und abgeglichen** (`max_age`, `max_bytes`, `num_replicas`).

| Variable | Default | `0` bedeutet |
|---|---|---|
| `BPMNINJA_EVENT_RETENTION_DAYS` | `7` | unbegrenzt |
| `BPMNINJA_HISTORY_RETENTION_DAYS` | `30` | unbegrenzt |
| `BPMNINJA_STREAM_MAX_BYTES` | `1073741824` (1 GiB) | unbegrenzt |
| `BPMNINJA_STREAM_REPLICAS` | `1` | ungültig → auf `1` geklemmt; Werte > 5 werden auf `5` geklemmt |

> **Migrationswarnung — Datenverlust möglich:** Der Abgleich merged **nur** `max_age`, `max_bytes` und `num_replicas` per `update_stream`; Subjects, Storage-Typ und Discard-Policy bleiben erhalten. Der **erste Start mit niedrigeren Limits wendet diese sofort an** — NATS verwirft dann ältere bzw. größere Nachrichten. Wenn du bestehende Historie behalten willst, setze die Retention-Variablen **vor** dem ersten Start der neuen Version auf `0` (oder einen ausreichend großen Wert) oder erhöhe die Limits nach Prüfung des Streams.

```bash
# Vor dem ersten Start der neuen Version: bestehende Historie nicht beschneiden
export BPMNINJA_EVENT_RETENTION_DAYS=0
export BPMNINJA_HISTORY_RETENTION_DAYS=0
export BPMNINJA_STREAM_MAX_BYTES=0
export BPMNINJA_STREAM_REPLICAS=1
```

**Erwartetes Ergebnis:** Nach dem Start sind die Stream-Limits unverändert bzw. wie gesetzt. Prüfe das über das NATS-Monitoring (Port `8222` im Compose-Stack, nur Loopback):

```bash
curl -s http://127.0.0.1:8222/jsz?streams=true | grep -E '"name":"WORKFLOW_(EVENTS|HISTORY)"|max_age|max_bytes|num_replicas'
```

Erwartet: Für beide Streams erscheinen `max_age`, `max_bytes` und `num_replicas` mit den Werten, die du gesetzt hast (`max_age: 0` bedeutet unbegrenzt).

> **Wenn dein NATS-Konto `STREAM.UPDATE` verbietet**, schlägt der Start mit der Serverursache fehl, statt still die alten Limits zu behalten. Das ist beabsichtigt: Ein stiller Fehlschlag würde eine falsche Retention vermuten lassen.

---

## Woran du erkennst, dass du fertig bist

Führe diese Prüfliste nach dem Neustart einmal vollständig durch. Jede Zeile ist ein Abnahmekriterium.

| Prüfung | Befehl | Erwartet |
|---|---|---|
| Liveness | `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/health` | `200` |
| Readiness | `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/ready` | `200` |
| Auth fail-closed | `curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8081/api/definitions` | `401` |
| Admin-Rolle | derselbe Aufruf mit `X-API-Key: $BPMNINJA_API_KEY` | `200` |
| Worker-Rolle | derselbe Aufruf mit `X-API-Key: $BPMNINJA_WORKER_API_KEY` | `403` |
| Body-Limit | Deploy einer Datei > `MAX_XML_BYTES` | `413` |
| Durability | NATS stoppen, `/api/ready` prüfen | `503`, nach Neustart `200` |
| Logging | `GET /api/logs?limit=5` mit Admin-Key | `200` |
| Retention | `curl -s http://127.0.0.1:8222/jsz?streams=true` | Limits wie gesetzt |

Wenn eine Zeile abweicht, behebe sie, bevor du die Konfiguration als abgenommen betrachtest.

---

## Typische Fehler

### Fehler 1: Alle `/api`-Routen antworten mit 401

**Symptom:** Jeder Aufruf außer `/api/health` und `/api/ready` liefert **401**, obwohl du keinen Key gesetzt hast.

**Ursache:** Die Engine bindet an eine Nicht-Loopback-Adresse (Default `0.0.0.0`) und es ist **kein** Key gesetzt. Das fail-closed-Gate sperrt dann alle `/api`-Routen. `127.0.0.1`, `::1` und `localhost` gelten als Loopback; jeder andere Wert — auch `0.0.0.0` — nicht.

**Behebung:** Entweder `BIND_ADDR=127.0.0.1` setzen (nur lokal) oder einen Key setzen und mitsenden:

```bash
curl -s -H "X-API-Key: $BPMNINJA_API_KEY" http://localhost:8081/api/definitions
```

**Nicht** die Lösung: `BPMNINJA_ALLOW_UNAUTHENTICATED=true` in einer von außen erreichbaren Umgebung. Fehlversuche werden als `bpmn_auth_failures_total{reason}` gezählt (`invalid_or_missing_key`, `worker_role_forbidden`) — ein steigender Zähler ist der Beleg, dass jemand ohne gültigen Key zugreift.

### Fehler 2: `/api/ready` bleibt 503 oder die Engine startet nicht

**Symptom A:** Der Start bricht mit einer NATS-Verbindungsfehlermeldung ab.
**Symptom B:** Die Engine läuft, aber `/api/ready` liefert dauerhaft `503`.

**Ursache:** `REQUIRE_NATS=true` (Default) und NATS/JetStream ist nicht erreichbar, oder das Bootstrap der Streams/Buckets schlägt fehl.

**Behebung:**

1. Prüfe den Broker: `docker compose ps nats`.
2. Prüfe die URL. Im Compose-Stack verlangt NATS Credentials: `NATS_URL=nats://bpmninja:bpmninja@nats:4222`. Ein unauthentifiziertes `nats://localhost:4222` schlägt gegen diesen Stack fehl.
3. Prüfe, ob JetStream aktiv ist — ohne JetStream gibt es keine KV-Buckets und keinen Object Store.
4. Prüfe, ob das NATS-Konto `STREAM.UPDATE` erlaubt (Retention-Abgleich, Schritt 7).

**Nicht** die Lösung: `REQUIRE_NATS=false`. Das schaltet die Persistenz ab, alle Daten gehen beim Neustart verloren, und `/api/ready` bleibt `503`.

### Fehler 3: Nach dem Upgrade fehlen ältere Historieneinträge

**Symptom:** Nach dem ersten Start der neuen Version sind `WORKFLOW_EVENTS`- oder `WORKFLOW_HISTORY`-Nachrichten verschwunden.

**Ursache:** Der Retention-Abgleich hat beim Start niedrigere Limits angewendet, als der Stream zuvor hatte. NATS hat ältere bzw. größere Nachrichten verworfen. Das passiert **beim ersten Start**, nicht erst wenn du die Variable später änderst.

**Behebung:** Vor dem ersten Start der neuen Version `BPMNINJA_EVENT_RETENTION_DAYS=0`, `BPMNINJA_HISTORY_RETENTION_DAYS=0` und `BPMNINJA_STREAM_MAX_BYTES=0` setzen, starten, Stream prüfen, dann schrittweise auf die gewünschten Werte senken. Bereits verworfene Nachrichten sind nicht wiederherstellbar.

### Fehler 4: Die Log-Datei füllt das Dateisystem

**Symptom:** `engine_logs.jsonl` wächst unbegrenzt; irgendwann schlägt der Start oder ein Schreibvorgang fehl.

**Ursache:** `LOG_FILE` ist gesetzt (Default `engine_logs.jsonl`), aber es gibt keine Rotation. Die Engine rotiert die Datei nicht selbst.

**Behebung:** `logrotate` einrichten oder `LOG_FILE=off` setzen und den In-Memory-Puffer (5 000 Einträge) plus den NATS-Stream `ENGINE_LOGS` (50 000 Einträge) nutzen.

---

## Nächster sinnvoller Schritt

Du hast eine Konfiguration, die fail-closed startet und ihre Grenzen belegt. Der nächste Schritt ist der Vertrag, den deine Clients sehen:

→ **[`reference/rest-api.md`](../reference/rest-api.md)** — Auth-Modi, Rollenmatrix, Statuscodes und das Fehlerformat `{ "error": … }`.

Wenn du verstehen willst, warum die Engine bei NATS-Ausfall Jobs verwirft statt unbegrenzt zu puffern, lies:

→ **[`explanation/architektur.md`](../explanation/architektur.md)** — Retry-Queue, Retention-Abgleich und die Single-Writer-Grenze.

Wenn du die Startbefehle und Compose-Portbindungen nachschlagen willst, lies:

→ **[`reference/cli.md`](../reference/cli.md)** — Engine-Start, Docker Compose, Desktop-UI.

---

## Offen

- **Vollständigkeit:** Die kanonische Tabelle aller Umgebungsvariablen steht in [`reference/konfiguration.md`](../reference/konfiguration.md). Dieser Guide nennt die aufgabenrelevanten Werte; Variablen ohne Bezug zur Einrichtung (etwa Rhai-Limits) sind dort dokumentiert.
- **`GET /api/info` als Prüfschritt:** In einer früheren Fassung wurde `/api/info` als Prüfschritt verwendet. Die Antwortstruktur ist nicht belegt; der Schritt ist entfernt und durch `/api/health` und `/api/ready` ersetzt.
- **Metriken:** `reference/metriken.md` existiert nicht. Die Zähler `bpmn_auth_failures_total` und `bpmn_persistence_retry_dropped_total` sind hier genannt, aber nicht mit einer vollständigen Metrikliste hinterlegt.
- **Ankerlinks:** Die internen Verweise auf `#fehler-1-…` bis `#fehler-4-…` zeigen auf Überschriften mit Umlauten und Sonderzeichen. Die Linkprüfung prüft Anker nicht. **Offen:** Ankerauflösung im Zielsystem einmalig prüfen.
