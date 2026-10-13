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
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use super::state::AppState;

/// SSE handler — each connecting client gets its own broadcast receiver.
///
/// Event names remain compatible; data contains entity IDs and compact deltas.
/// Broadcast loss is explicit: clients must reload after `resync_required`.
pub async fn engine_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.engine.subscribe_events();
    let stream = BroadcastStream::new(rx).map(|result| match result {
        Ok(event) => {
            let event_type = match &event {
                EngineEvent::InstanceChanged { .. } => "instance_changed",
                EngineEvent::TaskChanged { .. } => "task_changed",
                EngineEvent::DefinitionChanged { .. } => "definition_changed",
            };
            Ok(Event::default()
                .event(event_type)
                .json_data(event)
                .unwrap_or_else(|_| resync_event()))
        }
        Err(_) => Ok(resync_event()),
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn resync_event() -> Event {
    Event::default()
        .event("resync_required")
        .data(r#"{"type":"resync_required"}"#)
}
