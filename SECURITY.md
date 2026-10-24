# Sicherheitsrichtlinie

BPMNinja ist eine Workflow-Engine mit Netzwerk-API, NATS-Persistenz und
External-Task-Workern. Wir nehmen Sicherheitsmeldungen ernst und bitten darum,
Schwachstellen verantwortungsvoll zu melden.

## Unterstützte Versionen

Sicherheitsfixes erscheinen ausschließlich für die jeweils aktuelle
Minor-Version der 0.7-Reihe.

| Version | Unterstützt |
|---|---|
| 0.7.x (aktuelles Release) | ja |
| ältere 0.7.x | nein — bitte auf die aktuelle Version aktualisieren |
| < 0.7.0 | nein |

Es gibt keine Long-Term-Support-Zusage. Der `main`-Branch ist die
Entwicklungslinie, Releases werden über GitHub Releases getaggt.

## Schwachstelle melden

**Bitte keine öffentlichen Issues für Sicherheitslücken.**

1. GitHub Security Advisory im Repository anlegen:
   <https://github.com/maatini/bpmninja/security/advisories/new>
2. Alternativ den Maintainer [@maatini](https://github.com/maatini) über GitHub
   kontaktieren, falls kein Advisory möglich ist.

Hilfreich sind:

- betroffene Version bzw. Commit
- Reproduktion (BPMN-Datei, HTTP-Request, Konfiguration)
- erwartete und tatsächliche Wirkung
- ob eine öffentliche Offenlegung geplant ist und in welchem Zeitrahmen

Bitte keine echten Secrets, personenbezogenen Daten oder produktiven
Prozessdefinitionen mitschicken.

## Reaktionszeiten

Diese Zeiten sind Ziele, keine Garantie — das Projekt wird nebenbei gepflegt:

| Schritt | Ziel |
|---|---|
| Eingangsbestätigung | 3 Werktage |
| Erste Einschätzung (Schweregrad, Betroffenheit) | 10 Werktage |
| Fix oder Zwischenmitteilung | 30 Tage |

Du erhältst eine Rückmeldung, sobald der Fix in einem Release enthalten ist.
Auf Wunsch nennen wir dich im Advisory, sofern du das möchtest. Ein Bug-Bounty-
Programm gibt es nicht.

## Sicherheitsrelevante Konfiguration

Die Defaults sind fail-closed gewählt; für Exposition außerhalb von `localhost`
sind folgende Punkte zu prüfen:

- **API-Key:** `BPMNINJA_API_KEY` aktiviert Bearer-/`X-API-Key`-Auth. Ohne
  gesetzten Key ist die API offen (Dev-Default). In produktiven Deployments
  immer setzen.
- **CORS:** `CORS_ORIGINS` ist eine kommagetrennte Allowlist; ungesetzt gilt
  eine feste Liste statt `Any`. `*` ist ausschließlich als Dev-Notausgang
  gedacht.
- **Limits:** XML-/JSON-Body maximal 10 MiB (`MAX_XML_BYTES`), Upload maximal
  5 MiB (`MAX_UPLOAD_BYTES`). Höhere Werte vergrößern die Angriffsfläche.
- **Persistence:** `REQUIRE_NATS=true` ist der Default. Schlägt der
  NATS-Bootstrap fehl, startet der Server nicht mit In-Memory-Fallback.
- **Compose:** NATS läuft mit Authentifizierung und Loopback-Ports; diese
  Einstellungen nicht ungeprüft nach außen öffnen.
- **TLS:** Die Engine terminiert kein TLS. Für Internet-Exposition einen
  Reverse-Proxy mit TLS und vorgeschalteter Authentifizierung verwenden —
  Details im Abschnitt „Betriebshärtung" unten.

## Betriebshärtung: TLS, Rate-Limiting, Credentials

Weder TLS-Terminierung noch Rate-Limiting sind in der Engine implementiert.
Beides gehört in die vorgelagerte Infrastruktur (Reverse Proxy / Ingress); die
folgenden Punkte sind Betriebsempfehlungen, keine eingebauten Features.

### TLS-Terminierung

- Reverse Proxy (nginx, Caddy, Traefik oder Cloud-Loadbalancer) vor `PORT`
  (Default 8081) setzen und `engine-server` nur im internen Netz bzw. im
  Compose-/Pod-Netz lauschen lassen.
- Nach außen ausschließlich `https://` und `wss://` anbieten (die Desktop-UI
  spricht HTTP/WebSocket); Klartext-Ports am Proxy auf HTTPS umleiten.
- Zertifikate am Proxy verwalten (ACME/Let's Encrypt). Die Engine hat keine
  Zertifikats- oder Keystore-Konfiguration.
- NATS ebenfalls absichern: Client und Server über ein vertrauenswürdiges Netz
  oder TLS (`tls://…` in `NATS_URL`, Zertifikate in der `nats-server`-Konfiguration).
  Sonst laufen JetStream-Daten und Log-Streams im Klartext.
- Der Proxy kann `X-Forwarded-Proto`/`X-Forwarded-For` setzen; die Engine wertet
  diese Header nicht aus. Auth-Entscheidungen hängen nur an `BIND_ADDR`,
  gesetzten API-Keys und `BPMNINJA_ALLOW_UNAUTHENTICATED`.

### Rate-Limiting

Rate-Limits im Proxy pro Client (API-Key bzw. IP) durchsetzen, nicht in der
Engine. Startwerte:

| Route | Empfehlung | Begründung |
|---|---|---|
| `POST /api/deploy` | 10 req/min | XML-Parsing und Deployment, teuer pro Request |
| `POST /api/start`, `/api/message`, `/api/service-task/fetchAndLock` | 60 req/min | schreiben Instanz- und Task-Zustand |
| übrige `/api/*`-Lesezugriffe | 300 req/min | UI-Polling und Monitoring |
| `GET /metrics` | nur intern | Prometheus-Scrape, nicht öffentlich |

Zusätzlich die Body-Limits (`MAX_XML_BYTES` 10 MiB, `MAX_UPLOAD_BYTES` 5 MiB) im
Proxy spiegeln (z. B. `client_max_body_size 10m`). Die In-Process-Limits schützen
nur gegen große Bodies, nicht gegen Request-Fluten; 429/413-Antworten nicht
automatisch wiederholen.

### Credentials in `NATS_URL`

- `NATS_URL` kann Credentials (`nats://user:pass@nats:4222`) oder einen Token im
  Query-String (`?token=…`) enthalten. Werte über Docker-/Kubernetes-Secret oder
  eine `.env` außerhalb des Repos injizieren, nicht im Compose-File oder als
  Image-`ENV` hinterlegen. Das `ENV NATS_URL=nats://nats:4222` im Image ist
  bewusst credential-frei.
- Die Engine maskiert die URL in ihren Logs, Startfehlern und in
  `GET /api/info` (`nats://***@nats:4222`, Query-String → `?***`).
- Credentials rotieren: NATS-User/Account neu anlegen, Secret aktualisieren,
  Container neu starten. `NATS_URL` wird nur beim Start gelesen.
- `docker inspect` und Compose-Environment sind für jeden mit Docker-Zugriff
  lesbar — deshalb Secrets statt Umgebungsvariablen bevorzugen.

## Umfang

Zum Umfang zählen unter anderem: Parser (XML-Entities, Rekursion, Ressourcen-
verbrauch), Rhai-Skripte, Timer- und Incident-Pfade, Auth-/CORS-Verhalten der
REST-API, Persistenz- und Restore-Logik sowie die Desktop-App inklusive CSP.

Nicht als Schwachstelle gelten: fehlende Auth bei bewusst gesetztem leerem
`BPMNINJA_API_KEY` in lokalen Setups, Denial-of-Service durch absichtlich
überdimensionierte BPMN-Dateien innerhalb der dokumentierten Limits sowie
veraltete Abhängigkeiten ohne nachgewiesene Ausnutzbarkeit.
