//! Server-Sent Events (SSE) endpoint for push-based engine state notifications.
//!
//! Clients connect to `GET /api/events` and receive a stream of events whenever
//! the engine changes state (instance changed, task created/completed, definition deployed).

use axum::{
    extract::State,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use engine_core::engine::EngineEvent;
use std::{convert::Infallible, sync::Arc};
use tokio_stream::Stream;

use super::state::AppState;

/// SSE handler — each connecting client gets its own broadcast receiver.
///
/// Event names remain compatible; data contains entity IDs and compact deltas.
/// Broadcast loss is explicit: clients must reload after `resync_required`.
pub async fn engine_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.engine.subscribe_events();
    Sse::new(event_stream(rx)).keep_alive(KeepAlive::default())
}

fn event_stream(
    rx: tokio::sync::broadcast::Receiver<EngineEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    futures::stream::unfold(rx, |mut rx| async move {
        let event = match rx.recv().await {
            Ok(event) => {
                let event_type = match &event {
                    EngineEvent::InstanceChanged { .. } => "instance_changed",
                    EngineEvent::TaskChanged { .. } => "task_changed",
                    EngineEvent::DefinitionChanged { .. } => "definition_changed",
                };
                Event::default()
                    .event(event_type)
                    .json_data(event)
                    .unwrap_or_else(|_| resync_event())
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                // Veraltete Restereignisse nicht nach dem Resync erneut anwenden.
                rx = rx.resubscribe();
                resync_event()
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
        };
        Some((Ok(event), rx))
    })
}

fn resync_event() -> Event {
    Event::default()
        .event("resync_required")
        .data(r#"{"type":"resync_required"}"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use futures::StreamExt;
    use uuid::Uuid;

    #[tokio::test]
    async fn sse_transmits_entity_payload() {
        let (tx, rx) = tokio::sync::broadcast::channel(2);
        let key = Uuid::new_v4();
        tx.send(EngineEvent::DefinitionChanged {
            definition_key: key,
        })
        .unwrap();
        let mut body = Sse::new(event_stream(rx))
            .into_response()
            .into_body()
            .into_data_stream();
        let bytes = body.next().await.unwrap().unwrap();
        let frame = std::str::from_utf8(&bytes).unwrap();
        assert!(frame.contains("event: definition_changed"));
        let data = frame
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["definition_key"], key.to_string());
        assert_eq!(payload["type"], "definition_changed");
    }

    #[tokio::test]
    async fn lag_resync_discards_stale_backlog() {
        let (tx, rx) = tokio::sync::broadcast::channel(2);
        for _ in 0..3 {
            tx.send(EngineEvent::DefinitionChanged {
                definition_key: Uuid::new_v4(),
            })
            .unwrap();
        }
        let mut body = Sse::new(event_stream(rx))
            .into_response()
            .into_body()
            .into_data_stream();
        let bytes = body.next().await.unwrap().unwrap();
        assert!(
            std::str::from_utf8(&bytes)
                .unwrap()
                .contains("resync_required")
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), body.next())
                .await
                .is_err()
        );
        let key = Uuid::new_v4();
        tx.send(EngineEvent::DefinitionChanged {
            definition_key: key,
        })
        .unwrap();
        let bytes = body.next().await.unwrap().unwrap();
        assert!(
            std::str::from_utf8(&bytes)
                .unwrap()
                .contains(&key.to_string())
        );
    }
}
