//! OpenCode adapter.
//!
//! `OpenCodeSource` is a thin wrapper that wires [`http::OpenCodeClient`]
//! and [`sse::open_sse`] together and implements [`Source`].

pub mod http;
pub mod normalize;
pub mod sse;

use crate::sources::{CanonicalEvent, CanonicalSession, Source, SourceError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Source adapter for OpenCode's local HTTP+SSE server.
#[derive(Clone)]
pub struct OpenCodeSource {
    pub(crate) client: Arc<http::OpenCodeClient>,
    channel_capacity: usize,
}

impl OpenCodeSource {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Arc::new(http::OpenCodeClient::new(base_url)),
            channel_capacity: 1024,
        }
    }

    /// Override the channel buffer used for live events.
    pub fn with_channel_capacity(mut self, cap: usize) -> Self {
        self.channel_capacity = cap;
        self
    }

    /// Reference to the inner HTTP client (used by tests + sync engine).
    pub fn client(&self) -> &http::OpenCodeClient {
        &self.client
    }
}

#[async_trait]
impl Source for OpenCodeSource {
    fn id(&self) -> &'static str {
        "opencode"
    }

    async fn health(&self) -> Result<(), SourceError> {
        self.client.health().await
    }

    async fn backfill(
        &self,
        since: Option<f64>,
    ) -> Result<Vec<(CanonicalSession, Vec<CanonicalEvent>)>, SourceError> {
        let mut out: Vec<(CanonicalSession, Vec<CanonicalEvent>)> = Vec::new();
        let mut offset: u32 = 0;
        let limit: u32 = 100;

        loop {
            let page = self.client.list_sessions(limit, offset).await?;
            let any = !page.sessions.is_empty();
            for session in page.sessions {
                let events = self.client.messages(&session.native_id, since).await?;
                out.push((session, events));
            }
            match page.next_offset {
                Some(next) => offset = next,
                None => break,
            }
            if !any {
                break;
            }
        }

        Ok(out)
    }

    async fn live_events(&self) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
        sse::open_sse(&self.client.base_url, self.channel_capacity).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_opencode() {
        assert_eq!(OpenCodeSource::new("http://127.0.0.1:4096").id(), "opencode");
    }
}