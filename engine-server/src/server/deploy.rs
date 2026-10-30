use crate::server::model_check::log_report;
use crate::server::state::{ApiJson, AppError, AppState, MAX_XML_BYTES, parse_uuid};
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
    /// Present unless `MODEL_CHECK=off` (the default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_check: Option<crate::server::model_check::ModelCheckReport>,
}

pub(crate) async fn deploy_definition(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<DeployRequest>,
) -> Result<Json<DeployResponse>, AppError> {
    // Shared with DefaultBodyLimit: default `MAX_XML_BYTES` (10 MiB); env may override.
    if payload.xml.len() > state.max_xml_bytes {
        return Err(AppError::PayloadTooLarge(format!(
            "XML too large: {} bytes (max {})",
            payload.xml.len(),
            state.max_xml_bytes
        )));
    }
    let engine = &state.engine;
    let def = bpmn_parser::parse_bpmn_xml(&payload.xml)
        .map_err(|e| AppError::BadRequest(format!("Invalid BPMN XML: {e:?}")))?;

    // Invalid XML never occupies the single analysis slot.
    let model_check = if let Some(report) = state.model_checker.check(&payload.xml).await {
        log_report(&payload.name, &report);
        if state.model_checker.rejects_deploy(&report) {
            return Err(AppError::ModelCheckRejected(report));
        }
        Some(report)
    } else {
        None
    };

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
    /// Metadaten sind gespeichert. Automatische interne Zustellung ist aus.
    pub collaborations: Vec<engine_core::Collaboration>,
}

pub(crate) async fn deploy_collection(
    State(state): State<Arc<AppState>>,
    ApiJson(payload): ApiJson<DeployRequest>,
) -> Result<Json<CollectionDeployResponse>, AppError> {
    if payload.xml.len() > state.max_xml_bytes {
        return Err(AppError::PayloadTooLarge(format!(
            "XML too large: {} bytes (max {})",
            payload.xml.len(),
            state.max_xml_bytes
        )));
    }
    let document = bpmn_parser::parse_bpmn_collaboration(&payload.xml)
        .map_err(|err| AppError::BadRequest(format!("Invalid BPMN XML: {err:?}")))?;
    if document.processes.is_empty() {
        return Err(AppError::BadRequest(
            "Collection has no executable processes".into(),
        ));
    }
    let mut reports = Vec::new();
    for process in &document.processes {
        let checked_xml = xml_for_model_check(&payload.xml, &process.id)?;
        if let Some(report) = state.model_checker.check(&checked_xml).await {
            log_report(&process.id, &report);
            if state.model_checker.rejects_deploy(&report) {
                return Err(AppError::BadRequest(format!(
                    "Model check failed for '{}': {}",
                    process.id,
                    report.summary()
                )));
            }
            reports.push((process.id.clone(), Some(report)));
        } else {
            reports.push((process.id.clone(), None));
        }
    }
    let metadata = serde_json::to_value(&document.collaborations)
        .map_err(|err| AppError::BadRequest(err.to_string()))?;
    let collection = state
        .engine
        .deploy_collection(document.processes, payload.xml.clone(), metadata)
        .await?;
    {
        let mut xml_cache = state.deployed_xml.write().await;
        for definition in &collection.definitions {
            xml_cache.insert(definition.key.to_string(), payload.xml.clone());
        }
    }
    let mut processes = Vec::with_capacity(reports.len());
    for (id, model_check) in reports {
        let definition = collection
            .definitions
            .iter()
            .find(|definition| definition.id == id)
            .ok_or_else(|| {
                AppError::BadRequest(format!("selected process '{id}' was not deployed"))
            })?;
        processes.push(CollectionProcessResult {
            process_id: id,
            definition_key: definition.key.to_string(),
            version: definition.version,
            model_check,
        });
    }
    Ok(Json(CollectionDeployResponse {
        collection_id: collection.id.to_string(),
        processes,
        collaborations: document.collaborations,
    }))
}

fn xml_for_model_check(xml: &str, process_id: &str) -> Result<String, AppError> {
    #[cfg(feature = "model-check")]
    {
        isolate_process(xml, process_id)
    }
    #[cfg(not(feature = "model-check"))]
    {
        let _ = process_id;
        Ok(xml.to_string())
    }
}

/// Behält gemeinsame Deklarationen und Namespace-Bindings, entfernt aber
/// andere Pools, Collaborations und DI, damit die Analyse nur diesen Pool sieht.
#[cfg(feature = "model-check")]
fn isolate_process(xml: &str, process_id: &str) -> Result<String, AppError> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut writer = quick_xml::Writer::new(Vec::new());
    let mut depth = 0usize;
    let mut skip = None;
    loop {
        let event = reader
            .read_event()
            .map_err(|err| AppError::BadRequest(err.to_string()))?;
        if matches!(event, Event::Eof) {
            break;
        }
        let mut omit = skip.is_some();
        if let Event::Start(element) | Event::Empty(element) = &event
            && depth == 1
        {
            let local = element.local_name();
            let name = local.as_ref();
            let remove = if name == "process" {
                let mut selected = false;
                for attribute in element.attributes() {
                    let attribute =
                        attribute.map_err(|err| AppError::BadRequest(err.to_string()))?;
                    if attribute.key.as_ref() == "id" {
                        selected = attribute
                            .normalized_value(quick_xml::XmlVersion::default())
                            .map_err(|err| AppError::BadRequest(err.to_string()))?
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
            writer
                .write_event(event)
                .map_err(|err| AppError::BadRequest(err.to_string()))?;
        }
    }
    String::from_utf8(writer.into_inner()).map_err(|err| AppError::BadRequest(err.to_string()))
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

    Err(AppError::NotFound(format!(
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
