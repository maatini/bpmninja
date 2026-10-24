# BPMNinja — Betriebs-, Security- und Qualitätsrisiken (Workspace 0.7.24)

Methodik: Code-Durchsicht der Crates + CI-Workflows, `cargo metadata`, cargo-deny-/RustSec-Konfiguration, Abgleich der Behauptungen in `quality-metrics.json`/`HANDOFF.md` gegen den tatsächlichen Code. Keine Datei geändert. Unsicherheiten sind markiert.

---

### 1 — Fail-closed gilt nur für `connect`: Bootstrap-Fehler werden verschluckt, Readiness lügt (hoch)
**Evidenz:** `persistence-nats/src/client.rs:23-29,32-49,51-128` — jeder `get_or_create_stream`/`create_key_value`/`create_object_store` ist `let _ = …`. Nur `connect` (`:15`) propagiert. `engine-server/src/main.rs:82-116` wertet deshalb ausschließlich den Connect-Fehler als `REQUIRE_NATS`-Verletzung; `monitoring.rs:28-48` prüft nur `get_storage_info()`.
**Szenario:** NATS-Account ohne JetStream-Rechte, erschöpftes `max_streams`/`max_buckets` oder fremder Account → Server startet, loggt „Connected“ (`main.rs:86`), `/api/ready` liefert 200, aber jeder `save_instance` scheitert. Die Zusage „fail-closed“ ist damit nur halb eingelöst; ein Orchestrator routet Traffic auf eine Instanz ohne Persistenz.
**Maßnahme:** Bootstrap-Fehler außer „existiert bereits“ propagieren und in `StorageInfo`/`/api/ready` spiegeln; Readiness erst nach erfolgreichem KV-/Stream-Ping.

### 2 — Graceful Shutdown verwirft den Persistenz-Backlog (hoch)
**Evidenz:** `engine-core/src/engine/retry_queue.rs:135-139` bricht bei `PersistJob::Shutdown` sofort aus `while let Some(job) = rx.recv()` aus, ohne zu drainen; dagegen `main.rs:183-184` („Flushing persistence queues…“) und `engine/mod.rs` („flush and exit“).
**Szenario:** NATS-Störung füllt die Queue (`:36`, 10 000 Slots); ein Rollout/Deploy fährt geordnet herunter → alle noch nicht abgearbeiteten Jobs sind weg, das Log behauptet das Gegenteil. Genau der Störfall, für den die Queue existiert, verliert Daten.
**Maßnahme:** Drain mit Zeitbudget (`try_recv`-Schleife), `Shutdown` erst nach leerem Kanal akzeptieren, verbleibende Jobs zählen/loggen; Logtext an reales Verhalten binden.

### 3 — JetStream-Streams ohne Retention, Größen- und Replikationslimits (hoch)
**Evidenz:** `client.rs:23-29` (`WORKFLOW_EVENTS`) und `:122-128` (`WORKFLOW_HISTORY`) nutzen `..Default::default()` — kein `max_age`, `max_bytes`, `max_messages`, keine Replicas. Kontrast: der Log-Stream setzt `max_messages` explizit (`engine-server/src/log_nats.rs:36-40`).
**Szenario:** Die History wächst monoton mit jedem Token-/Timer-Ereignis; der Account läuft voll, JetStream blockiert Publishes → alle Writes scheitern reihenweise (sichtbar nur als `persistence_errors` in `/api/monitoring`). Zusätzlich: ohne Replikation ist ein NATS-Node-Verlust ein Datenverlust. *Unsicherheit:* „Default 1 Replica“ ist NATS-Herstellerverhalten, nicht im Repo belegt — die fehlende explizite Konfiguration ist belegt.
**Maßnahme:** `max_age`/`max_bytes`/`num_replicas` beim Anlegen festschreiben, Alert auf `storage_bytes`/`persistence_errors`, Retention-Policy dokumentieren.

### 4 — `fetchAndLock` ohne Eingabegrenzen: Remote-Panic und Lock-Amplification (hoch)
**Evidenz:** `engine-server/src/server/tasks.rs:73-78` übernimmt `lockDuration` des Clients ungeprüft; `state.rs:258-263` rechnet nur `saturating_add`, ohne Obergrenze; `engine-core/src/engine/service_task.rs:83` bildet `now + TimeDelta::seconds(lock_duration)`. `payload.max_tasks` wird ungefiltert durchgereicht (`tasks.rs:91`). Gegenbeispiele für gute Praxis im selben Server: `instances.rs:110` (`clamp(1,1000)`), `logs.rs:27` (`.min(5_000)`).
**Szenario:** `lockDuration: 9223372036854775000` → `DateTime + TimeDelta` läuft in den chrono-Overflow (Add panikt bei Overflow, dokumentiert) mitten im `DashMap::get_mut`-Guard; die restlichen Tasks der Runde werden nicht persistiert (`service_task.rs:96-98` erst nach der Schleife). `maxTasks: 10^12` lockt alle freien Tasks aller Topics in einem Request und erzeugt einen Persist-Burst.
**Maßnahme:** `lockDuration` auf z. B. 1 s–1 h clampen, `maxTasks` ≤ 100, `topics`-Anzahl begrenzen; Persist pro gelocktem Task oder in `Drop`-sicherem Pfad.

### 5 — Timer-Pfad löscht vor der Verarbeitung und bricht die Runde mit `?` ab (hoch)
**Evidenz:** `engine-core/src/engine/timer_processor.rs:32` entfernt Timer aus SSOT und Due-Index; danach `?` bei `:54` (`NoSuchInstance`) und weiteren Stellen; `remove_persisted_timer` folgt erst am Ende. `main.rs:144-152` loggt `Err` nur per `tracing::error`, ohne Metrik/Incident.
**Szenario:** Ein inkonsistenter Timer (verwaiste Instanz) entfernt sich aus dem Speicher, löscht den NATS-Key nicht und bricht per `?` alle weiteren fälligen Timer dieser Runde ab — bei 1-s-Tick (`main.rs:132-160`) dauerhaft, ohne Alert. Nach Restart feuert der persistierte Timer erneut auf einen bereits mutierten Token (Doppelausführung).
**Maßnahme:** Fehler pro Timer isolieren (`match`), Dead-Letter/Incident statt stillem Rückkehren, Entfernen erst nach erfolgreichem Batch, Fehlerzähler + Metrik für die Scheduler-Schleife.

### 6 — Default-Deployment ist unauthentifiziert: ein Key, keine Drosselung, kein Auth-Audit (hoch)
**Evidenz:** `state.rs:194-208` (leer/unset → `None` → Auth aus), `auth.rs:22-24` (No-op ohne Key), `docker-compose.yaml:17-28` (Port `8081:8081` auf allen Interfaces, `BPMNINJA_API_KEY` auskommentiert), `main.rs:163-169` (Bind-Default `0.0.0.0`).
**Szenario:** Im LAN-Compose-Betrieb kann jeder `/api/instances/{id}` löschen, `/api/deploy`, `/api/timers/process` und `/api/service-task/{id}/resolve` aufrufen. Fehlgeschlagene Auth-Versuche werden nicht geloggt (keine Zeile in `auth.rs`), es gibt keine Rate-Limitierung und keine Rollentrennung — ein Worker-Key ist ein Admin-Key.
**Maßnahme:** Auth fail-closed erzwingen, wenn nicht an Loopback gebunden; getrennte Worker-/Admin-Credentials; Fehlversuche loggen und zählen; Compose mit Pflicht-Key.

### 7 — HTTP-Rand ohne Timeouts/Concurrency-Limit; Upload-Limit greift erst nach dem Puffern (mittel)
**Evidenz:** `server/mod.rs:255-259` — nur Metrics-Layer und `DefaultBodyLimit::max(max_xml_bytes)`; kein `TimeoutLayer`, kein `ConcurrencyLimitLayer` im gesamten `engine-server` (Suche ohne Treffer). `files.rs:69-80` liest `field.bytes().await` vollständig ein und prüft `max_upload_bytes` erst danach (effektiv bis 10 MiB pro Request).
**Szenario:** N parallele Uploads/Slow-Body-Requests halten Speicher und Verbindungen; zusammen mit 30-s-Long-Polls (`tasks.rs:79-107`) und fehlender Auth (Fund 6) genügt bescheidene Parallelität, um den Prozess zu drücken. Fehlertexte geben zudem Backend-Details preis (`files.rs:94,123,162` mit `{:?}`).
**Maßnahme:** `TimeoutLayer` (z. B. 30 s, Long-Poll separat), `ConcurrencyLimitLayer`/Rate-Limit pro IP, Multipart-Feld streamend mit Limit, interne Fehler nur serverseitig loggen.

### 8 — Qualitätsgates ohne Durchsetzung: Mutation, Fuzz, Persistenz-Tests, Advisory-Scope (mittel)
**Evidenz:** `docs/quality-metrics.json:7-12` (68,2 %, 177 überlebende Mutanten), `mutation-tests.yml:43` `continue-on-error: true` und `:151-152` nur `::warning::` — der Score kann nie rot werden. `ci.yml:20-63` (fmt/clippy/test/external-client/TypeSpec/cargo-deny/E2E) enthält keinen Fuzz-Job; `fuzz.yml`/`fuzz-testing.yml` existieren nicht (weitere Workflow-Dateien konnte ich nicht enumerieren — Hidden-Verzeichnis wird vom Tool übersprungen). `fuzz/Cargo.toml:72` deklariert `[workspace]`, die 9 Targets sind damit von `cargo test/clippy --workspace` nicht erfasst. `persistence-nats/src/tests.rs:12-32` überspringt die einzigen 4 Integrationstests des dauerhaften Adapters still, wenn `CI`/`REQUIRE_NATS_TESTS` fehlt. `deny.toml:18` `unmaintained = "workspace"` prüft laut cargo-deny-Doku nur *direkte* Workspace-Abhängigkeiten (Default wäre `all`), `:17` `yanked = "warn"` blockt nichts.
**Szenario:** Regressionen in Persistenz, Incident-Recovery und Timer laufen grün durch PRs; die Fuzz-Targets verrotten unbemerkt; unmaintained transitive Crates (z. B. in der async-nats-/rhai-Kette) werden nie gemeldet.
**Maßnahme:** Mindest-Mutationsscore (≥ 75 %, keine neuen überlebenden Mutanten in `engine/`) gaten, nächtlichen Fuzz-Job (`cargo fuzz build` + 10 min pro Target) ergänzen, `REQUIRE_NATS_TESTS=true` lokal/devbox, `unmaintained = "all"`, `yanked = "deny"`.

---

## Geprüft, ohne Befund
`unwrap`/`expect`/`panic!` ausschließlich in `#[cfg(test)]`-Modulen (`files.rs:172-209`, `boundary.rs:74-117`); kein `unsafe`, kein `todo!`/`unimplemented!` in Produktionspfaden. Bucket-Namen sind allowlist-geprüft (`persistence-nats/src/trait_impl.rs:615-628`). Header-Injection/Path-Traversal beim Download ist gefiltert (`files.rs:14-33`). API-Key-Vergleich inhaltlich konstantzeitig (`state.rs:211-222`). `REQUIRE_NATS`-Default `true` inkl. Tests (`state.rs:117-155`). CORS ohne `Any` per Default (`state.rs:164-169`, `mod.rs:276-319`). Paginierung außer bei `fetchAndLock` geclampt. Priorisierung: Funde 1–3 sind Datenverlust-/Verfügbarkeitsrisiken im Störfall, 4–6 sind direkt angreifbar, 7–8 sind Rand- und Nachweisrisiken.