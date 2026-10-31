# Changelog

Alle nennenswerten Änderungen an BPMNinja werden in dieser Datei dokumentiert.

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/),
die Versionierung [Semantic Versioning](https://semver.org/lang/de/).

## [Unreleased]

### Hinzugefügt

- Event-Subprozesse (`subProcess triggeredByEvent="true"`) direkt im Prozess.
  Message-, Timer-, Signal-, Error- (nur unterbrechend) und Escalation-Starts.
  Der Scope abonniert beim Instanzstart und räumt Subscriptions beim Verlassen auf.
  Unterbrechende Starts brechen die Scope-Tokens ab, nicht unterbrechende laufen
  parallel. Verschachtelte Event-Subprozesse, Conditional-Starts und Sequenzflüsse
  am Container bleiben abgelehnt. Subscriptions überstehen einen NATS-Neustart.

- Collaboration (X12): `parse_bpmn_collaboration` liest Pools, Participants und
  gerichtete Message-Flows. `POST /api/deploy/collection` deployed alle Prozesse
  mit `isExecutable="true"` atomar, pinnt die Collaboration an die vergebenen
  Definition-Versionen und schreibt sie als einen JetStream-KV-Commit
  (`definition_collections`). `plan_internal_messages` wählt den Empfänger der
  gepinnten Version (Start, genau ein Catch, oder explizit unavailable). Es gibt
  keine automatische Zustellung. `POST /api/deploy` bleibt ein Prozess.
  Ein instanziierender Receive-Task auf Prozessebene ist ein Message-Start.
  Message-End-Events und Schleifen bleiben abgelehnt.

- Daten-Mappings (X11): Prozess-Defaults (`bpmninja:default`), Camunda-`inputOutput`
  (Literal, `${variable}`, Liste, Map) und BPMN-Input-/Output-Assoziationen an
  User-, Service-, Script- und Send-Tasks sowie Call Activities. `camunda:in` /
  `camunda:out` ersetzen den vollständigen Variablenaustausch, sobald eine
  Richtung gesetzt ist. Lokale Inputs bleiben bis zum erfolgreichen Abschluss
  unsichtbar; fehlende Outputs halten den Task retry-fähig. Ein Fehler beim
  Betreten der Aktivität beendet die Instanz mit `DATA_MAPPING_ERROR`.
  Der Modeler bearbeitet Defaults und Mappings im Properties Panel.

### Geändert

- **Breaking:** Der Deploy lehnt BPMN-Elemente ab, deren Ausführung die Engine nicht
  kennt, statt sie still zu verwerfen oder auf ein schwächeres Element abzubilden.
  `InvalidDefinition` nennt Typ, Id und Zeile **aller** Treffer im deployten Prozess.
  Neu abgelehnt werden unter anderem `adHocSubProcess`, `standardLoopCharacteristics`,
  instanziierende Receive-Tasks, instanziierende
  oder parallele Event-Based-Gateways, Message-End-Events, mehrere Event-Definitionen
  am selben Ereignis und Trigger an Start-Events eingebetteter Sub-Prozesse.
  Signal-, Link- und Conditional-Events (Catch/Boundary) sowie sequentielles
  Multi-Instance auf Service-Tasks bleiben gültig. Lanes, Annotationen und
  Datenobjekte bleiben erlaubt. Kein Umschalter. Matrix:
  `docs/knowledge-base/modules/bpmn-parser/element-matrix.md`.

## [0.7.25] - 2026-10-24

Release mit Nx-Orchestrierung, dreistufigem Release-Workflow und der
Incident-Domäne in der Engine.

### Hinzugefügt

- Nx orchestriert lokale und CI-Targets für Rust-Workspace, Desktop, TS-Client
  und TypeSpec. Cargo bleibt die Build-Engine für Rust.
- CI wird auf betroffene Projekte zugeschnitten: ein Plan-Job ermittelt sie über
  `nx show projects --affected`, Jobs für unbeteiligte Projekte entfallen.
- Tauri-Paritätsprüfung `desktop-tauri/scripts/assert-tauri-parity.mjs`, erreichbar
  über `npm run test:deps`, vergleicht Crate- und npm-Versionen von `tauri`,
  `tauri-plugin-dialog` und `tauri-plugin-fs`.
- Community-Dateien: `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`,
  `CHANGELOG.md`, Issue-Formulare, PR-Template und `CODEOWNERS`.
- Engine: eigene Incident-Domäne mit Persistenz sowie REST-Endpunkten zum
  Auflisten und Auflösen von Incidents.
- External-Task-Client: Timeout und Retry für `completeTask` und
  `handleFailure`, Integrationstests gegen einen echten `engine-server` und
  ein vollständiges npm-Paket.

### Geändert

- Release-Workflow in drei Stufen: Draft anlegen, Builds für Linux, Windows und
  macOS (arm/intel), anschließend Publish. Schlägt ein Build fehl, bleibt der
  Draft zur Inspektion stehen.
- Eine Versionsquelle (`version.json`) als Canonical; ein Sync-Werkzeug schreibt
  Cargo-, Tauri- und npm-Manifeste. `npm run version:check` prüft Drift in der CI.
- Qualitätsmetriken erscheinen wöchentlich auf dem `metrics`-Branch statt bei
  jedem Lauf als direkter Commit auf `main`.

### Entfernt

- `reports/` ist nicht mehr versioniert (479 Dateien, rund 0,94 MB, davon 442
  `.lance`-Dateien). `reports/README.md` bleibt als Zusammenfassung im Repo.

### Behoben

- Server: Der History-Filter wertet `actor_type` case-insensitiv aus; das
  Desktop sendet den Filter wieder als Server-Parameter.
- API: TypeSpec, OpenAPI und Server-Verträge für Deploy, Start und Incidents
  sind angeglichen.

## [0.7.24] - 2026-10-10

Produkt-Release nach Welle 1–5: Engine-Korrektheit, Camunda-/Modeler-Interop,
Production-Bar für LAN, Betrieb/Restore und Hygiene.

### Hinzugefügt

- Server: CORS-Allowlist statt `Any`; optionaler API-Key über `BPMNINJA_API_KEY`
  (Bearer und `X-API-Key`).
- Server: Instances-API mit `limit`/`offset` und `X-Total-Count`; Restore liest
  bevorzugt JSON und behandelt `keys()`-Fehler als Fehler statt als leeres Bucket.
- Desktop: Assignee- und Topic-Properties-Provider, API-Key und SSE-URL aus den
  Settings.
- CI: TypeSpec-Kompilierung, `cargo deny` und ein `cargo check`-Gate vor
  `docker-publish`; die Mutation-Summary warnt bei überlebenden Mutanten.
- Docker: native Multi-Arch-Builds für `linux/amd64` und `linux/arm64` statt QEMU.

### Geändert

- HTTP-Felder `lockDuration`/`newDuration` werden in Millisekunden gelesen
  (Camunda-kompatibel statt Sekunden).
- Engine: Timer-Due-Index für `process_timers`; `unit_tests.rs` nach Domänen
  aufgeteilt.
- Parser: User-/Service-Tasks lesen `data-*` und `camunda:assignee`/`camunda:topic`.
- OpenAPI auf 0.7.23; README und Agent-Spec an 29 BPMN-Varianten,
  Inclusive-Join-Abweichung und Multi-Instance-Reject angeglichen.
- Lockfile-Bump für rustls/h2/webpki; `cargo-deny` wieder ohne Ignores.
- External-Task-Client nutzt standardmäßig Port `:8081` und sendet
  Millisekunden ohne Umrechnung.

### Behoben

- Engine: XOR-Merge nimmt unbedingte Flows; Inclusive-Join wartet auf die
  tatsächlich genommenen Split-Pfade.
- Engine: `R3`-Timer stoppt nach der konfigurierten Wiederholungszahl;
  verschwundene Timer brechen die Runde nicht mehr ab; Boundary-Timer und
  -Messages werden persistiert.
- Engine: `fetchAndLock` überspringt Incidents (`retries <= 0`); unbehandelter
  `bpmnError` erzeugt einen Incident statt einer hängenden Task.
- Engine: parallele Call-Activities werden fortgesetzt; Spawn-Fehler werden zum
  Incident.
- Parser: leere Timer-/Script-Definitionen und ungetriggerte Catch-/Throw-/
  Boundary-Events werden abgelehnt; Flatten behält `sendTask`, Inclusive-,
  EventBased- und Intermediate-Elemente; Multi-Instance wird explizit als nicht
  unterstützt abgelehnt.
- Desktop: CSP und Bundle-Identifier `dev.bpmninja.desktop`; Dummy-Updater samt
  Fake-Pubkey entfernt.
- Server: XML-Body-Limit einheitlich 10 MiB; Compose-NATS mit Authentifizierung
  und Loopback-Ports; Deploy-Fehler liefern HTTP 500 mit Rollback.
- TypeSpec: `ApiKeyAuth` nutzt `ApiKeyLocation.header`; Clippy
  `collapsible_if` im agent-orchestrator behoben.

[Unreleased]: https://github.com/maatini/bpmninja/compare/v0.7.25...HEAD
[0.7.25]: https://github.com/maatini/bpmninja/compare/v0.7.24...v0.7.25
[0.7.24]: https://github.com/maatini/bpmninja/compare/v0.7.23...v0.7.24
[0.7.23]: https://github.com/maatini/bpmninja/compare/v0.7.22...v0.7.23
