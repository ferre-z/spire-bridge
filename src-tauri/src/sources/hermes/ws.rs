//! Hermes WebSocket client.
//!
//! Connects to `ws://127.0.0.1:9119/api/events` and forwards JSON-RPC
//! envelopes into the normaliser. The connection is owned by the
//! returned [`mpsc::Receiver`]: dropping the receiver aborts the loop.

use crate::sources::hermes::normalize::hermes_event_from_rpc;
use crate::sources::{CanonicalEvent, SourceError};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// Open the WS stream. `auth_token` is sent in a `hello` envelope right
/// after connect (Hermes expects the token in the first frame, not in
/// the upgrade headers).
pub async fn open_ws(
    base_url: &str,
    auth_token: Option<&str>,
    channel_capacity: usize,
) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
    let url = format!("{}/api/events", base_url.replacen("http://", "ws://", 1));
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(SourceError::WebSocket)?;

    if let Some(tok) = auth_token {
        let hello = serde_json::json!({
            "method": "hello",
            "params": {"token": tok}
        });
        ws.send(Message::Text(hello.to_string()))
            .await
            .map_err(SourceError::WebSocket)?;
    }

    let (tx, rx) = mpsc::channel(channel_capacity);
    let tx = Arc::new(tx);

    tokio::spawn(async move {
        let mut seq: i64 = 0;
        let mut read = ws.split().1;

        while let Some(msg) = read.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(error = %e, "hermes ws read error; aborting");
                    break;
                }
            };

            let text = match msg {
                Message::Text(t) => t,
                Message::Binary(b) => match String::from_utf8(b) {
                    Ok(s) => s,
                    Err(_) => continue,
                },
                Message::Ping(_) | Message::Pong(_) => continue,
                Message::Close(_) => break,
                _ => continue,
            };

            let value: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Skip the hello ack.
            if value.get("method").and_then(|v| v.as_str()) == Some("hello") {
                continue;
            }

            let native = value
                .get("params")
                .and_then(|p| p.get("session_id").or_else(|| p.get("sessionId")))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let placeholder = format!("hermes:{native}");

            seq += 1;
            let canonical = hermes_event_from_rpc(&placeholder, seq, &value);
            if tx.send(canonical).await.is_err() {
                break;
            }
        }
    });

    Ok(rx)
}