//! Spire Bridge top-level error type.
//!
//! Every fallible operation in the Rust core returns `AppResult<T>`. The
//! `Serialize` impl lets us return errors over Tauri IPC without extra
//! mapping — the renderer sees the stringified message verbatim.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io: {0}")]          Io(#[from] std::io::Error),
    #[error("sqlite: {0}")]      Sqlite(#[from] rusqlite::Error),
    #[error("http: {0}")]        Http(#[from] reqwest::Error),
    #[error("websocket: {0}")]  WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("json: {0}")]        Json(#[from] serde_json::Error),
    #[error("notify: {0}")]      Notify(#[from] notify::Error),
    #[error("not found: {0}")]   NotFound(String),
    #[error("upstream: {0}")]    Upstream(String),
    #[error("auth: {0}")]        Auth(String),
    #[error("other: {0}")]       Other(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;