//! OpenCode SSE event subscriber.
//!
//! Opens a long-lived `text/event-stream` connection to `GET /event` and
//! yields one [`CanonicalEvent`] per inbound event. The placeholder
//! session id is `opencode:<event.sessionID>` (the real id is filled
//! in by the store when the session row is upserted).
//!
//! The connection is owned by the receiver: dropping the `mpsc::Receiver`
//! aborts the stream and tears down the underlying HTTP response.

use crate::sources::opencode::normalize::opencode_event_from_sse;
use crate::sources::{CanonicalEvent, SourceError};
use eventsource_client as esc;
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Spawn the SSE consumer and return a [`mpsc::Receiver`] of canonical
/// events. Phase 1 only consumes the event stream; backfill comes from
/// the REST list call ([`super::http`]).
pub async fn open_sse(
    base_url: &str,
    channel_capacity: usize,
) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
    let url = format!("{base_url}/event");
    let client = esc::ClientBuilder::for_url(&url)
        .map_err(|e| SourceError::Upstream(format!("invalid sse url: {e}")))?
        .build();

    let (tx, rx) = mpsc::channel(channel_capacity);
    let tx = Arc::new(tx);

    tokio::spawn(async move {
        let mut stream = client.stream();
        let mut seq: i64 = 0;
        while let Some(event) = stream.next().await {
            match event {
                Ok(es) => {
                    if es.event_type == "ping" {
                        continue;
                    }
                    let value: Value = match serde_json::from_str(&es.data) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    // The SSE event carries sessionID at the top level.
                    let native = value
                        .get("sessionID")
                        .or_else(|| value.get("sessionId"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let placeholder = format!("opencode:{native}");

                    seq += 1;
                    let canonical = opencode_event_from_sse(&placeholder, seq, &value);
                    if tx.send(canonical).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "opencode sse error; aborting stream");
                    break;
                }
            }
        }
    });

    Ok(rx)
}