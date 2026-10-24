# BPMNinja — Repository-Audit (Fokus: all)

## Projektzustand

Rust-Workspace mit 6 Crates, alle auf `0.7.24` (Tag `v0.7.24` = `a8f7305`). 136 Rust-Dateien / ~32.200 Zeilen, 384 Testattribute, **0** `TODO`/`FIXME`/`HACK`. Arbeitsbaum ist deutlich dirty: `improve1026.md`, `plan.md`, `reports/`, `tools/` untracked; `TASK_MEMORY.md` und `implementation_plan.md` gelöscht; zahlreiche `engine-core/src/**`-Dateien sowie `engine-server/tests/e2e_files.rs` modifiziert. Architektur ist sauber hexagonal (`port::WorkflowPersistence`, 35 async-Methoden), Token-SSOT in `ProcessInstance`, Subprozess-Flattening, NATS JetStream als Produktionsbackend. Welle 1–5, Timer-/Topic-Index, bounded Retry, `REQUIRE_NATS`, API-Key und CORS sind laut HANDOFF/`improve1026.md` umgesetzt und werden hier nicht erneut vorgeschlagen.

---

## Funde

### 1. Silent-Durability-Skip bei großen Instanzen — **hoch**
**Evidenz:** `engine-core/src/engine/persistence_ops.rs:239-257`
```rust
let estimated_size = serde_json::to_vec(&*inst).map(|v| v.len()).unwrap_or(0);
if estimated_size > crate::runtime::MAX_INSTANCE_PAYLOAD_BYTES { … return; }
```
**Wirkung:** Überschreitet eine Instanz das Payload-Limit, wird sie stillschweigend *nicht* persistiert. Der Fehler landet nur im Zähler/Log (`log_persistence_error`), der Prozess läuft im RAM weiter, `/api/health` bleibt grün. Nach einem Restart ist die Instanz weg — ohne Incident, ohne Retry-Job (der Guard kehrt vor `inline_retry!` zurück), ohne HTTP-Signal an den Aufrufer, der die Variablen geschrieben hat.
**Änderung:** Guard in einen echten Fehlerpfad wandeln: Incident auf der Instanz erzeugen (oder `InstanceState::CompletedWithError`), Readiness degradieren, Metrik `bpmn_instance_persist_oversize_total`. Alternativ Variablen-Offloading in den Object Store statt Skip.
**Aufwand:** M

### 2. Retry-Worker: Head-of-Line-Blocking und endgültiger Job-Verlust — **hoch**
**Evidenz:** `engine-core/src/engine/retry_queue.rs:135-187` (ein `recv()`-Loop, ein Job exklusiv), `:163-172` (nach `MAX_RETRIES = 50` nur `counter!` + `error!`, `break`), `:183-184` (Backoff bis `MAX_BACKOFF_MS = 60_000`)
**Wirkung:** Der Worker verarbeitet strikt seriell. Ein einziger Job gegen ein nicht erreichbares NATS blockiert die Queue bis zu 50 × 60 s ≈ 25 min+; alle anderen Save/Delete-Jobs (auch `DeleteTimer`, `SaveUserTask`) stauen sich, bis `try_send` in `persistence_ops.rs:61-71` „Queue full“ meldet und Jobs verwirft. Nach Erschöpfung gibt es kein Dead-Letter, kein Replay, keine Readiness-Verknüpfung — nur eine Zähler-Metrik.
**Änderung:** Pro Job einen eigenen Task mit `Semaphore`-Limit spawnen (max. N parallel), `error_counter`/Retry-Exhaustion in Readiness und Health-Endpoint spiegeln, verbrauchte Jobs in ein Dead-Letter-KV schreiben.
**Aufwand:** M

### 3. `NatsPersistence::connect` verschluckt alle Initialisierungsfehler — **mittel**
**Evidenz:** `persistence-nats/src/client.rs:23-29, 32-38, 52-58, 61-67, 70-76, 79-85, 88-94, 96-102, 104-110, 113-119, 122-128` — durchgehend `let _ = js.get_or_create_stream(...).await;` / `let _ = js.create_key_value(...).await;`
**Wirkung:** Fehlende Berechtigungen, ein voller NATS-Account oder ein nicht angelegter Bucket führen **nicht** zu einem Verbindungsfehler. Der Server startet „erfolgreich“ und scheitert erst beim ersten `save_instance` mit `Failed to get instances KV` — weit weg von der Ursache und mitten in einer Nutzeranfrage. Das unterläuft den Fail-fast-Anspruch von `REQUIRE_NATS`.
**Änderung:** Bucket-/Stream-Setup mit Fehlerpropagierung (nur „already exists“-Fälle tolerieren) und beim Start einmalig verifizieren (`get_key_value` auf alle Buckets).
**Aufwand:** S

### 4. History: Publish ohne Ack, Query mit 100-ms-Drain und Consumer-Leak — **mittel**
**Evidenz:** `persistence-nats/src/trait_impl.rs:514-522` (`js.publish(...).await.map_err(...)`, kein `PublishAck`-`await`), `:537-547` (pro Query `create_consumer` ohne `delete_consumer`), `:557-559` (`timeout(100ms, messages.next())` beendet die Schleife)
**Wirkung:** History-Einträge können ohne Fehler verloren gehen (Fire-and-forget), die Query liefert bei mehr als 100 ms Zustellverzögerung ein **abgeschnittenes** Ergebnis, das wie ein vollständiges aussieht. Zusätzlich häuft jeder History-Aufruf einen Consumer im `WORKFLOW_HISTORY`-Stream an → wachsende JetStream-Metadaten.
**Änderung:** `publish(...).await?.await?` (Ack abwarten) bzw. `publish_with_ack`; Consumer mit `inactive_threshold`/`delete_consumer` ephemer halten oder `get_raw_message`-Range statt Consumer; Drain über Sequenz-`last_sequence` statt Timeout.
**Aufwand:** M

### 5. Toter Token-Persistenzpfad — **mittel**
**Evidenz:** Port `engine-core/src/port/persistence.rs:91-97`, Implementierungen `engine-core/src/adapter/in_memory.rs:43-65` und `persistence-nats/src/trait_impl.rs:14-65`, Bucket `persistence-nats/src/client.rs:52-58`. Suchlauf über alle `src`-Bäume: **kein** Aufrufer außerhalb von Adapter/Test-Stub (`engine-core/src/engine/tests/stress_tests.rs:597-605`).
**Wirkung:** Tokens werden ausschließlich über `save_instance` (Instanz-JSON) persistiert. Die drei async-Methoden + der `tokens`-KV sind reine Last: Sie vergrößern den ohnehin breiten Trait (Verbesserung S2), verleiten zu falschen Annahmen über Persistenzgranularität und erzeugen einen ungenutzten Bucket.
**Änderung:** Methoden und Bucket entfernen (oder Traitsplit S2 vorziehen und dort fallen lassen); Doku in `docs/knowledge-base/modules/persistence/interfaces.md` nachziehen.
**Aufwand:** S

### 6. Restore verliert Einträge still bei Deserialisierungsfehlern — **mittel**
**Evidenz:** `persistence-nats/src/client.rs:152-169` — `serde_json::from_slice` → `tracing::warn!` und weiter; ebenso `get`-Fehler. `list_instances`/`list_definitions`/`list_user_tasks`/`list_timers` nutzen alle diesen Helfer.
**Wirkung:** Ein einziger inkompatibler/teilkorrupter Eintrag (Schema-Drift nach Upgrade, abgeschnittener Write) verschwindet beim Startup-Restore lautlos aus dem Bestand. Der Server meldet „healthy“, die Instanz ist faktisch verloren, und die KV-Zeile bleibt als Leiche liegen.
**Änderung:** Fehler pro Key sammeln und als `EngineError::PersistenceError` mit Keyliste zurückgeben (oder `RestoreReport { ok, failed }`), beim Start laut fehlschlagen bzw. Alarm-Metrik emittieren.
**Aufwand:** M

### 7. History-Recording klont die komplette Instanz pro Event — **niedrig**
**Evidenz:** `engine-core/src/engine/persistence_ops.rs:156-159` (`lk.read().await.clone()` des vollen `ProcessInstance`), `:88-122` (im Snapshot-Pfad zwei aufeinanderfolgende `read()`-Locks, `:92` und `:121`)
**Wirkung:** Jeder Token-Move mit aktivierter Persistenz kostet einen vollen Instanz-Klon (inkl. `audit_log`, Variablen, Token-Map) plus doppelte Lock-Akquise — bei `MAX_AUDIT_LOG_ENTRIES`-großen Instanzen im heißen Ausführungspfad messbar. Zudem können die zwei Locks unterschiedliche Zustände sehen (Diff gegen einen Zustand, der nicht mehr dem Snapshot entspricht).
**Änderung:** Einen `read()`-Guard halten und `calculate_diff(&snapshot_ref, &inst)` darauf arbeiten lassen; `DiffSnapshot` als leichtes Vorher-Bild statt Voll-Klon nutzen.
**Aufwand:** S

### 8. Monitoring-/Cleanup-Pfade liefern Falschdaten und verschlucken Fehler — **niedrig**
**Evidenz:** `persistence-nats/src/trait_impl.rs:654` (`created_at: Some(chrono::Utc::now())` als Fallback für KV-Einträge), `:138` (`let _ = obj_store.delete(key)` in `delete_definition`), `:345` (dito in `delete_file`); dazu `engine-core/src/engine/instance_store.rs:41-53` und `engine-core/src/engine/registry.rs:41-63` mit `#[allow(dead_code)]` entgegen CLAUDE.md:193.
**Wirkung:** Die Storage-/Bucket-Ansicht zeigt für KV-Einträge die Abrufzeit als Erstellungszeit — Dashboards und Aufräumlogik, die darauf filtern, arbeiten mit erfundenen Werten. Fehlgeschlagene Objekt-Löschungen hinterlassen verwaiste BLOBs, ohne dass jemand es erfährt. Die `dead_code`-Allows verstecken ungenutzte API-Fläche (`len`/`is_empty`/`clear`, `find_by_bpmn_id`).
**Änderung:** `BucketEntry.created_at` für KV auf `None` setzen (oder `stream.info`-Metadaten nutzen); Delete-Fehler loggen bzw. propagieren; `dead_code`-Methoden entfernen statt erlauben.
**Aufwand:** S

---

## Geprüft und verworfen

- **Auth/CORS/Upload-Limit/Rhai-Memory/`REQUIRE_NATS`** — in Welle 3 + Phase 0 umgesetzt (`engine-server/src/server/auth.rs`, `MAX_XML_BYTES`, `files.rs`-Limits), keine erneute Meldung.
- **Timer-Full-Scan / Topic-Full-Scan** — durch `timer_index.rs` und `topic_index.rs` (Hygiene `1d0b1fe` / Welle 4) adressiert.
- **XOR-Merge, Inclusive-Join, Incidents in fetch-and-lock, bpmnError, Boundary-Restore, MI-Reject** — Welle 1/2, Tests vorhanden (`xor_merge.rs`, `inclusive_join.rs`, `service_incidents.rs`, `boundary_restore.rs`).
- **`unwrap()`/`expect()` in Produktionscode** — Treffer ausschließlich in `#[cfg(test)]`-Blöcken (`runtime/instance.rs`, `history/mod.rs`, `retry_queue.rs:339-344`, `files.rs:179-198`); kein Prod-Verstoß.
- **Unbounded Retry-Queue** — behoben (`mpsc::channel`, `DEFAULT_RETRY_QUEUE_CAPACITY = 10_000`, `retry_dropped_total`).
- **`async-trait` am Persistence-Port, Tauri Edition 2024, Dependabot-/Lockfile-Themen, Desktop-Release-Assets** — laut HANDOFF bewusst offene, separat beauftragte Punkte; hier nicht als neuer Fund geführt.
- **Tech-Stack-Drift in `CLAUDE.md`** (quick-xml 0.39 vs. 0.42, Rhai 1.19 vs. 1.25.1, async-nats 0.47 vs. 0.50, Tauri 2.10) — bereits als offener Docs-Drift in HANDOFF:172/184 verbucht, daher kein Duplikat.
- **`engine_logs.jsonl` im Repo, Mutation-Badge 0.0 %, `Content-Disposition`-Sanitizing, Mega-Testdatei `unit_tests.rs`** — erledigt (Phase 0 / M5-Split / `files.rs`-Sanitizer).
- **SSE-Broadcast-Kapazität 256** (`engine/mod.rs:77`, `emit_event` mit `let _ =`) — als „by design“ in `improvements.md:170` dokumentiert; nur als Beobachtung, nicht als Fund gezählt: ein `RUST_LOG`-Warn bei `Lagged` wäre wünschenswert, ist aber ohne Lastbeleg nicht priorisierbar.
- **`get_bucket_entry_detail` mit `String::from_utf8`-Fallback „Binary Data“** (`trait_impl.rs:775-778`) — irreführend, aber durch `encoding`-Feld abgedeckt; in Fund 8 thematisch enthalten, nicht separat gezählt.