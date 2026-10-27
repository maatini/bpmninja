use crate::server::model_check::{ModelCheckMode, ModelCheckStatus};
use crate::server::state::{AppError, AppState, MAX_XML_BYTES, parse_uuid};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use engine_core::error::EngineError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const _: () = assert!(MAX_XML_BYTES == 10 * 1024 * 1024);

#[derive(Serialize, Deserialize)]
pub(crate) struct DeployRequest {
    pub xml: String,
    pub name: String,
}

#[derive(Serialize)]
pub(crate) struct DeployResponse {
    pub definition_key: String,
    pub version: i32,
    /// Deploy-time model checking result (absent when `MODEL_CHECK=off`).
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub model_check: Option<crate::server::model_check::ModelCheckReport>,
}

pub(crate) async fn deploy_definition(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeployRequest>,
) -> Result<Json<DeployResponse>, AppError> {
    // Shared with DefaultBodyLimit: default `MAX_XML_BYTES` (10 MiB); env may override.
    if payload.xml.len() > state.max_xml_bytes {
        return Err(AppError::BadRequest(format!(
            "XML too large: {} bytes (max {})",
            payload.xml.len(),
            state.max_xml_bytes
        )));
    }
    let engine = &state.engine;
    let def = bpmn_parser::parse_bpmn_xml(&payload.xml)
        .map_err(|e| AppError::BadRequest(format!("Invalid BPMN XML: {e:?}")))?;

    let model_check = state.model_checker.check(&payload.xml).await;
    if let Some(report) = &model_check
        && report.status == ModelCheckStatus::Violations
    {
        if state.model_checker.mode == ModelCheckMode::Enforce {
            return Err(AppError::BadRequest(format!(
                "Model check failed: {}",
                report.summary()
            )));
        }
        tracing::warn!("Model check for '{}': {}", payload.name, report.summary());
    }
    let (key, version) = engine.deploy_definition(def).await;
    let key_str = key.to_string();

    if let Some(persistence) = &state.persistence
        && let Err(e) = persistence.save_bpmn_xml(&key_str, &payload.xml).await
    {
        tracing::error!("Failed to save BPMN XML to persistence layer: {e:?}");
        if let Err(rollback_err) = engine.delete_definition(key, true).await {
            tracing::error!(
                "Failed to rollback in-memory definition after XML save failure: {rollback_err:?}"
            );
        }
        return Err(AppError::Engine(EngineError::PersistenceError(format!(
            "Failed to save BPMN XML: {e}"
        ))));
    }
    state
        .deployed_xml
        .write()
        .await
        .insert(key_str.clone(), payload.xml.clone());

    Ok(Json(DeployResponse {
        definition_key: key_str,
        version,
        model_check,
    }))
}

#[derive(Serialize)]
pub(crate) struct CollectionProcessResult {
    pub process_id: String,
    pub definition_key: String,
    pub version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_check: Option<crate::server::model_check::ModelCheckReport>,
}

#[derive(Serialize)]
pub(crate) struct CollectionDeployResponse {
    pub collection_id: String,
    pub processes: Vec<CollectionProcessResult>,
    /// Metadata is stored; automatic internal message delivery is not enabled.
    pub collaborations: Vec<bpmn_parser::Collaboration>,
}

pub(crate) async fn deploy_collection(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeployRequest>,
) -> Result<Json<CollectionDeployResponse>, AppError> {
    if payload.xml.len() > state.max_xml_bytes {
        return Err(AppError::BadRequest(format!(
            "XML too large (max {})",
            state.max_xml_bytes
        )));
    }
    let document = bpmn_parser::parse_bpmn_collaboration(&payload.xml)
        .map_err(|e| AppError::BadRequest(format!("Invalid BPMN XML: {e:?}")))?;
    if document.processes.is_empty() {
        return Err(AppError::BadRequest(
            "Collection has no executable processes".into(),
        ));
    }
    // Validate every selected pool before calling the publishing path.
    let mut reports = Vec::new();
    for process in &document.processes {
        let isolated = isolate_process(&payload.xml, &process.id).map_err(AppError::BadRequest)?;
        let report = state.model_checker.check(&isolated).await;
        if let Some(report) = &report
            && report.status == ModelCheckStatus::Violations
            && state.model_checker.mode == ModelCheckMode::Enforce
        {
            return Err(AppError::BadRequest(format!(
                "Model check failed for '{}': {}",
                process.id,
                report.summary()
            )));
        }
        reports.push((process.id.clone(), report));
    }
    let metadata = serde_json::to_value(&document.collaborations)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    let mut xml_cache = state.deployed_xml.write().await;
    let collection = state
        .engine
        .deploy_collection(document.processes, payload.xml.clone(), metadata)
        .await?;
    for definition in &collection.definitions {
        xml_cache.insert(definition.key.to_string(), payload.xml.clone());
    }
    let processes = reports
        .into_iter()
        .map(|(id, model_check)| {
            let definition = collection
                .definitions
                .iter()
                .find(|d| d.id == id)
                .expect("selected process was deployed");
            CollectionProcessResult {
                process_id: id,
                definition_key: definition.key.to_string(),
                version: definition.version,
                model_check,
            }
        })
        .collect();
    Ok(Json(CollectionDeployResponse {
        collection_id: collection.id.to_string(),
        processes,
        collaborations: document.collaborations,
    }))
}

/// Retain shared declarations and namespace bindings, but remove other pools,
/// collaboration diagrams, and DI so analyzer findings belong to this pool.
fn isolate_process(xml: &str, process_id: &str) -> Result<String, String> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut writer = quick_xml::Writer::new(Vec::new());
    let mut depth = 0usize;
    let mut skip = None;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        if matches!(event, Event::Eof) {
            break;
        }
        let mut omit = skip.is_some();
        if let Event::Start(e) | Event::Empty(e) = &event
            && depth == 1
        {
            let local = e.local_name();
            let name = local.as_ref();
            let remove = if name == "process" {
                let mut selected = false;
                for attribute in e.attributes() {
                    let attribute = attribute.map_err(|e| e.to_string())?;
                    if attribute.key.as_ref() == "id" {
                        selected = attribute
                            .normalized_value(quick_xml::XmlVersion::default())
                            .map_err(|e| e.to_string())?
                            == process_id;
                    }
                }
                !selected
            } else {
                name == "collaboration" || name == "BPMNDiagram"
            };
            if remove {
                omit = true;
                if matches!(event, Event::Start(_)) {
                    skip = Some(depth);
                }
            }
        }
        if matches!(event, Event::Start(_)) {
            depth += 1;
        }
        if matches!(event, Event::End(_)) {
            depth -= 1;
            if skip == Some(depth) {
                skip = None;
            }
        }
        if !omit {
            writer.write_event(event).map_err(|e| e.to_string())?;
        }
    }
    String::from_utf8(writer.into_inner()).map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub(crate) struct DefinitionInfo {
    pub key: String,
    pub bpmn_id: String,
    pub version: i32,
    pub node_count: usize,
    pub is_latest: bool,
}

pub(crate) async fn list_definitions(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<DefinitionInfo>> {
    let engine = &state.engine;
    let raw = engine.list_definitions().await;

    // Determine the latest version per bpmn_id
    let mut latest_versions: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();
    for (_, bpmn_id, version, _) in &raw {
        let entry = latest_versions.entry(bpmn_id.clone()).or_insert(0);
        if *version > *entry {
            *entry = *version;
        }
    }

    let defs: Vec<DefinitionInfo> = raw
        .into_iter()
        .map(|(key, bpmn_id, version, node_count)| {
            let is_latest = latest_versions.get(&bpmn_id).copied() == Some(version);
            DefinitionInfo {
                key: key.to_string(),
                bpmn_id,
                version,
                node_count,
                is_latest,
            }
        })
        .collect();
    Json(defs)
}

pub(crate) async fn get_definition_xml(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<String, AppError> {
    {
        let xml_store = &state.deployed_xml;
        if let Some(xml) = xml_store.read().await.get(&id) {
            return Ok(xml.clone());
        }
    }

    if let Some(persistence) = &state.persistence
        && let Ok(xml) = persistence.load_bpmn_xml(&id).await
    {
        return Ok(xml);
    }

    Err(AppError::BadRequest(format!(
        "No XML found for definition '{id}'"
    )))
}

#[derive(Deserialize)]
pub(crate) struct DeleteDefinitionQuery {
    pub cascade: Option<bool>,
}

pub(crate) async fn delete_definition(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<DeleteDefinitionQuery>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let def_key = parse_uuid(&id)?;

    engine
        .delete_definition(def_key, query.cascade.unwrap_or(false))
        .await?;

    state.deployed_xml.write().await.remove(&id);

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn delete_all_definitions(
    State(state): State<Arc<AppState>>,
    Path(bpmn_id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<DeleteDefinitionQuery>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;

    // Get all versions before deleting so we can clean up the XML cache
    let versions = engine.list_definition_versions(&bpmn_id).await;

    engine
        .delete_all_definitions(&bpmn_id, query.cascade.unwrap_or(false))
        .await?;

    let mut xml_cache = state.deployed_xml.write().await;
    for (key, _, _) in versions {
        xml_cache.remove(&key.to_string());
    }

    Ok(StatusCode::NO_CONTENT)
}
