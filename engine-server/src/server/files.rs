use crate::server::state::{AppError, AppState, parse_uuid};
use axum::http::HeaderValue;
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// RFC 6266 / RFC 5987 `Content-Disposition` for downloads.
/// Strips CR/LF, quotes, and path separators so filenames cannot inject headers.
pub(crate) fn content_disposition_attachment(filename: &str) -> HeaderValue {
    let basename = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .trim();
    let ascii_name: String = basename
        .chars()
        .filter(|c| c.is_ascii() && *c >= ' ' && !matches!(*c, '"' | '\\' | ';' | '\r' | '\n'))
        .take(200)
        .collect();
    let ascii_name = if ascii_name.is_empty() {
        "download".to_string()
    } else {
        ascii_name
    };
    let encoded = rfc5987_encode(basename);
    let header = format!("attachment; filename=\"{ascii_name}\"; filename*=UTF-8''{encoded}");
    HeaderValue::from_str(&header).unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

fn rfc5987_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub(crate) async fn upload_instance_file(
    State(state): State<Arc<AppState>>,
    Path((id, var_name)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;
    if engine.get_instance_details(instance_id).await.is_err() {
        return Err(AppError::BadRequest("Instance not found".into()));
    }

    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let filename = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        let max_bytes = state.max_upload_bytes;
        if data.len() > max_bytes {
            return Err(AppError::PayloadTooLarge(format!(
                "Upload exceeds maximum size of {max_bytes} bytes (got {} bytes)",
                data.len()
            )));
        }

        let file_ref = engine_core::model::FileReference::new(
            instance_id,
            &var_name,
            &filename,
            &content_type,
            data.len() as u64,
        );

        if let Some(persistence) = &state.persistence {
            persistence
                .save_file(&file_ref.object_key, &data)
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to save file: {:?}", e)))?;
        }

        let mut vars = HashMap::new();
        vars.insert(var_name, file_ref.to_variable_value());
        engine.update_instance_variables(instance_id, vars).await?;

        Ok(StatusCode::CREATED)
    } else {
        Err(AppError::BadRequest("No file field provided".into()))
    }
}

pub(crate) async fn get_instance_file(
    State(state): State<Arc<AppState>>,
    Path((id, var_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;
    let instance = engine.get_instance_details(instance_id).await?;

    let file_ref = instance
        .get_file_reference(&var_name)
        .ok_or_else(|| AppError::BadRequest("Variable is not a file".into()))?;

    if let Some(persistence) = &state.persistence {
        let data = persistence
            .load_file(&file_ref.object_key)
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to load file: {:?}", e)))?;

        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            file_ref
                .mime_type
                .parse()
                .unwrap_or(axum::http::HeaderValue::from_static(
                    "application/octet-stream",
                )),
        );
        headers.insert(
            axum::http::header::CONTENT_DISPOSITION,
            content_disposition_attachment(&file_ref.filename),
        );

        Ok((headers, data))
    } else {
        Err(AppError::BadRequest("No persistence configured".into()))
    }
}

pub(crate) async fn delete_instance_file(
    State(state): State<Arc<AppState>>,
    Path((id, var_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let engine = &state.engine;
    let instance_id = parse_uuid(&id)?;
    let instance = engine.get_instance_details(instance_id).await?;

    let file_ref = instance
        .get_file_reference(&var_name)
        .ok_or_else(|| AppError::BadRequest("Variable is not a file".into()))?;

    if let Some(persistence) = &state.persistence {
        persistence
            .delete_file(&file_ref.object_key)
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to delete file: {:?}", e)))?;
    }

    let mut vars = HashMap::new();
    vars.insert(var_name, Value::Null);
    engine.update_instance_variables(instance_id, vars).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_disposition_uses_quoted_ascii_and_rfc5987() {
        let value = content_disposition_attachment("report.pdf");
        let s = value.to_str().expect("ascii header");
        assert!(s.starts_with("attachment; filename=\"report.pdf\""));
        assert!(s.contains("filename*=UTF-8''report.pdf"));
    }

    #[test]
    fn content_disposition_strips_header_injection_and_paths() {
        let value = content_disposition_attachment("evil\r\nX-Injected: yes\n../../secret.txt");
        let s = value.to_str().expect("ascii header");
        assert!(!s.contains('\r'));
        assert!(!s.contains('\n'));
        assert!(!s.contains("X-Injected"));
        assert!(s.contains("filename=\"secret.txt\""));
    }

    #[test]
    fn content_disposition_encodes_quotes_and_utf8() {
        let value = content_disposition_attachment("größe \";attack.txt");
        let s = value.to_str().expect("ascii header");
        assert!(!s.contains('\r'));
        assert!(!s.contains('\n'));
        let quoted = s
            .split("filename=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or("");
        assert!(!quoted.contains('"'));
        assert!(!quoted.contains(';'));
        assert!(s.contains("filename*=UTF-8''"));
        assert!(s.contains("attack.txt"));
    }
}
