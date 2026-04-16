//! NATS JetStream-Persistenz für den rollenden Log-Buffer.
//!
//! Legt einen JetStream-Stream `ENGINE_LOGS` an (max. 50 000 Nachrichten,
//! älteste werden automatisch verdrängt).  Neue Log-Einträge werden per
//! asynchronem `publish()` an den Stream gesandt; beim Serverstart lädt
//! `load_recent()` die letzten N Einträge zurück in den In-Memory-Buffer.

use async_nats::jetstream::{
    consumer::{self, AckPolicy, DeliverPolicy},
    context::Context,
    stream,
};
use futures::StreamExt;

use crate::log_buffer::LogEntry;

/// Name des JetStream-Streams für Log-Einträge.
const STREAM_NAME: &str = "ENGINE_LOGS";
/// NATS-Subject für alle Log-Publikationen.
const SUBJECT: &str = "engine.logs";
/// Maximale Anzahl von Nachrichten, die NATS im Stream hält.
const MAX_MESSAGES: i64 = 50_000;

// ---------------------------------------------------------------------------

pub struct NatsLogSink {
    js: Context,
}

impl NatsLogSink {
    /// Erstellt den `ENGINE_LOGS`-Stream (falls noch nicht vorhanden) und gibt
    /// eine `NatsLogSink`-Instanz zurück.
    pub async fn new(js: Context) -> Self {
        let _ = js
            .get_or_create_stream(stream::Config {
                name: STREAM_NAME.to_string(),
                subjects: vec![SUBJECT.to_string()],
                max_messages: MAX_MESSAGES,
                ..Default::default()
            })
            .await
            .inspect_err(|e| tracing::warn!("Konnte ENGINE_LOGS Stream nicht anlegen: {e}"));

        Self { js }
    }

    /// Publiziert einen Log-Eintrag als JSON-Zeile an den NATS-Stream.
    /// Fehler werden nur geloggt (kein Panic / kein Propagate).
    pub async fn publish(&self, entry: &LogEntry) {
        let Ok(json) = serde_json::to_vec(entry) else {
            return;
        };
        if let Err(e) = self.js.publish(SUBJECT, json.into()).await {
            tracing::warn!("ENGINE_LOGS publish fehlgeschlagen: {e}");
        }
    }

    /// Lädt die letzten `n` Log-Einträge aus dem NATS-Stream.
    /// Gibt im Fehlerfall einen leeren Vec zurück (Server-Restart-sicher).
    pub async fn load_recent(&self, n: usize) -> Vec<LogEntry> {
        let stream = match self.js.get_stream(STREAM_NAME).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("ENGINE_LOGS Stream nicht gefunden: {e}");
                return vec![];
            }
        };

        // Ephemerer Pull-Consumer — holt alle verfügbaren Nachrichten.
        // Der Stream ist durch MAX_MESSAGES auf 50 000 Einträge begrenzt;
        // wir laden maximal n davon (die neuesten).
        let consumer = match stream
            .create_consumer(consumer::pull::Config {
                deliver_policy: DeliverPolicy::All,
                ack_policy: AckPolicy::None,
                ..Default::default()
            })
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Konnte ENGINE_LOGS Consumer nicht anlegen: {e}");
                return vec![];
            }
        };

        let mut messages = match consumer.messages().await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Konnte ENGINE_LOGS Messages nicht abrufen: {e}");
                return vec![];
            }
        };

        let mut entries: Vec<LogEntry> = Vec::new();

        // Timeout-Loop: Nachrichten lesen bis keine mehr kommen (< 200 ms Pause).
        while let Ok(Some(msg)) = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            messages.next(),
        )
        .await
        {
            if let Ok(msg) = msg
                && let Ok(entry) = serde_json::from_slice::<LogEntry>(&msg.payload)
            {
                entries.push(entry);
            }
        }

        // Nur die letzten n Einträge zurückgeben (stream kann mehr enthalten)
        if entries.len() > n {
            entries.drain(..entries.len() - n);
        }
        entries
    }
}
