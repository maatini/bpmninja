use crate::state::{AppState, MonitoringData};

#[tauri::command]
pub async fn get_api_url(state: tauri::State<'_, AppState>) -> Result<String, String> {
    crate::state::get_base_url(&state)
}

#[tauri::command]
pub async fn set_api_url(state: tauri::State<'_, AppState>, url: String) -> Result<(), String> {
    let mut lock = state.base_url.lock().map_err(|e| e.to_string())?;
    *lock = url.trim_end_matches('/').to_string();
    Ok(())
}

#[tauri::command]
pub async fn get_api_key(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(crate::state::get_api_key(&state)?.unwrap_or_default())
}

#[tauri::command]
pub async fn set_api_key(state: tauri::State<'_, AppState>, key: String) -> Result<(), String> {
    let normalized = key.trim();
    let mut lock = state.api_key.lock().map_err(|e| e.to_string())?;
    *lock = if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    };
    Ok(())
}

#[tauri::command]
pub async fn get_monitoring_data(
    state: tauri::State<'_, AppState>,
) -> Result<MonitoringData, String> {
    let base = crate::state::get_base_url(&state)?;
    let url = format!("{}/api/monitoring", base);
    let res = crate::api_helpers::with_auth(state.client.get(&url), &state)?
        .send()
        .await
        .map_err(|e| format!("Verbindung fehlgeschlagen: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Engine antwortete mit Status {}", res.status()));
    }
    res.json()
        .await
        .map_err(|e| format!("Ungültige Antwort: {e}"))
}

#[tauri::command]
pub async fn get_log_entries(
    state: tauri::State<'_, AppState>,
    level: Option<String>,
    search: Option<String>,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let base = crate::state::get_base_url(&state)?;
    let url = format!("{}/api/logs", base);
    let mut req = crate::api_helpers::with_auth(state.client.get(&url), &state)?;
    if let Some(l) = &level {
        req = req.query(&[("level", l.as_str())]);
    }
    if let Some(s) = &search {
        req = req.query(&[("search", s.as_str())]);
    }
    if let Some(n) = limit {
        req = req.query(&[("limit", n.to_string().as_str())]);
    }

    match req.send().await {
        Ok(res) if res.status().is_success() => res
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string()),
        Ok(res) => Err(format!("Status {}", res.status())),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn read_bpmn_file(path: String) -> Result<String, String> {
    let xml = std::fs::read_to_string(&path)
        .map_err(|e| format!("Could not read file '{}': {}", path, e))?;
    Ok(xml)
}

#[tauri::command]
pub async fn get_bucket_entries(
    state: tauri::State<'_, AppState>,
    bucket: String,
    offset: usize,
    limit: usize,
) -> Result<serde_json::Value, String> {
    let base = crate::state::get_base_url(&state)?;
    let url = format!(
        "{}/api/monitoring/buckets/{}/entries?offset={}&limit={}",
        base, bucket, offset, limit
    );
    match crate::api_helpers::with_auth(state.client.get(&url), &state)?
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            let data = res
                .json::<serde_json::Value>()
                .await
                .map_err(|e| e.to_string())?;
            Ok(data)
        }
        Ok(res) => Err(res
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string())),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_bucket_entry_detail(
    state: tauri::State<'_, AppState>,
    bucket: String,
    key: String,
) -> Result<serde_json::Value, String> {
    let base = crate::state::get_base_url(&state)?;
    let url = format!("{}/api/monitoring/buckets/{}/entries/{}", base, bucket, key);
    match crate::api_helpers::with_auth(state.client.get(&url), &state)?
        .send()
        .await
    {
        Ok(res) if res.status().is_success() => {
            let data = res
                .json::<serde_json::Value>()
                .await
                .map_err(|e| e.to_string())?;
            Ok(data)
        }
        Ok(res) => Err(res
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string())),
        Err(e) => Err(e.to_string()),
    }
}
