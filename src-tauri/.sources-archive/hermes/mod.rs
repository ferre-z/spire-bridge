//! Hermes Agent adapter.
//!
//! Wires the local HTTP API (sessions/message replay) with the JSON-RPC
//! WebSocket subscription (`/api/events`) and normalises everything into
//! canonical events via `normalize`.
pub mod http;
pub mod normalize;
pub mod ws;

use crate::sources::{CanonicalEvent, CanonicalSession, Source, SourceError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Hermes adapter. Defaults to `http://127.0.0.1:9119`; override with
/// [`HermesSource::with_url`] for a custom host/port.
#[derive(Clone)]
pub struct HermesSource {
    pub(crate) client: Arc<http::HermesClient>,
    pub(crate) ws_url: String,
}

impl HermesSource {
    pub fn new() -> Self {
        let ws_url = std::env::var("HERMES_WS_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:9119/api/events".to_string());
        Self {
            client: Arc::new(http::HermesClient::new(
                std::env::var("HERMES_HTTP_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:9119".to_string()),
            )),
            ws_url,
        }
    }

    pub fn with_url(http_url: impl Into<String>, ws_url: impl Into<String>) -> Self {
        Self {
            client: Arc::new(http::HermesClient::new(http_url.into())),
            ws_url: ws_url.into(),
        }
    }
}

impl Default for HermesSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Source for HermesSource {
    fn id(&self) -> &'static str {
        "hermes"
    }

    async fn health(&self) -> Result<(), SourceError> {
        self.client.health().await
    }

    async fn backfill(
        &self,
        since_ts: f64,
    ) -> Result<Vec<(CanonicalSession, CanonicalEvent)>, SourceError> {
        let sessions = self.client.list_sessions().await?;
        let mut out = Vec::new();
        for s in sessions {
            let cs = normalize::session_from_api(&s);
            if cs.started_at < since_ts {
                continue;
            }
            let msgs = self.client.messages(&cs.native_id).await?;
            for m in msgs {
                let ce = normalize::event_from_message(&cs.id, &m, &cs.source_path);
                out.push((cs.clone(), ce));
            }
        }
        Ok(out)
    }

    async fn live_events(&self) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
        let (tx, rx) = mpsc::channel(256);
        let stream = ws::connect(&self.ws_url, self.client.token())
            .await
            .map_err(SourceError::Transport)?;
        tokio::spawn(async move {
            use futures::StreamExt;
            let mut stream = stream;
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(ev) => {
                        if tx.send(ev).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(rx)
    }
}
