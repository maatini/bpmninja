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
  Reverse-Proxy mit TLS und vorgeschalteter Authentifizierung verwenden.

## Umfang

Zum Umfang zählen unter anderem: Parser (XML-Entities, Rekursion, Ressourcen-
verbrauch), Rhai-Skripte, Timer- und Incident-Pfade, Auth-/CORS-Verhalten der
REST-API, Persistenz- und Restore-Logik sowie die Desktop-App inklusive CSP.

Nicht als Schwachstelle gelten: fehlende Auth bei bewusst gesetztem leerem
`BPMNINJA_API_KEY` in lokalen Setups, Denial-of-Service durch absichtlich
überdimensionierte BPMN-Dateien innerhalb der dokumentierten Limits sowie
veraltete Abhängigkeiten ohne nachgewiesene Ausnutzbarkeit.
