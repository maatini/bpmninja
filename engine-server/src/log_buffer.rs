//! Rollender In-Memory-Log-Buffer für den Engine-Server.
//!
//! Implementiert einen `tracing::Layer`, der alle Log-Events abfängt
//! und in einem `VecDeque` mit maximal `MAX_ENTRIES` Einträgen speichert.
//! Älteste Einträge werden automatisch verdrängt.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::Serialize;
use tracing::Level;
use tracing_subscriber::Layer;

/// Maximale Anzahl an Log-Einträgen im Buffer.
const MAX_ENTRIES: usize = 5_000;

/// Ein einzelner Log-Eintrag.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// ISO-8601 Zeitstempel (UTC).
    pub timestamp: String,
    /// Log-Level: "ERROR", "WARN", "INFO", "DEBUG", "TRACE".
    pub level: String,
    /// Rust-Modul-Pfad der Quelle.
    pub target: String,
    /// Die formatierte Log-Nachricht.
    pub message: String,
}

/// Rollender Log-Buffer — thread-sicher über `Arc<Mutex<...>>`.
#[derive(Debug, Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_ENTRIES))),
        }
    }

    /// Gibt alle Einträge als geklonten Vec zurück, optional gefiltert.
    ///
    /// - `level_filter`: Mindest-Level ("error", "warn", "info", "debug", "trace").
    ///   Einträge mit niedrigerem Level werden übersprungen.
    /// - `search`: Substring-Filter auf `message` und `target` (case-insensitive).
    pub fn entries(&self, level_filter: Option<&str>, search: Option<&str>) -> Vec<LogEntry> {
        let min_level = level_filter
            .and_then(|l| parse_level(l))
            .unwrap_or(Level::TRACE);

        let search_lower = search.map(|s| s.to_lowercase());

        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .iter()
            .filter(|e| {
                let entry_level = parse_level(&e.level).unwrap_or(Level::TRACE);
                entry_level <= min_level
            })
            .filter(|e| {
                if let Some(ref q) = search_lower {
                    e.message.to_lowercase().contains(q.as_str())
                        || e.target.to_lowercase().contains(q.as_str())
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    fn push(&self, entry: LogEntry) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if guard.len() >= MAX_ENTRIES {
            guard.pop_front();
        }
        guard.push_back(entry);
    }
}

fn parse_level(s: &str) -> Option<Level> {
    match s.to_uppercase().as_str() {
        "ERROR" => Some(Level::ERROR),
        "WARN" => Some(Level::WARN),
        "INFO" => Some(Level::INFO),
        "DEBUG" => Some(Level::DEBUG),
        "TRACE" => Some(Level::TRACE),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// tracing::Layer-Implementierung
// ---------------------------------------------------------------------------

impl<S> Layer<S> for LogBuffer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = event.metadata().level().to_string();
        let target = event.metadata().target().to_string();

        // Nachricht aus den Fields extrahieren
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        self.push(LogEntry {
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            level,
            target,
            message: visitor.message,
        });
    }
}

/// Besucher, der das `message`-Field aus einem tracing-Event extrahiert.
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value).trim_matches('"').to_string();
        } else if self.message.is_empty() {
            self.message = format!("{}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}
