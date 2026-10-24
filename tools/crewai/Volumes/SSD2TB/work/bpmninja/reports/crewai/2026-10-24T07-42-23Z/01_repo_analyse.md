# BPMNinja — Architektur-Audit „all“

## Projektzustand (belegt)

Monorepo, Workspace-Version **0.7.24**, 6 Rust-Crates (`engine-core`, `bpmn-parser`, `persistence-nats`, `persistence-memory`, `agent-orchestrator`, `engine-server`) plus `desktop-tauri`. 136 Rust-Dateien / 32 165 Zeilen / 384 Testattribute / 48 Doku-Dateien, **0 TODO/FIXME/HACK** im Rust-Code. Hexagonale Struktur ist real: `engine-core/src/port/persistence.rs` (34 async-Methoden) als einziger IO-Vertrag, `adapter/in_memory.rs` und `persistence-nats/src/trait_impl.rs` als Implementierungen, `runtime/*` als Domänen-SSOT (`tokens: HashMap<Uuid, Token>` in `instance_store.rs`). Sekundärindizes existieren für Service-Topics (`engine/topic_index.rs`) und Timer (`engine/timer_index.rs`), Retry-Queue ist bounded (`retry_queue.rs:36`, Drop-Metrik `retry_queue.rs:8-9`). Welle 1–5, Phase 0, Hygiene und Release sind laut `HANDOFF.md`/`improvements.md` abgeschlossen; das Audit bestätigt das im Code (Auth-Middleware, CORS-Allowlist, `REQUIRE_NATS`-Default `true`, Upload-/XML-Limits, Testdatei-Split).

Der Zustand ist damit deutlich über dem in `improvements.md` beschriebenen 0.7.18-Audit. Die verbleibenden Schwachstellen liegen **nicht** in der BPMN-Semantik, sondern in den Rändern: Shutdown-Durability, Bootstrap-Fehlerbehandlung und zwei Hotpaths (History, Fetch-and-Lock).

---

## Funde

### 1 — Shutdown „flush“ leert die Retry-Queue nicht (hoch)
**Evidenz:** `engine-core/src/engine/retry_queue.rs:135-139` — der Worker bricht bei `PersistJob::Shutdown` sofort aus der `while let Some(job) = rx.recv()`-Schleife aus, ohne den Kanal zu drainen. Dagegen behaupten `engine-core/src/engine/mod.rs:136-138` („signals the background persistence retry worker to **flush** and exit“) und `engine-server/src/main.rs:183-184` („Flushing persistence queues…“) das Gegenteil.
**Wirkung:** Alle noch im bounded Channel (10 000 Slots) liegenden Persist-Jobs gehen bei jedem geordneten Shutdown verloren — genau die Jobs, die wegen NATS-Störung überhaupt dort gelandet sind. Der Log sagt „geflusht“, die Realität ist Drop. Das ist der einzige Pfad, auf dem ein *graceful* Shutdown Daten verliert.
**Änderung:** `Shutdown` in ein Drain überführen (`while let Ok(job) = rx.try_recv()` mit begrenztem Zeitbudget, Restfehler zählen) oder `Shutdown` erst nach leerem Kanal akzeptieren; Logtext an tatsächliches Verhalten binden.
**Aufwand:** S

### 2 — NATS-Bootstrap verschluckt jeden Fehler; „fail-closed“ gilt nur für `connect` (hoch)
**Evidenz:** `persistence-nats/src/client.rs:22-29, 32-49, 51-128` — sämtliche `get_or_create_stream`- und `create_key_value`/`create_object_store`-Aufrufe sind `let _ = …`. Nur `async_nats::connect` (Zeile 15) propagiert. `engine-server/src/main.rs:82-116` behandelt entsprechend ausschließlich den Connect-Fehler als `REQUIRE_NATS`-Verletzung; `REQUIRE_NATS`-Default `true` in `engine-server/src/server/state.rs:117-131`.
**Wirkung:** Fehlende Rechte, erschöpfte `max_streams`/`max_buckets` oder ein abweichend konfigurierter Account erzeugen einen Server, der `/api/ready` erfüllt und „Connected to NATS“ loggt (`main.rs:86`), aber bei jedem `save_instance` in `PersistenceError` läuft. Diese Fehler landen über die Inline-Retries in der Retry-Queue — und werden durch Fund 1 beim Shutdown verworfen. Die Fail-closed-Zusage ist damit nur halb eingelöst.
**Änderung:** Bootstrap-Fehler propagieren, außer „existiert bereits“ (`BucketAlreadyExists`/`StreamAlreadyExists`); Ergebnis in `StorageInfo`/Readiness spiegeln.
**Aufwand:** S

### 3 — `process_timers` löscht vor der Verarbeitung und kann danach mit `Err` abbrechen (hoch)
**Evidenz:** `engine-core/src/engine/timer_processor.rs:32` entfernt Timer aus SSOT **und** Due-Index, danach setzen `:54-56` (`NoSuchInstance`), `:116` (`NoSuchDefinition`), `:175-181` („Token … not found“) und `:237` (`run_instance_batch`) per `?` ab. `remove_persisted_timer` kommt erst bei `:236`, nach der Token-Mutation.
**Wirkung:** Bei einem Fehler nach Zeile 32 ist der Timer aus dem Speicher verschwunden, in NATS aber noch vorhanden (Delete übersprungen) — nach Restart feuert er erneut auf einen Token-Zustand, der bereits mutiert wurde. Zusätzlich bricht `?` die `for tid in expired`-Schleife ab: alle weiteren fälligen Timer dieser Runde werden stillschweigend auf den nächsten Tick (1 s, `main.rs:132-160`) verschoben. Ein einziges inkonsistentes Token kann so die Timer-Runde serialisieren.
**Änderung:** Fehler pro Timer isolieren (`match` statt `?`), Incident/Dead-Letter statt stiller Rückkehr; Timer erst nach erfolgreichem Batch entfernen oder bei Fehler wieder einfügen und `persist_timer` erzwingen.
**Aufwand:** M

### 4 — `query_history` erzeugt pro Request einen JetStream-Consumer und scannt vollständig (mittel)
**Evidenz:** `persistence-nats/src/trait_impl.rs:537-547` legt pro Aufruf `stream.create_consumer(... DeliverPolicy::All, AckPolicy::None, filter_subject)` an, ohne ihn zu löschen; `:556-559` drainiert über einen **100-ms-Timeout** und bricht dann ab; `:600-603` wendet `offset`/`limit` erst nach dem vollständigen Lesen und Sortieren an.
**Wirkung:** Jeder `GET /api/instances/{id}/history` hinterlässt einen Consumer (sichtbar in `StorageInfo.consumers`, `:502`) und liest die komplette Event-Historie der Instanz in den Heap. Bei langen Instanzen ist die Antwort zudem still abgeschnitten, wenn 100 ms nicht reichen — ohne Fehler, ohne Cursor. Das ist der teuerste Lesepfad der API.
**Änderung:** Einmaliger, benannter Consumer pro `instance_id` mit `deliver_policy: ByStartTime`/`LastPerSubject` und Wiederverwendung (`get_or_create_consumer`), Server-seitige `limit`/`offset` im Consumer, Timeout durch „bis `pending == 0`“ ersetzen.
**Aufwand:** M

### 5 — Fetch-and-Lock pollt den gesamten Topic-Index pro Worker (mittel)
**Evidenz:** `engine-core/src/engine/topic_index.rs:33-46` liefert **alle** Task-IDs der Topics als `Vec` (mit `HashSet::insert`-Dedup, also voller Kopie); `engine-core/src/engine/service_task.rs:57-94` iteriert diese Liste, prüft pro Eintrag `lock_expiration` und springt bei „locked“ per `continue` weiter; `engine-server/src/server/tasks.rs:85-107` wiederholt den kompletten Aufruf alle 500 ms bis zu 30 s.
**Wirkung:** Die Kosten eines leeren Long-Polls sind O(#Tasks im Topic), nicht O(#freie Tasks); N Worker erzeugen N × 2 × #Tasks Lookups pro Sekunde. Der Topic-Index hat den Scan aus `improvements.md` (M4) nur von „alle Tasks“ auf „alle Tasks dieser Topics“ reduziert. Das ist der verbleibende Performance-Hotspot.
**Änderung:** Index um eine „unlocked“-Menge pro Topic erweitern (beim Lock entfernen, bei Ablauf/Retry zurückführen) oder `candidate_ids` lazy über einen Iterator ohne Vollkopie; Long-Poll über Notify/`broadcast` statt 500-ms-Polling.
**Aufwand:** M

### 6 — Multi-Topic-Fetch ignoriert `lockDuration` aller Topics außer dem ersten (mittel)
**Evidenz:** `engine-server/src/server/tasks.rs:67-78` — `topics` wird auf Namen reduziert, `lock_duration` kommt aus `payload.topics.first()`, `unwrap_or(30_000)`.
**Wirkung:** Ein Worker, der `{a: 60s}` und `{b: 5s}` anfragt, erhält für `b` stillschweigend 60 s. Camunda/OpenAPI-Vertrag ist per-Topic; `PendingServiceTask` trägt die Dauer nicht, also ist die Abweichung im Ergebnis unsichtbar. Führt zu überlangen Locks und verspäteten Retries.
**Änderung:** `lock_duration` je Topic in `fetch_and_lock_service_tasks` durchreichen (Signatur `&[(String, i64)]`) und pro Task anwenden.
**Aufwand:** S

### 7 — `InstanceStore` ist durchgehend `async`, ohne je zu awaiten (niedrig)
**Evidenz:** `engine-core/src/engine/instance_store.rs:26-63` — `insert`, `get`, `remove`, `len`, `is_empty`, `clear` haben keinen einzigen `.await` im Rumpf, sind aber `pub async fn`; Folge sind hunderte `.await` an Aufrufstellen (z. B. `timer_processor.rs:18, 44, 127`).
**Wirkung:** Kein Laufzeitfehler, aber irreführende API: Die Signaturen suggerieren IO/Lock-Wartezeit und verhindern die Nutzung in synchronen Kontexten (z. B. `Drop`, `restore_*`-Pfade, `timer_index`-Helfer). Jede spätere echte Async-Anbindung wäre ein Breaking Change über den halben Core.
**Änderung:** Auf `pub fn` zurückbauen (die Methoden sind bereits nicht-blockierend); `all()` separat lassen.
**Aufwand:** S

### 8 — Timer-Due-Index ist ein globaler `std::sync::Mutex`, entgegen der „lock-free“-Zusage (niedrig)
**Evidenz:** `engine-core/src/engine/timer_index.rs:14-16, 25-29` (`Mutex<BTreeMap<(DateTime<Utc>, Uuid>, ()>>`), `:40-42` alloziert pro `process_timers`-Tick ein vollständiges `Vec` aller fälligen IDs; `docs/knowledge-base/index.md:40` bewirbt „Wait-state queues use `DashMap` (lock-free sharding)“.
**Wirkung:** Der Timer-Pfad serialisiert Insert/Remove/Due-Scan über einen globalen Mutex und ist damit der einzige Nicht-DashMap-Hotpath im Core — bei hoher Timer-Dichte (Cron-Start-Events) ein Contention-Punkt, der in der KB nicht auftaucht. Die Zusage ist an dieser Stelle falsch dokumentiert.
**Änderung:** Entweder `tokio::sync::RwLock`/`DashMap<DateTime, HashSet<Uuid>>` mit sortierter Nachbarstruktur, mindestens aber `due_ids` als inkrementelles Pop aus dem `BTreeMap` statt Vollkopie; KB-Aussage präzisieren.
**Aufwand:** M

---

## Geprüft, aber verworfen (kein neuer Vorschlag)

- **Auth, CORS-Allowlist, Upload-/XML-Limits** — umgesetzt (`engine-server/src/server/auth.rs`, `state.rs:151-159`, `mod.rs:111-148`); Welle 3, nicht erneut gemeldet.
- **`REQUIRE_NATS`-Default fail-closed** — korrekt (`state.rs:117-131`, Tests `:133-155`); nur die Bootstrap-Lücke (Fund 2) ist neu.
- **Rhai `max_memory`** — in `improvements.md` Phase 0 als erledigt geführt; nicht neu aufgerollt.
- **Bounded Retry-Queue + Drop-Metrik (M3)** — vorhanden (`retry_queue.rs:20-45`); Kritik betrifft nur den Shutdown-Pfad.
- **Timer-Due-Index und Topic-Index als solche (M4/Hygiene `1d0b1fe`)** — existieren; nur Mutex-/Allokations- bzw. Poll-Semantik beanstandet.
- **M5 `unit_tests.rs`-Split** — erledigt, 19 Testmodule unter `engine-core/src/engine/tests/`.
- **`unwrap`/`expect` im Produktionspfad** — alle Treffer liegen in `#[cfg(test)]`-Modulen oder `#[test]`-Funktionen; kein Fund.
- **`all()`-Vollklon als R1** — API-seitig paginiert, daher verworfen.
- **`async-trait` am Persistence-Port (S2)** und **Tauri-Crate/npm-Mismatch** — in `HANDOFF.md` als eigene, bewusst offene Punkte geführt; nicht redundant gemeldet.