use tokio::sync::RwLock;
pub(crate) mod auth;
pub(crate) mod deploy;
pub(crate) mod events;
pub(crate) mod files;
pub(crate) mod history;
pub(crate) mod instances;
pub(crate) mod logs;
pub(crate) mod messages;
pub(crate) mod monitoring;
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

pub use state::{MAX_XML_BYTES, default_cors_origins, require_nats_from_env};

/// Optional overrides for app construction (primarily for tests).
///
/// When a field is `None`, the value is taken from the environment.
/// Explicit values avoid process-wide env races in parallel integration tests.
///
/// `api_key`: `None` = from `BPMNINJA_API_KEY`, `Some(None)` = auth off,
/// `Some(Some(k))` = auth on with that key.
#[derive(Debug, Clone, Default)]
pub struct AppBuildConfig {
    pub require_nats: Option<bool>,
    pub max_upload_bytes: Option<usize>,
    pub max_xml_bytes: Option<usize>,
    pub api_key: Option<Option<String>>,
    pub cors_origins: Option<Vec<String>>,
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
            cors_origins: Some(default_cors_origins()),
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
    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());

    let api_key = match config.api_key {
        Some(explicit) => explicit.and_then(|k| state::parse_api_key(Some(&k))),
        None => state::api_key_from_env(),
    };
    let cors_origins = match config.cors_origins {
        Some(origins) if !origins.is_empty() => origins,
        Some(_) => state::default_cors_origins(),
        None => state::cors_origins_from_env(),
    };
    let max_xml_bytes = config
        .max_xml_bytes
        .unwrap_or_else(state::max_xml_bytes_from_env);

    let state = Arc::new(AppState {
        engine,
        persistence,
        deployed_xml: Arc::new(RwLock::new(xml_cache)),
        nats_url,
        log_buffer,
        require_nats: config
            .require_nats
            .unwrap_or_else(state::require_nats_from_env),
        max_upload_bytes: config
            .max_upload_bytes
            .unwrap_or_else(state::max_upload_bytes_from_env),
        max_xml_bytes,
        api_key,
        cors_origins: cors_origins.clone(),
    });

    tracing::info!(
        auth_enabled = state.api_key.is_some(),
        cors_origins = ?state.cors_origins,
        max_xml_bytes,
        "HTTP security config"
    );

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
