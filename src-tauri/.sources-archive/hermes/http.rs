//! Hermes REST client.
//!
//! Hermes exposes `GET /api/sessions?limit=N&offset=M` for history and
//! `GET /api/profiles/sessions` for per-profile grouping. Auth (when
//! configured) is `Authorization: Bearer <keyring token>`.

use crate::sources::hermes::normalize::{hermes_event_from_rpc, HermesSessionWire};
use crate::sources::{CanonicalEvent, CanonicalSession, SourceError};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct HermesClient {
    pub base_url: String,
    pub auth_token: Arc<Option<String>>,
    http: reqwest::Client,
}

impl HermesClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client builds with default config");
        Self {
            base_url: base_url.into(),
            auth_token: Arc::new(None),
            http,
        }
    }

    /// Install a bearer token (typically fetched from the OS keychain).
    pub fn with_token(mut self, token: String) -> Self {
        self.auth_token = Arc::new(Some(token));
        self
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(tok) = self.auth_token.as_ref() {
            req.bearer_auth(tok)
        } else {
            req
        }
    }

    /// Health probe — hits `/api/sessions?limit=1`.
    pub async fn health(&self) -> Result<(), SourceError> {
        let url = format!("{}/api/sessions", self.base_url);
        let req = self.apply_auth(self.http.get(&url)).query(&[("limit", 1)]);
        let resp = req.send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(SourceError::Upstream(format!(
                "GET {url} returned {}",
                resp.status()
            )))
        }
    }

    /// Page through `/api/sessions`.
    pub async fn list_sessions(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<HermesSessionWire>, SourceError> {
        let url = format!("{}/api/sessions", self.base_url);
        let req = self
            .apply_auth(self.http.get(&url))
            .query(&[("limit", limit), ("offset", offset)]);
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(SourceError::Upstream(format!(
                "GET {url} returned {}",
                resp.status()
            )));
        }
        let body: Value = resp.json().await?;
        let arr = body
            .as_array()
            .ok_or_else(|| SourceError::Decode("/api/sessions root is not array".into()))?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            match serde_json::from_value::<HermesSessionWire>(item.clone()) {
                Ok(s) => out.push(s),
                Err(_) => continue, // skip partial / future schemas
            }
        }
        Ok(out)
    }

    /// Fetch the raw event history for one session (Hermes doesn't expose
    /// a per-session message endpoint — instead we replay the WS stream
    /// from a checkpoint. This REST helper is reserved for Phase 2.)
    pub async fn session_events(
        &self,
        native_session_id: &str,
        since: Option<f64>,
    ) -> Result<Vec<CanonicalEvent>, SourceError> {
        let url = format!("{}/api/sessions/{native_session_id}/events");
        let mut req = self.apply_auth(self.http.get(&url));
        if let Some(s) = since {
            req = req.query(&[("since", s)]);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(SourceError::Upstream(format!(
                "GET {url} returned {}",
                resp.status()
            )));
        }
        let body: Value = resp.json().await?;
        let arr = body
            .as_array()
            .ok_or_else(|| SourceError::Decode("events root is not array".into()))?;

        let placeholder = format!("hermes:{native_session_id}");
        let mut out = Vec::with_capacity(arr.len());
        let mut seq: i64 = 0;
        for v in arr {
            seq += 1;
            out.push(hermes_event_from_rpc(&placeholder, seq, v));
        }
        Ok(out)
    }
}