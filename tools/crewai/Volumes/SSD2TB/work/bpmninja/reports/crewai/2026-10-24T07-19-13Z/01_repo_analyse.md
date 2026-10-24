# BPMNinja — Repository-Analyse (Fokus „all")

## Projektzustand

| Merkmal | Stand |
|---|---|
| Crates | 6 im Workspace, alle `0.7.24`, Resolver-3-Workspace, `rust-toolchain.toml` = 1.99.0 |
| Umfang | 136 Rust-Dateien / 32 165 Zeilen, 384 Testattribute, 48 Doku-Dateien, 0 `TODO/FIXME/HACK` |
| HEAD | `a8f7305` (Tag `v0.7.24`), Arbeitsbaum mit Doku-/Test-Noise dirty, 1 Commit hinter `origin/main` |
| Architektur | Hexagonal: `engine-core/src/port/persistence.rs` als Port, NATS- und In-Memory-Adapter, Server/Tauri als Adapter |
| Bereits erledigt (nicht erneut vorschlagen) | Welle 1–5, Timer-Due-Index, Topic-Index, `REQUIRE_NATS` fail-closed, CORS-Allowlist, API-Key, Rhai-Caps, bounded Retry-Queue, M5-Split, `cargo deny` |

Die Engine ist handwerklich sauber geschnitten: Token-SSOT in der Instanz, sekundäre Indizes (`timer_index.rs`, `topic_index.rs`) mit dokumentierter SSOT-Regel, Persistenzfehler laufen über einen zentralen Retry-Worker. Die folgenden acht Funde sind Restlücken, die nach Welle 1–5 übrig bleiben.

---

## Funde

### F1 — `process_timers` bricht die gesamte Runde bei einem inkonsistenten Timer ab — **hoch**
**Evidenz:** `engine-core/src/engine/timer_processor.rs:31-38` fängt den Fall „Timer zwischen Sammeln und Verarbeiten verschwunden" korrekt mit `continue` ab (E4-Fix). Innerhalb derselben `for tid in expired`-Schleife propagieren danach aber `?`/`ok_or_else`: `:54` (`NoSuchInstance`), `:93` (`NoSuchInstance`, Event-Subprocess-Zweig), `:106` (`spawn_call_activity(...).await?`), `:132` (`NoSuchInstance`), `:175-181` (`InvalidDefinition("Token … not found")`), `:230-233` (`NoSuchInstance`).

**Wirkung:** Ein einziger divergierter Zustand — gelöschte Instanz, per `/move-token`/`migrate` entfernter Token, fehlgeschlagener Call-Spawn — beendet die Runde mit `Err`. Alle nachfolgenden fälligen Timer desselben Ticks (`TimerDueIndex::due_ids`, `timer_index.rs:40-43`) werden nicht mehr verarbeitet, ihr Fehler wird vom Scheduler nur geloggt (`engine-server/src/main.rs:148-152`). Der E4-Fix ist damit nur für einen von sechs Fehlerpfaden wirksam; bei wiederkehrendem Fehler entsteht ein dauerhaft stehender Timer-Stack.

**Änderung:** Pro Timer eine innere Funktion `process_one_timer(tid) -> EngineResult<()>`; in der Schleife Fehler loggen, `metrics::counter!("bpmn_timer_processing_failed_total").increment(1)` und `continue`, statt die Runde zu verlassen. Rückgabewert bleibt die Anzahl verarbeiteter Timer. **Aufwand: S**

### F2 — `RestoreStats` wird verworfen; Teil-Restore meldet sich als „healthy" — **hoch**
**Evidenz:** `engine-server/src/main.rs:90-92` ruft `StartupCoordinator::new(p_arc).restore(&mut engine, &mut xml_cache).await;` und ignoriert den Rückgabewert vollständig (kein Binding, kein Log, kein `#[must_use]`). Die Fehlerpfade in `engine-server/src/startup.rs:150-155`, `:167-171`, `:184-188` und `:201-…` wandeln jeden `list_*`-Fehler in `tracing::error!` + `0` um.

**Wirkung:** Ein KV-Ausfall beim Start (vgl. `docs/knowledge-base/modules/persistence/gotchas.md:68-70`, das genau dieses Verschlucken verbietet) führt zu einem Server, der ohne Instanzen, Tasks oder Timer läuft, aber `/api/ready` (`state.rs:105-106`) prüft nur, *ob* ein Backend hängt — nicht, ob der Restore gelang. Persistierte Vorgänge sind damit unsichtbar, obwohl sie in NATS liegen.

**Änderung:** Rückgabe annehmen, strukturiert loggen, `restore_failures` in `AppState` spiegeln und `/api/ready` bei `definitions == 0 && instances_expected` bzw. gesetztem Fehlerzähler auf 503 schalten. `RestoreStats` um `errors: usize` erweitern. **Aufwand: S**

### F3 — Retry-Queue verwirft nach 50 Versuchen ohne Dead-Letter oder Incident — **hoch**
**Evidenz:** `engine-core/src/engine/retry_queue.rs:163-173`: `Err(e) if attempt >= MAX_RETRIES` → `error_counter.fetch_add`, `bpmn_persistence_retry_exhausted_total`, `tracing::error!("PERMANENT PERSISTENCE FAILURE")`, `break`. Kein Job-Ablageort, keine Rückmeldung in die Instanz, kein Zustandswechsel.

**Wirkung:** Nach ~50 Versuchen (Backoff bis `MAX_BACKOFF_MS = 60_000`, `:27`) divergiert RAM- und NATS-Zustand endgültig: die Instanz läuft im Speicher weiter, in NATS steht der alte Wait-State. Ein Neustart „rollt zurück" und führt Arbeit doppelt aus bzw. verliert sie. M3 („Dead-Letter + Alert-Metric", `improvements.md:83`) ist nur zur Hälfte umgesetzt — die Metrik existiert, der Dead-Letter nicht.

**Änderung:** `PersistJob` bei Erschöpfung in einen Dead-Letter-Puffer (KV-Bucket `dead_letters` oder Log-Sink) schreiben und einen `Incident` auf Instanz-/Task-Ebene setzen, damit `/api/service-task/{id}/retry` greift. **Aufwand: M**

### F4 — `query_history` erzeugt pro Aufruf einen Ephemeral-Consumer mit Full-Replay — **mittel**
**Evidenz:** `persistence-nats/src/trait_impl.rs:527-547`: pro Query `stream.create_consumer(pull::Config { deliver_policy: All, ack_policy: None, filter_subject, ..Default::default() })` — kein `durable_name`, kein `inactive_threshold`, kein `consumer.delete()`. Danach `:557-558` Drain über `timeout(100ms, messages.next())` pro Message, Filterung und Pagination erst im Client (`:600-603`), Sortierung über alle Einträge (`:598`).

**Wirkung:** Drei Effekte: (a) jede History-Anfrage replayt die *komplette* Historie der Instanz ab Stream-Anfang; (b) es akkumulieren Ephemeral-Consumer im Stream bis zum Inaktivitäts-Timeout; (c) das 100-ms-Idle-Timeout ist eine Heuristik — bei langsam nachlaufender Produktion verlängert sich die Anfrage unbegrenzt, bei schneller Kette bricht sie ab, sobald eine Lücke > 100 ms auftritt. Die Pagination (`offset`/`limit`) reduziert weder Serverlast noch Latenz.

**Änderung:** Durable gefilterten Consumer einmalig anlegen und wiederverwenden, `max_messages` auf `offset + limit` setzen und den Consumer nach Erreichen des Limits abbrechen; zusätzlich Gesamt-Timeout statt Idle-Timeout. **Aufwand: M**

### F5 — Token-KV: bucket-weiter Key-Scan und im Engine-Pfad ungenutzter Port — **mittel**
**Evidenz:** `persistence-nats/src/trait_impl.rs:28-54`: `load_tokens` holt `store.keys().await` über den **gesamten** Bucket `tokens` und filtert den Instanz-Prefix clientseitig (`:39 if key.starts_with(&prefix)`). `save_token`/`delete_token` (`:14-26`, `:56-65`) lösen pro Aufruf erneut `get_key_value("tokens")` auf. Im Engine-Code existiert kein Schreibaufruf — Token leben in der Instanz-JSON (`trait_impl.rs:67-84`), der Port ist damit ein zweiter, nicht genutzter Persistenzpfad, der in `gotchas.md:37` weiter als aktiver Bucket dokumentiert ist.

**Wirkung:** Falls der Pfad genutzt wird, ist der Aufwand pro Instanz O(#Keys im Bucket), nicht O(#Tokens der Instanz); bei 10⁵ Token-Keys und Health-/Restore-Aufrufen wird das zum Vollscan. Als tote Last kostet er Pflege in beiden Adaptern (`engine-core/src/adapter/in_memory.rs`) und im Port.

**Änderung:** Entweder `load_tokens` auf `store.keys()` mit serverseitigem Filter/`history`-freiem Präfix umstellen und im Restore nutzen — oder Port-Methoden entfernen und Bucket-Liste in der KB korrigieren. **Aufwand: S**

### F6 — KB-Doku widerspricht dem Code beim History-Speicherort — **mittel**
**Evidenz:** `docs/knowledge-base/modules/persistence/gotchas.md:55-57`: „History is stored as a **KV bucket** (not a stream …). Each entry is a separate KV key. Querying history scans all entries for the instance." Code: `persistence-nats/src/trait_impl.rs:505-522` publiziert auf einen JetStream-**Stream** (`self.js.publish(subject, payload)`), `:531-533` liest über `get_stream("WORKFLOW_HISTORY")`; `:474` behandelt `WORKFLOW_HISTORY` explizit als Stream (`bucket_type: "stream"`).

**Wirkung:** Betriebsfehler sind vorprogrammiert: Wer Retention, Max-Age oder Speicherplatz nach der KB dimensioniert, konfiguriert den falschen Bucket-Typ. Die Aussage „scannt alle KV-Keys" verdeckt das reale Problem aus F4 (Consumer-Replay).

**Änderung:** Abschnitt auf Stream-Semantik umschreiben (Subjects `history.instance.{id}`, Retention, Consumer-Verhalten) und auf F4 verweisen. **Aufwand: S**

### F7 — `AppBuildConfig` bricht die zugesagte Env-Isolation — **mittel**
**Evidenz:** `engine-server/src/server/mod.rs:71-72` verspricht „explicit config (no env side effects for tests)"; `:51-53` verspricht für `build_app()` „so parallel tests are not affected by process env". In `build_app_with_config` wird dennoch `std::env::var("NATS_URL")` gelesen (`:108-109`) und in `AppState.nats_url` gespeichert; `AppBuildConfig` (`:39-46`) hat kein `nats_url`-Feld.

**Wirkung:** Prozessweite Env-Rennen bleiben genau an der Stelle bestehen, die als race-frei dokumentiert ist — Integrationstests, die `/api/info` oder Monitoring prüfen, hängen von der Shell des Testläufers ab. Die API-Konsistenz von `AppBuildConfig` ist unvollständig.

**Änderung:** `nats_url: Option<String>` in `AppBuildConfig` ergänzen, Fallback auf Env nur in `build_app_with_engine`/`main`. **Aufwand: S**

### F8 — Persistenzfehler leakt internen Text als 500, plus API-Hygiene in `InstanceStore` — **niedrig**
**Evidenz:** `engine-server/src/server/state.rs:86`: `Self::Engine(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))`. `EngineError::PersistenceError(String)` trägt den rohen NATS-Text inklusive Bucket/Subject (z. B. `trait_impl.rs:80`). Daneben API-Hygiene: `engine-core/src/engine/instance_store.rs:31` `pub async fn get(...)` ohne `.await