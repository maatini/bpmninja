use tokio::sync::RwLock;
pub(crate) mod auth;
pub(crate) mod deploy;
pub(crate) mod events;
pub(crate) mod files;
pub(crate) mod history;
pub(crate) mod instances;
pub(crate) mod logs;
pub(crate) mod messages;
pub(crate) mod model_check;
pub(crate) mod monitoring;
pub(crate) mod signals;
pub(crate) mod state;
pub(crate) mod tasks;
pub(crate) mod timers;

use crate::log_buffer::LogBuffer;
use axum::{
    Router,
    http::{HeaderName, HeaderValue, Method, header},
    middleware,
    routing::{delete, get, post, put},
};
use engine_core::WorkflowEngine;
use engine_core::persistence::WorkflowPersistence;
use metrics_exporter_prometheus::PrometheusHandle;
use state::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

pub use state::{
    MAX_XML_BYTES, StorageState, StorageStatus, default_cors_origins, require_nats_from_env,
};

/// Optional overrides for app construction (primarily for tests).
///
/// When a field is `None`, the value is taken from the environment.
/// Explicit values avoid process-wide env races in parallel integration tests.
///
/// `api_key`: `None` = from `BPMNINJA_API_KEY`, `Some(None)` = auth explicitly
/// off, `Some(Some(k))` = auth on with that key.
/// `worker_api_key`: same scheme for `BPMNINJA_WORKER_API_KEY`.
/// `bind_addr`: host only (`BIND_ADDR`/`BIND_ADDRESS`); drives the fail-closed
/// decision when no key is configured.
#[derive(Debug, Clone, Default)]
pub struct AppBuildConfig {
    pub require_nats: Option<bool>,
    pub max_upload_bytes: Option<usize>,
    pub max_xml_bytes: Option<usize>,
    pub api_key: Option<Option<String>>,
    pub worker_api_key: Option<Option<String>>,
    pub bind_addr: Option<String>,
    pub allow_unauthenticated: Option<bool>,
    pub cors_origins: Option<Vec<String>>,
    /// Ergebnis des NATS-Bootstraps; `None` leitet den Status aus `persistence` ab.
    pub storage_status: Option<StorageStatus>,
    /// Anzeige-URL für `/api/info`; `None` = aus `NATS_URL`. Wird vor der
    /// Ausgabe über [`crate::redact::redact_nats_url`] maskiert.
    pub nats_url: Option<String>,
    /// `None` liest `MODEL_CHECK` (Default `off`).
    pub model_check: Option<model_check::ModelCheckMode>,
    /// `None` liest `MODEL_CHECK_TIMEOUT_MS` (Default 2000). Kein positiver
    /// Integer fällt auf den Default zurück.
    pub model_check_timeout_ms: Option<u64>,
}

/// Builds the Axum router with all routes and middleware.
///
/// Exposed as `pub` so integration tests can create the app without
/// starting a full server binary. Forces `require_nats = false` and auth off
/// so parallel tests are not affected by process env. CORS is the default
/// allowlist (not `Any`).
pub fn build_app() -> Router {
    build_app_with_config(
        Arc::new(WorkflowEngine::new()),
        None,
        HashMap::new(),
        None,
        Arc::new(LogBuffer::new()),
        AppBuildConfig {
            require_nats: Some(false),
            max_upload_bytes: None,
            max_xml_bytes: Some(MAX_XML_BYTES),
            api_key: Some(None),
            worker_api_key: Some(None),
            bind_addr: Some(state::DEFAULT_BIND_ADDR.to_string()),
            allow_unauthenticated: None,
            cors_origins: Some(default_cors_origins()),
            storage_status: None,
            nats_url: None,
            // Tests must not follow a process-wide `MODEL_CHECK` setting.
            model_check: Some(model_check::ModelCheckMode::Off),
            model_check_timeout_ms: None,
        },
    )
}

/// Like [`build_app`], but with explicit config (no env side effects for tests).
pub fn build_app_with_options(config: AppBuildConfig) -> Router {
    build_app_with_config(
        Arc::new(WorkflowEngine::new()),
        None,
        HashMap::new(),
        None,
        Arc::new(LogBuffer::new()),
        config,
    )
}

pub fn build_app_with_engine(
    engine: Arc<WorkflowEngine>,
    persistence: Option<Arc<dyn WorkflowPersistence>>,
    xml_cache: HashMap<String, String>,
    prometheus_handle: Option<PrometheusHandle>,
    log_buffer: Arc<LogBuffer>,
) -> Router {
    build_app_with_config(
        engine,
        persistence,
        xml_cache,
        prometheus_handle,
        log_buffer,
        AppBuildConfig::default(),
    )
}

pub fn build_app_with_config(
    engine: Arc<WorkflowEngine>,
    persistence: Option<Arc<dyn WorkflowPersistence>>,
    xml_cache: HashMap<String, String>,
    prometheus_handle: Option<PrometheusHandle>,
    log_buffer: Arc<LogBuffer>,
    config: AppBuildConfig,
) -> Router {
    let nats_url = config.nats_url.unwrap_or_else(|| {
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string())
    });
    // Credentials (Userinfo/Token-Query) dürfen die API nicht verlassen.
    let nats_url = crate::redact::redact_nats_url(&nats_url);

    // `Some(None)` is the explicit "auth off" switch (tests / dev helpers).
    let explicit_auth_off = matches!(config.api_key, Some(None));
    let api_key = match config.api_key {
        Some(explicit) => explicit.and_then(|k| state::parse_api_key(Some(&k))),
        None => state::api_key_from_env(),
    };
    let worker_api_key = match config.worker_api_key {
        Some(explicit) => explicit.and_then(|k| state::parse_api_key(Some(&k))),
        None => state::worker_api_key_from_env(),
    };
    let bind_addr = config.bind_addr.unwrap_or_else(state::bind_addr_from_env);
    let allow_unauthenticated = config
        .allow_unauthenticated
        .unwrap_or_else(state::allow_unauthenticated_from_env);
    let auth_required = state::resolve_auth_required(
        api_key.is_some() || worker_api_key.is_some(),
        explicit_auth_off,
        allow_unauthenticated,
        &bind_addr,
    );
    let cors_origins = match config.cors_origins {
        Some(origins) if !origins.is_empty() => origins,
        Some(_) => state::default_cors_origins(),
        None => state::cors_origins_from_env(),
    };
    let max_xml_bytes = config
        .max_xml_bytes
        .unwrap_or_else(state::max_xml_bytes_from_env);

    let storage_status = config
        .storage_status
        .unwrap_or_else(|| StorageStatus::inferred(persistence.is_some()));

    let state = Arc::new(AppState {
        engine,
        persistence,
        deployed_xml: Arc::new(RwLock::new(xml_cache)),
        nats_url,
        log_buffer,
        storage_status,
        require_nats: config
            .require_nats
            .unwrap_or_else(state::require_nats_from_env),
        max_upload_bytes: config
            .max_upload_bytes
            .unwrap_or_else(state::max_upload_bytes_from_env),
        max_xml_bytes,
        api_key,
        worker_api_key,
        auth_required,
        cors_origins: cors_origins.clone(),
        model_checker: model_check::ModelChecker::new(
            config
                .model_check
                .unwrap_or_else(model_check::ModelCheckMode::from_env),
            config
                .model_check_timeout_ms
                .unwrap_or_else(model_check::timeout_ms_from_env),
        ),
    });

    tracing::info!(
        auth_required = state.auth_required,
        admin_key = state.api_key.is_some(),
        worker_key = state.worker_api_key.is_some(),
        bind_addr = %bind_addr,
        allow_unauthenticated,
        cors_origins = ?state.cors_origins,
        max_xml_bytes,
        model_check = ?state.model_checker.mode,
        model_check_timeout_ms = state.model_checker.timeout_ms(),
        model_check_compiled = cfg!(feature = "model-check"),
        "HTTP security config"
    );

    #[cfg(not(feature = "model-check"))]
    if state.model_checker.mode != model_check::ModelCheckMode::Off {
        tracing::warn!(
            mode = ?state.model_checker.mode,
            "MODEL_CHECK is set, but this binary was built without the model-check feature"
        );
    }

    if !state.auth_required && !state::is_loopback_bind(&bind_addr) {
        tracing::warn!(
            bind_addr = %bind_addr,
            "API authentication is disabled on a non-loopback bind (explicit opt-out)"
        );
    }

    let cors = build_cors_layer(&cors_origins);

    let mut router = Router::new()
        .route("/api/deploy", post(deploy::deploy_definition))
        .route("/api/start", post(instances::start_instance))
        .route("/api/start/latest", post(instances::start_instance_latest))
        .route("/api/start/timer", post(instances::start_timer_instance))
        .route("/api/tasks", get(tasks::get_tasks))
        .route("/api/complete/{id}", post(tasks::complete_task))
        .route("/api/instances", get(instances::list_instances))
        .route(
            "/api/instances/{id}",
            get(instances::get_instance).delete(instances::delete_instance),
        )
        .route("/api/definitions", get(deploy::list_definitions))
        .route("/api/definitions/{id}/xml", get(deploy::get_definition_xml))
        .route("/api/definitions/{id}", delete(deploy::delete_definition))
        .route(
            "/api/definitions/bpmn/{bpmn_id}",
            delete(deploy::delete_all_definitions),
        )
        .route(
            "/api/instances/{id}/move-token",
            post(instances::move_token),
        )
        .route(
            "/api/instances/{id}/migrate",
            post(instances::migrate_instance),
        )
        .route(
            "/api/instances/{id}/suspend",
            post(instances::suspend_instance),
        )
        .route(
            "/api/instances/{id}/resume",
            post(instances::resume_instance),
        )
        .route(
            "/api/instances/{id}/variables",
            put(instances::update_instance_variables),
        )
        .route(
            "/api/instances/{id}/files/{var_name}",
            post(files::upload_instance_file)
                .get(files::get_instance_file)
                .delete(files::delete_instance_file),
        )
        .route(
            "/api/instances/{id}/history",
            get(history::get_instance_history),
        )
        .route(
            "/api/instances/{id}/history/{event_id}",
            get(history::get_instance_history_entry),
        )
        .route(
            "/api/instances/{id}/incidents",
            get(instances::get_instance_incidents),
        )
        .route(
            "/api/instances/{id}/incidents/{incident_id}/resolve",
            post(instances::resolve_instance_incident),
        )
        .route("/api/info", get(monitoring::get_backend_info))
        .route("/api/monitoring", get(monitoring::get_monitoring_data))
        .route(
            "/api/monitoring/buckets/{bucket}/entries",
            get(monitoring::get_bucket_entries),
        )
        .route(
            "/api/monitoring/buckets/{bucket}/entries/{key}",
            get(monitoring::get_bucket_entry_detail),
        )
        // Historical (completed) instances
        .route(
            "/api/history/instances",
            get(history::list_completed_instances),
        )
        .route(
            "/api/history/instances/{id}",
            get(history::get_completed_instance),
        )
        // Phase 1 endpoints
        .route("/api/message", post(messages::correlate_message))
        .route("/api/timers", get(timers::get_pending_timers))
        .route("/api/timers/process", post(timers::process_timers))
        .route("/api/messages", get(messages::get_pending_messages))
        .route("/api/signal", post(signals::correlate_signal))
        .route("/api/signals", get(signals::get_pending_signals))
        // Service Task endpoints
        .route("/api/service-tasks", get(tasks::get_service_tasks))
        .route(
            "/api/service-task/fetchAndLock",
            post(tasks::fetch_and_lock_service_tasks),
        )
        .route(
            "/api/service-task/{id}/complete",
            post(tasks::complete_service_task),
        )
        .route(
            "/api/service-task/{id}/failure",
            post(tasks::fail_service_task),
        )
        .route("/api/service-task/{id}/retry", post(tasks::retry_incident))
        .route(
            "/api/service-task/{id}/resolve",
            post(tasks::resolve_incident),
        )
        .route(
            "/api/service-task/{id}/extendLock",
            post(tasks::extend_lock),
        )
        .route("/api/service-task/{id}/bpmnError", post(tasks::bpmn_error))
        .route("/api/logs", get(logs::get_logs))
        .route("/api/events", get(events::engine_events))
        .route("/api/health", get(|| async { axum::http::StatusCode::OK }))
        .route("/api/ready", get(monitoring::ready_endpoint))
        .layer(middleware::from_fn(
            crate::observability::http_metrics_middleware,
        ))
        .layer(axum::extract::DefaultBodyLimit::max(max_xml_bytes))
        .with_state(state.clone());

    // Mount /metrics endpoint (separate state: PrometheusHandle)
    if let Some(handle) = prometheus_handle {
        let metrics_router = Router::new()
            .route("/metrics", get(crate::observability::metrics_handler))
            .with_state(handle);
        router = router.merge(metrics_router);
    }

    // Auth + CORS wrap the merged router so `/metrics` is also protected.
    // CORS is outermost so preflight OPTIONS is answered before the API-key gate.
    router
        .layer(middleware::from_fn_with_state(state, auth::auth_middleware))
        .layer(cors)
}

fn build_cors_layer(origins: &[String]) -> CorsLayer {
    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ];
    let headers = [
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        HeaderName::from_static("x-api-key"),
    ];

    if origins.iter().any(|o| o == "*") {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(methods)
            .allow_headers(headers);
    }

    let mut values: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|origin| match origin.parse() {
            Ok(v) => Some(v),
            Err(_) => {
                tracing::warn!(origin, "Ignoring invalid CORS origin");
                None
            }
        })
        .collect();
    if values.is_empty() {
        values = state::DEFAULT_CORS_ORIGINS
            .iter()
            .filter_map(|o| (*o).parse().ok())
            .collect();
    }

    CorsLayer::new()
        .allow_origin(values)
        .allow_methods(methods)
        .allow_headers(headers)
}
