use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_nats::Client;
use async_nats::jetstream::context::{
    CreateKeyValueError, CreateStreamError, CreateStreamErrorKind, GetStreamError,
    GetStreamErrorKind,
};
use async_nats::jetstream::object_store::{Config as ObjectStoreConfig, ObjectStore};
use async_nats::jetstream::stream::Stream;
use async_nats::jetstream::{self, Context, ErrorCode, kv, stream::Config as StreamConfig};
use engine_core::error::{EngineError, EngineResult};
use futures::StreamExt;
use tokio::sync::OnceCell;

/// Stream für `history.instance.*` (Event-Sourcing der Instanz-Historie).
const HISTORY_STREAM: &str = "WORKFLOW_HISTORY";
const HISTORY_SUBJECTS: &[&str] = &["history.instance.*"];

/// KV-Buckets, die beim Bootstrap vorhanden sein müssen.
const KV_BUCKETS: &[(&str, &str)] = &[
    ("tokens", "Token state per process instance (JSON)"),
    ("instances", "ProcessInstance state (JSON)"),
    ("definitions", "ProcessDefinition metadata (JSON)"),
    ("user_tasks", "PendingUserTask objects (JSON)"),
    ("service_tasks", "PendingServiceTask objects (JSON)"),
    ("timers", "PendingTimer objects (JSON)"),
    ("messages", "PendingMessageCatch objects (JSON)"),
    (
        "pending_signal_catches",
        "PendingSignalCatch objects (JSON)",
    ),
    (
        "pending_conditional_catches",
        "PendingConditionalCatch objects (JSON)",
    ),
    (
        "history_instances",
        "Archived completed ProcessInstance objects (JSON)",
    ),
];

/// Object-Store-Buckets, die beim Bootstrap vorhanden sein müssen.
const OBJECT_STORES: &[(&str, &str)] = &[
    ("bpmn_xml", "Original BPMN 2.0 XML artifacts"),
    (
        "instance_files",
        "Binary file attachments for process instance variables",
    ),
];

const EVENT_RETENTION_ENV: &str = "BPMNINJA_EVENT_RETENTION_DAYS";
const HISTORY_RETENTION_ENV: &str = "BPMNINJA_HISTORY_RETENTION_DAYS";
const STREAM_MAX_BYTES_ENV: &str = "BPMNINJA_STREAM_MAX_BYTES";
const STREAM_REPLICAS_ENV: &str = "BPMNINJA_STREAM_REPLICAS";

/// Default-Aufbewahrung für den Event-Stream (7 Tage).
const DEFAULT_EVENT_RETENTION_DAYS: u64 = 7;
/// Default-Aufbewahrung für den History-Stream (30 Tage).
const DEFAULT_HISTORY_RETENTION_DAYS: u64 = 30;
/// Default-Größenlimit pro Stream (1 GiB).
const DEFAULT_STREAM_MAX_BYTES: i64 = 1024 * 1024 * 1024;
/// Default-Replikationsfaktor (Einzelknoten).
const DEFAULT_STREAM_REPLICAS: usize = 1;
/// JetStream erlaubt maximal 5 Replicas pro Stream.
const MAX_STREAM_REPLICAS: usize = 5;
const SECONDS_PER_DAY: u64 = 86_400;

/// Retention- und Replikationsvorgaben für die Workflow-Streams.
///
/// Für Tage und Bytes bedeutet `0` "unbegrenzt" (NATS-Semantik), für Replicas
/// ist `0` ungültig. Die Werte kommen aus `BPMNINJA_*`-Umgebungsvariablen;
/// ungültige Werte fallen mit Warnung auf die Defaults zurück.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RetentionConfig {
    event_retention_days: u64,
    history_retention_days: u64,
    max_bytes: i64,
    replicas: usize,
}

impl RetentionConfig {
    fn from_env() -> Self {
        Self::parse(
            std::env::var(EVENT_RETENTION_ENV).ok().as_deref(),
            std::env::var(HISTORY_RETENTION_ENV).ok().as_deref(),
            std::env::var(STREAM_MAX_BYTES_ENV).ok().as_deref(),
            std::env::var(STREAM_REPLICAS_ENV).ok().as_deref(),
        )
    }

    fn parse(
        event_days: Option<&str>,
        history_days: Option<&str>,
        max_bytes: Option<&str>,
        replicas: Option<&str>,
    ) -> Self {
        Self {
            event_retention_days: parse_days(
                event_days,
                DEFAULT_EVENT_RETENTION_DAYS,
                EVENT_RETENTION_ENV,
            ),
            history_retention_days: parse_days(
                history_days,
                DEFAULT_HISTORY_RETENTION_DAYS,
                HISTORY_RETENTION_ENV,
            ),
            max_bytes: parse_max_bytes(max_bytes),
            replicas: parse_replicas(replicas),
        }
    }

    fn event_max_age(&self) -> Duration {
        Duration::from_secs(self.event_retention_days.saturating_mul(SECONDS_PER_DAY))
    }

    fn history_max_age(&self) -> Duration {
        Duration::from_secs(self.history_retention_days.saturating_mul(SECONDS_PER_DAY))
    }
}

fn parse_days(raw: Option<&str>, default_days: u64, var: &str) -> u64 {
    let Some(raw) = raw else {
        return default_days;
    };
    match raw.trim().parse::<u64>() {
        Ok(days) => days,
        Err(_) => {
            tracing::warn!("Ungültiger Wert für {var}='{raw}' — nutze Default {default_days} Tage");
            default_days
        }
    }
}

fn parse_max_bytes(raw: Option<&str>) -> i64 {
    let Some(raw) = raw else {
        return DEFAULT_STREAM_MAX_BYTES;
    };
    match raw.trim().parse::<i64>() {
        Ok(bytes) if bytes >= 0 => bytes,
        Ok(_) => {
            tracing::warn!(
                "Negativer Wert für {STREAM_MAX_BYTES_ENV}='{raw}' — nutze Default {DEFAULT_STREAM_MAX_BYTES} Bytes"
            );
            DEFAULT_STREAM_MAX_BYTES
        }
        Err(_) => {
            tracing::warn!(
                "Ungültiger Wert für {STREAM_MAX_BYTES_ENV}='{raw}' — nutze Default {DEFAULT_STREAM_MAX_BYTES} Bytes"
            );
            DEFAULT_STREAM_MAX_BYTES
        }
    }
}

fn parse_replicas(raw: Option<&str>) -> usize {
    let Some(raw) = raw else {
        return DEFAULT_STREAM_REPLICAS;
    };
    match raw.trim().parse::<usize>() {
        Ok(0) | Err(_) => {
            tracing::warn!(
                "Ungültiger Wert für {STREAM_REPLICAS_ENV}='{raw}' — nutze Default {DEFAULT_STREAM_REPLICAS}"
            );
            DEFAULT_STREAM_REPLICAS
        }
        Ok(replicas) if replicas > MAX_STREAM_REPLICAS => {
            tracing::warn!(
                "{STREAM_REPLICAS_ENV}='{raw}' überschreitet das NATS-Maximum — begrenze auf {MAX_STREAM_REPLICAS}"
            );
            MAX_STREAM_REPLICAS
        }
        Ok(replicas) => replicas,
    }
}

/// Stream-Konfiguration (Name, Subjects, Retention) für die Workflow-Streams.
fn workflow_stream_config(
    name: &str,
    subjects: &[&str],
    max_age: Duration,
    retention: RetentionConfig,
) -> StreamConfig {
    StreamConfig {
        name: name.to_string(),
        subjects: subjects.iter().map(|s| (*s).to_string()).collect(),
        max_age,
        max_bytes: retention.max_bytes,
        num_replicas: retention.replicas,
        ..Default::default()
    }
}

/// Übernimmt die Retention-Felder aus `desired` in eine bestehende
/// Stream-Config. Subjects, Storage, Discard usw. bleiben unverändert.
///
/// Rückgabe: `true`, wenn sich mindestens ein Feld geändert hat.
fn apply_retention(config: &mut StreamConfig, desired: &StreamConfig) -> bool {
    let changed = config.max_age != desired.max_age
        || config.max_bytes != desired.max_bytes
        || config.num_replicas != desired.num_replicas;
    if changed {
        config.max_age = desired.max_age;
        config.max_bytes = desired.max_bytes;
        config.num_replicas = desired.num_replicas;
    }
    changed
}

/// Ergebnis des JetStream-Bootstraps in [`NatsPersistence::connect`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootstrapReport {
    created: Vec<String>,
    existing: Vec<String>,
    updated: Vec<String>,
}

impl BootstrapReport {
    /// In diesem Prozess neu angelegte Ressourcen.
    pub fn created(&self) -> &[String] {
        &self.created
    }

    /// Bereits vorhandene Ressourcen.
    pub fn existing(&self) -> &[String] {
        &self.existing
    }

    /// Vorhandene Streams, deren Retention angepasst wurde.
    pub fn updated(&self) -> &[String] {
        &self.updated
    }

    /// Einzeilige Zusammenfassung für Startup-Log und Health-Status.
    pub fn summary(&self) -> String {
        format!(
            "{} vorhanden, {} angelegt, {} aktualisiert",
            self.existing.len(),
            self.created.len(),
            self.updated.len()
        )
    }
}

/// Cached JetStream handles for the buckets created at connect.
///
/// Without the cache every operation paid an extra JetStream API round trip
/// (`get_key_value` / `get_object_store`) before the actual read or write.
/// A failed lookup is not stored, so the next call retries. Names that are not
/// in the bootstrap list are fetched every time and never cached.
struct BucketHandles<T> {
    cells: HashMap<&'static str, OnceCell<Arc<T>>>,
}

impl<T> BucketHandles<T> {
    fn new(names: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            cells: names
                .into_iter()
                .map(|name| (name, OnceCell::new()))
                .collect(),
        }
    }

    async fn get_or_fetch<E, F, Fut>(&self, bucket: &str, fetch: F) -> Result<Arc<T>, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        match self.cells.get(bucket) {
            Some(cell) => cell
                .get_or_try_init(|| async { fetch().await.map(Arc::new) })
                .await
                .cloned(),
            None => fetch().await.map(Arc::new),
        }
    }

    #[cfg(test)]
    fn is_cached(&self, bucket: &str) -> bool {
        self.cells.get(bucket).and_then(OnceCell::get).is_some()
    }
}

struct HandleCache {
    kv: BucketHandles<kv::Store>,
    object_stores: BucketHandles<ObjectStore>,
}

impl HandleCache {
    fn new() -> Self {
        Self {
            kv: BucketHandles::new(KV_BUCKETS.iter().map(|(name, _)| *name)),
            object_stores: BucketHandles::new(OBJECT_STORES.iter().map(|(name, _)| *name)),
        }
    }
}

#[derive(Clone)]
pub struct NatsPersistence {
    pub(crate) client: Client,
    pub(crate) js: Context,
    bootstrap: BootstrapReport,
    handles: Arc<HandleCache>,
}

impl NatsPersistence {
    /// Verbindet zu NATS und stellt alle JetStream-Ressourcen sicher.
    ///
    /// Fail-closed: Jeder Bootstrap-Fehler außer "existiert bereits" wird
    /// propagiert. Der Aufrufer entscheidet (REQUIRE_NATS), ob der Start
    /// abgebrochen oder bewusst ohne Persistenz weitergearbeitet wird.
    pub async fn connect(url: &str, stream_name: &str) -> EngineResult<Self> {
        let client = async_nats::connect(url).await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to connect to NATS: {}", e))
        })?;

        let js = jetstream::new(client.clone());
        let retention = RetentionConfig::from_env();
        let mut report = BootstrapReport::default();

        let events_subject = format!("{stream_name}.*");
        let events = workflow_stream_config(
            stream_name,
            &[events_subject.as_str()],
            retention.event_max_age(),
            retention,
        );
        ensure_stream(&js, &events, &mut report).await?;

        for (bucket, description) in OBJECT_STORES {
            ensure_object_store(&js, bucket, description, &mut report).await?;
        }

        for (bucket, description) in KV_BUCKETS {
            ensure_kv_bucket(&js, bucket, description, &mut report).await?;
        }

        let history = workflow_stream_config(
            HISTORY_STREAM,
            HISTORY_SUBJECTS,
            retention.history_max_age(),
            retention,
        );
        ensure_stream(&js, &history, &mut report).await?;

        tracing::info!(
            "JetStream bootstrap verified: {} (event retention {} d, history retention {} d, max_bytes {}, replicas {})",
            report.summary(),
            retention.event_retention_days,
            retention.history_retention_days,
            retention.max_bytes,
            retention.replicas
        );

        Ok(Self {
            client,
            js,
            bootstrap: report,
            handles: Arc::new(HandleCache::new()),
        })
    }

    /// KV-Handle für `bucket`. Bekannte Buckets werden nach dem ersten Treffer gecacht.
    pub(crate) async fn kv(&self, bucket: &str) -> EngineResult<Arc<kv::Store>> {
        self.handles
            .kv
            .get_or_fetch(bucket, || async {
                self.js.get_key_value(bucket).await.map_err(|e| {
                    EngineError::PersistenceError(format!("Failed to get {bucket} KV: {e}"))
                })
            })
            .await
    }

    /// Object-Store-Handle für `bucket`. Bekannte Stores werden nach dem ersten Treffer gecacht.
    pub(crate) async fn object_store(&self, bucket: &str) -> EngineResult<Arc<ObjectStore>> {
        self.handles
            .object_stores
            .get_or_fetch(bucket, || async {
                self.js.get_object_store(bucket).await.map_err(|e| {
                    EngineError::PersistenceError(format!(
                        "Failed to get {bucket} Object Store: {e}"
                    ))
                })
            })
            .await
    }

    #[cfg(test)]
    pub(crate) fn kv_handle_cached(&self, bucket: &str) -> bool {
        self.handles.kv.is_cached(bucket)
    }

    #[cfg(test)]
    pub(crate) fn object_store_handle_cached(&self, bucket: &str) -> bool {
        self.handles.object_stores.is_cached(bucket)
    }

    /// Ergebnis des Bootstraps (angelegte, vorhandene, aktualisierte Ressourcen).
    pub fn bootstrap_report(&self) -> &BootstrapReport {
        &self.bootstrap
    }

    /// Gibt einen Klon des JetStream-Kontexts zurück, z. B. für den Log-Sink.
    pub fn jetstream(&self) -> async_nats::jetstream::context::Context {
        self.js.clone()
    }

    pub(crate) async fn list_kv_entries<T: serde::de::DeserializeOwned>(
        &self,
        bucket: &str,
        entity_name: &str,
    ) -> EngineResult<Vec<T>> {
        let store = self.kv(bucket).await?;

        // keys() Err is a persistence failure, not an empty bucket — must propagate.
        let mut keys = store.keys().await.map_err(|e| {
            EngineError::PersistenceError(format!("Failed to list {entity_name} keys: {}", e))
        })?;

        let mut entries = Vec::new();
        while let Some(key_result) = keys.next().await {
            match key_result {
                Ok(key) => match store.get(&key).await {
                    Ok(Some(entry)) => match serde_json::from_slice::<T>(&entry) {
                        Ok(item) => entries.push(item),
                        Err(e) => {
                            tracing::warn!("Failed to deserialize {entity_name} '{}': {}", key, e)
                        }
                    },
                    Ok(None) => {}
                    Err(e) => tracing::warn!("Failed to get {entity_name} '{key}': {}", e),
                },
                Err(e) => tracing::warn!("Failed to stream {entity_name} key: {}", e),
            }
        }

        Ok(entries)
    }
}

/// Stellt einen Stream sicher und gleicht bei bestehenden Streams die
/// Retention-Felder ab.
async fn ensure_stream(
    js: &Context,
    desired: &StreamConfig,
    report: &mut BootstrapReport,
) -> EngineResult<()> {
    let name = desired.name.as_str();
    match js.get_stream(name).await {
        Ok(stream) => reconcile_stream(js, stream, desired, report).await,
        Err(err) if is_stream_not_found(&err) => match js.create_stream(desired.clone()).await {
            Ok(_) => {
                report.created.push(format!("stream:{name}"));
                Ok(())
            }
            // Race mit einem anderen Serverstart: Stream wurde inzwischen angelegt.
            Err(err) if is_stream_exists(&err) => {
                let stream = js
                    .get_stream(name)
                    .await
                    .map_err(|e| bootstrap_error(&format!("stream '{name}'"), &e))?;
                reconcile_stream(js, stream, desired, report).await
            }
            Err(err) => Err(bootstrap_error(&format!("stream '{name}'"), &err)),
        },
        Err(err) => Err(bootstrap_error(&format!("stream '{name}'"), &err)),
    }
}

async fn reconcile_stream(
    js: &Context,
    stream: Stream,
    desired: &StreamConfig,
    report: &mut BootstrapReport,
) -> EngineResult<()> {
    let name = desired.name.as_str();
    let mut config = stream.cached_info().config.clone();
    if apply_retention(&mut config, desired) {
        js.update_stream(config)
            .await
            .map_err(|e| bootstrap_error(&format!("stream '{name}'"), &e))?;
        tracing::info!(
            "Stream '{name}': Retention aktualisiert (max_age {:?}, max_bytes {}, replicas {})",
            desired.max_age,
            desired.max_bytes,
            desired.num_replicas
        );
        report.updated.push(format!("stream:{name}"));
    } else {
        report.existing.push(format!("stream:{name}"));
    }
    Ok(())
}

async fn ensure_kv_bucket(
    js: &Context,
    bucket: &str,
    description: &str,
    report: &mut BootstrapReport,
) -> EngineResult<()> {
    let config = async_nats::jetstream::kv::Config {
        bucket: bucket.to_string(),
        description: description.to_string(),
        ..Default::default()
    };

    match js.create_key_value(config).await {
        Ok(_) => {
            report.created.push(format!("kv:{bucket}"));
            Ok(())
        }
        Err(err) if create_conflict(&err) => {
            // Validieren, dass der bestehende Bucket tatsächlich als KV nutzbar ist.
            js.get_key_value(bucket)
                .await
                .map_err(|e| bootstrap_error(&format!("KV bucket '{bucket}'"), &e))?;
            report.existing.push(format!("kv:{bucket}"));
            Ok(())
        }
        Err(err) => Err(bootstrap_error(&format!("KV bucket '{bucket}'"), &err)),
    }
}

async fn ensure_object_store(
    js: &Context,
    bucket: &str,
    description: &str,
    report: &mut BootstrapReport,
) -> EngineResult<()> {
    let config = ObjectStoreConfig {
        bucket: bucket.to_string(),
        description: Some(description.to_string()),
        ..Default::default()
    };

    match js.create_object_store(config).await {
        Ok(_) => {
            report.created.push(format!("object_store:{bucket}"));
            Ok(())
        }
        Err(err) if create_conflict(&err) => {
            // Validieren, dass der bestehende Bucket als Object Store nutzbar ist.
            js.get_object_store(bucket)
                .await
                .map_err(|e| bootstrap_error(&format!("object store '{bucket}'"), &e))?;
            report.existing.push(format!("object_store:{bucket}"));
            Ok(())
        }
        Err(err) => Err(bootstrap_error(&format!("object store '{bucket}'"), &err)),
    }
}

fn bootstrap_error(resource: &str, err: &(impl std::fmt::Display + ?Sized)) -> EngineError {
    EngineError::PersistenceError(format!("JetStream bootstrap failed for {resource}: {err}"))
}

fn is_stream_not_found(err: &GetStreamError) -> bool {
    match err.kind() {
        GetStreamErrorKind::JetStream(js) => {
            js.code() == 404 || js.kind() == ErrorCode::STREAM_NOT_FOUND
        }
        _ => false,
    }
}

fn is_stream_exists(err: &CreateStreamError) -> bool {
    matches!(
        err.kind(),
        CreateStreamErrorKind::JetStream(js) if js.kind() == ErrorCode::STREAM_NAME_EXIST
    )
}

/// `create_key_value`/`create_object_store` verpacken den zugrunde liegenden
/// `CreateStreamError` als Source. "Existiert bereits" ist der einzige Fehler,
/// der beim Bootstrap als Erfolg gilt.
fn create_conflict(err: &CreateKeyValueError) -> bool {
    let mut source = std::error::Error::source(err);
    while let Some(cause) = source {
        if let Some(stream_err) = cause.downcast_ref::<CreateStreamError>() {
            return is_stream_exists(stream_err);
        }
        source = cause.source();
    }
    false
}

#[cfg(test)]
mod retention_tests {
    use super::*;
    use async_nats::jetstream::context::CreateKeyValueErrorKind;

    #[test]
    fn defaults_set_bounded_retention() {
        let config = RetentionConfig::parse(None, None, None, None);
        assert_eq!(config.event_retention_days, DEFAULT_EVENT_RETENTION_DAYS);
        assert_eq!(
            config.history_retention_days,
            DEFAULT_HISTORY_RETENTION_DAYS
        );
        assert_eq!(config.max_bytes, DEFAULT_STREAM_MAX_BYTES);
        assert_eq!(config.replicas, DEFAULT_STREAM_REPLICAS);
        assert_eq!(
            config.event_max_age(),
            Duration::from_secs(DEFAULT_EVENT_RETENTION_DAYS * SECONDS_PER_DAY)
        );
        assert_eq!(
            config.history_max_age(),
            Duration::from_secs(DEFAULT_HISTORY_RETENTION_DAYS * SECONDS_PER_DAY)
        );
    }

    #[test]
    fn env_values_override_defaults() {
        let config = RetentionConfig::parse(Some("3"), Some("14"), Some("4096"), Some("3"));
        assert_eq!(config.event_retention_days, 3);
        assert_eq!(config.history_retention_days, 14);
        assert_eq!(config.max_bytes, 4096);
        assert_eq!(config.replicas, 3);
        assert_eq!(
            config.event_max_age(),
            Duration::from_secs(3 * SECONDS_PER_DAY)
        );
    }

    #[test]
    fn zero_means_unlimited() {
        let config = RetentionConfig::parse(Some("0"), Some("0"), Some("0"), None);
        assert_eq!(config.event_max_age(), Duration::ZERO);
        assert_eq!(config.history_max_age(), Duration::ZERO);
        assert_eq!(config.max_bytes, 0);
    }

    #[test]
    fn invalid_values_fall_back_to_defaults() {
        let config = RetentionConfig::parse(Some("abc"), Some("-1"), Some("-5"), Some("0"));
        assert_eq!(config.event_retention_days, DEFAULT_EVENT_RETENTION_DAYS);
        assert_eq!(
            config.history_retention_days,
            DEFAULT_HISTORY_RETENTION_DAYS
        );
        assert_eq!(config.max_bytes, DEFAULT_STREAM_MAX_BYTES);
        assert_eq!(config.replicas, DEFAULT_STREAM_REPLICAS);
    }

    #[test]
    fn replicas_are_capped_at_nats_limit() {
        let config = RetentionConfig::parse(None, None, None, Some("9"));
        assert_eq!(config.replicas, MAX_STREAM_REPLICAS);
    }

    #[test]
    fn stream_config_carries_retention_limits() {
        let retention = RetentionConfig::parse(Some("1"), Some("2"), Some("8192"), Some("2"));
        let config = workflow_stream_config(
            "WORKFLOW_EVENTS",
            &["WORKFLOW_EVENTS.*"],
            retention.event_max_age(),
            retention,
        );

        assert_eq!(config.name, "WORKFLOW_EVENTS");
        assert_eq!(config.subjects, vec!["WORKFLOW_EVENTS.*".to_string()]);
        assert_eq!(config.max_age, Duration::from_secs(SECONDS_PER_DAY));
        assert_eq!(config.max_bytes, 8192);
        assert_eq!(config.num_replicas, 2);
    }

    #[test]
    fn already_exists_conflict_is_recognized() {
        let js_error: async_nats::jetstream::Error = serde_json::from_str(
            r#"{"code":400,"err_code":10058,"description":"stream name already in use"}"#,
        )
        .expect("valid JetStream error JSON");
        let stream_err = CreateStreamError::new(CreateStreamErrorKind::JetStream(js_error));
        let kv_err =
            CreateKeyValueError::with_source(CreateKeyValueErrorKind::BucketCreate, stream_err);

        assert!(create_conflict(&kv_err));
    }

    #[test]
    fn other_bootstrap_errors_are_not_conflicts() {
        let js_error: async_nats::jetstream::Error = serde_json::from_str(
            r#"{"code":400,"err_code":10039,"description":"permissions violation"}"#,
        )
        .expect("valid JetStream error JSON");
        let stream_err = CreateStreamError::new(CreateStreamErrorKind::JetStream(js_error));
        let permission_err =
            CreateKeyValueError::with_source(CreateKeyValueErrorKind::BucketCreate, stream_err);
        assert!(!create_conflict(&permission_err));

        let timed_out = CreateKeyValueError::new(CreateKeyValueErrorKind::TimedOut);
        assert!(!create_conflict(&timed_out));
    }

    #[test]
    fn retention_update_preserves_foreign_stream_fields() {
        let retention = RetentionConfig::parse(None, None, Some("1024"), Some("1"));
        let desired = workflow_stream_config(
            "WORKFLOW_HISTORY",
            HISTORY_SUBJECTS,
            retention.history_max_age(),
            retention,
        );
        let mut existing = StreamConfig {
            name: "WORKFLOW_HISTORY".to_string(),
            subjects: vec!["custom.subject.>".to_string()],
            allow_direct: true,
            max_age: Duration::ZERO,
            max_bytes: 0,
            num_replicas: 3,
            ..Default::default()
        };

        assert!(apply_retention(&mut existing, &desired));
        assert_eq!(existing.subjects, vec!["custom.subject.>".to_string()]);
        assert!(existing.allow_direct);
        assert_eq!(existing.max_age, retention.history_max_age());
        assert_eq!(existing.max_bytes, 1024);
        assert_eq!(existing.num_replicas, 1);

        // Zweiter Lauf ist idempotent.
        assert!(!apply_retention(&mut existing, &desired));
    }
}

#[cfg(test)]
mod handle_cache_tests {
    use super::BucketHandles;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test(flavor = "current_thread")]
    async fn known_bucket_is_fetched_once() {
        let cache = BucketHandles::<u32>::new(["instances"]);
        let calls = AtomicUsize::new(0);

        let first = cache
            .get_or_fetch("instances", || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(7)
            })
            .await
            .unwrap();
        let second = cache
            .get_or_fetch("instances", || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(9)
            })
            .await
            .unwrap();

        assert_eq!(*first, 7);
        assert_eq!(*second, 7);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(cache.is_cached("instances"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn failed_lookup_is_not_cached() {
        let cache = BucketHandles::<u32>::new(["instances"]);
        let calls = AtomicUsize::new(0);

        let err = cache
            .get_or_fetch("instances", || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Err::<u32, &str>("down")
            })
            .await;
        assert_eq!(err, Err("down"));
        assert!(!cache.is_cached("instances"));

        let ok = cache
            .get_or_fetch("instances", || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, &str>(1)
            })
            .await
            .unwrap();
        assert_eq!(*ok, 1);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(cache.is_cached("instances"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn unknown_bucket_is_fetched_every_time() {
        let cache = BucketHandles::<u32>::new(["instances"]);
        let calls = AtomicUsize::new(0);

        for expected in [1u32, 2] {
            let value = cache
                .get_or_fetch("other", || async {
                    let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
                    Ok::<u32, ()>(n as u32)
                })
                .await
                .unwrap();
            assert_eq!(*value, expected);
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(!cache.is_cached("other"));
        assert!(!cache.is_cached("instances"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn concurrent_waiters_share_one_fetch() {
        let cache = Arc::new(BucketHandles::<u32>::new(["instances"]));
        let calls = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());

        let spawn = |cache: Arc<BucketHandles<u32>>, calls: Arc<AtomicUsize>| {
            let started = Arc::clone(&started);
            let release = Arc::clone(&release);
            tokio::spawn(async move {
                cache
                    .get_or_fetch("instances", || {
                        let calls = Arc::clone(&calls);
                        let started = Arc::clone(&started);
                        let release = Arc::clone(&release);
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            started.notify_one();
                            release.notified().await;
                            Ok::<_, ()>(4)
                        }
                    })
                    .await
                    .unwrap()
            })
        };

        let first = spawn(Arc::clone(&cache), Arc::clone(&calls));
        started.notified().await;
        let second = spawn(Arc::clone(&cache), Arc::clone(&calls));
        for _ in 0..32 {
            tokio::task::yield_now().await;
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        release.notify_waiters();
        assert_eq!(*first.await.unwrap(), 4);
        assert_eq!(*second.await.unwrap(), 4);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
