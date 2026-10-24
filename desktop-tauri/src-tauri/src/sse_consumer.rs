//! Background SSE consumer — verbindet sich mit `/api/events` der Engine und emittiert
//! Tauri-Events, damit die UI push-basiert aktualisiert werden kann.
//!
//! Das Tauri-Event `engine-event` übernimmt IDs und Deltas aus dem SSE-Payload.
//! Reconnect-Schleife mit exponentiellem Backoff (max 30s).
//! Die Base-URL wird bei jedem Connect/Reconnect aus `AppState` gelesen; ein Wechsel
//! bricht die aktuelle Verbindung ab und setzt den Backoff zurück.

use futures_util::TryStreamExt;
use reqwest::Client;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;

use crate::state::{get_base_url, AppState};

const URL_POLL_SECS: u64 = 1;

#[derive(Default)]
struct SseMessage {
    event_type: String,
    data: Vec<String>,
}

impl SseMessage {
    fn push_line(&mut self, line: &str) -> Option<serde_json::Value> {
        if line.is_empty() {
            let event_type = std::mem::take(&mut self.event_type);
            let data = std::mem::take(&mut self.data).join("\n");
            if event_type.is_empty() {
                return None;
            }
            // Alte Server senden leere Daten; deren Invalidierung bleibt gültig.
            if data.is_empty() {
                return Some(serde_json::json!({ "type": event_type }));
            }
            return Some(match serde_json::from_str::<serde_json::Value>(&data) {
                Ok(serde_json::Value::Object(mut payload)) => {
                    payload.insert("type".into(), event_type.into());
                    serde_json::Value::Object(payload)
                }
                _ => serde_json::json!({ "type": "resync_required" }),
            });
        }
        if let Some(event_type) = line.strip_prefix("event:") {
            self.event_type = event_type.trim().to_string();
        } else if let Some(data) = line.strip_prefix("data:") {
            self.data
                .push(data.strip_prefix(' ').unwrap_or(data).to_string());
        }
        None
    }
}

/// Spawnt den SSE-Consumer-Task. Gibt sofort zurück; Arbeit läuft im Hintergrund.
pub fn spawn(app_handle: AppHandle, client: Client) {
    tauri::async_runtime::spawn(run_consumer(app_handle, client));
}

fn sse_url_from_state(app_handle: &AppHandle) -> Result<String, String> {
    let state = app_handle.state::<AppState>();
    let base = get_base_url(&state)?;
    Ok(format!("{}/api/events", base.trim_end_matches('/')))
}

async fn run_consumer(app_handle: AppHandle, client: Client) {
    let mut backoff_secs: u64 = 1;
    let mut last_url = String::new();

    loop {
        let url = match sse_url_from_state(&app_handle) {
            Ok(u) => u,
            Err(e) => {
                tracing_or_eprintln(format!("SSE: Base-URL nicht lesbar — {e}"));
                tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(30);
                continue;
            }
        };

        if url != last_url {
            backoff_secs = 1;
            last_url = url.clone();
        }

        tracing_or_eprintln(format!("SSE: Verbinde mit {url}"));

        match connect_and_consume(&client, &url, &app_handle).await {
            Ok(ConsumeOutcome::StreamEnded) => {
                tracing_or_eprintln("SSE: Stream beendet, verbinde neu…".to_string());
                backoff_secs = 1;
            }
            Ok(ConsumeOutcome::UrlChanged) => {
                tracing_or_eprintln("SSE: Base-URL geändert, verbinde neu…".to_string());
                backoff_secs = 1;
                continue;
            }
            Err(e) => {
                tracing_or_eprintln(format!(
                    "SSE: Fehler — {e}, Wiederverbindung in {backoff_secs}s"
                ));
            }
        }

        if sleep_or_url_change(&app_handle, backoff_secs, &last_url).await {
            backoff_secs = 1;
            continue;
        }
        backoff_secs = (backoff_secs * 2).min(30);
    }
}

/// Sleeps `secs`, but returns `true` early if the SSE URL in AppState changes.
async fn sleep_or_url_change(app_handle: &AppHandle, secs: u64, current_url: &str) -> bool {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(secs);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return false;
        }
        let slice = remaining.min(tokio::time::Duration::from_secs(URL_POLL_SECS));
        tokio::time::sleep(slice).await;
        if let Ok(url) = sse_url_from_state(app_handle) {
            if url != current_url {
                return true;
            }
        }
    }
}

enum ConsumeOutcome {
    StreamEnded,
    UrlChanged,
}

async fn connect_and_consume(
    client: &Client,
    url: &str,
    app_handle: &AppHandle,
) -> Result<ConsumeOutcome, String> {
    let headers = {
        let state = app_handle.state::<AppState>();
        crate::api_helpers::auth_headers(&state)?
    };
    let response = client
        .get(url)
        .header("Accept", "text/event-stream")
        .headers(headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    // Kein Replay-Protokoll: jede neue Verbindung invalidiert den lokalen Cache.
    let _ = app_handle.emit(
        "engine-event",
        serde_json::json!({ "type": "resync_required" }),
    );

    let byte_stream = response.bytes_stream().map_err(std::io::Error::other);
    let stream_reader = StreamReader::new(byte_stream);
    let mut lines = BufReader::new(stream_reader).lines();

    let mut message = SseMessage::default();

    loop {
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(URL_POLL_SECS),
            lines.next_line(),
        )
        .await
        {
            Ok(Ok(Some(line))) => {
                if let Some(payload) = message.push_line(&line) {
                    let _ = app_handle.emit("engine-event", payload);
                }
            }
            Ok(Ok(None)) | Ok(Err(_)) => return Ok(ConsumeOutcome::StreamEnded),
            Err(_elapsed) => {
                if let Ok(current) = sse_url_from_state(app_handle) {
                    if current != url {
                        return Ok(ConsumeOutcome::UrlChanged);
                    }
                }
            }
        }
    }
}

/// Minimal fallback logging without requiring a tracing dependency in the Tauri crate.
fn tracing_or_eprintln(msg: String) {
    eprintln!("[bpmninja-sse] {msg}");
}

#[cfg(test)]
mod tests {
    use super::SseMessage;

    #[test]
    fn forwards_multiline_payload_only_at_message_boundary() {
        let mut message = SseMessage::default();
        assert!(message.push_line("event: instance_changed").is_none());
        assert!(message
            .push_line("data: {\"instance_id\":\"id\",")
            .is_none());
        assert!(message
            .push_line("data: \"delta\":{\"current_node\":\"end\"}}")
            .is_none());
        let payload = message.push_line("").unwrap();
        assert_eq!(payload["type"], "instance_changed");
        assert_eq!(payload["instance_id"], "id");
        assert_eq!(payload["delta"]["current_node"], "end");
        assert!(message.push_line("").is_none());
    }

    #[test]
    fn supports_legacy_empty_data_and_recovers_from_bad_json() {
        let mut message = SseMessage::default();
        message.push_line("event: task_changed");
        message.push_line("data:");
        assert_eq!(message.push_line("").unwrap()["type"], "task_changed");
        message.push_line("event: instance_changed");
        message.push_line("data: broken");
        assert_eq!(message.push_line("").unwrap()["type"], "resync_required");
    }
}
