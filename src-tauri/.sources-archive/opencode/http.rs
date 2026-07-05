//! OpenCode REST client.
//!
//! Phase 1 endpoints we exercise:
//!
//! * `GET /session` — list (also acts as the health probe)
//! * `GET /session/:id/message` — message history for one session
//!
//! Both bind to `127.0.0.1:<port>` only. The default port for OpenCode's
//! local server is `4096` but the binary may pick another; we accept it
//! via [`OpenCodeSource::with_base_url`].

use crate::sources::opencode::normalize::{
    opencode_event_from_message, opencode_session_from_http,
};
use crate::sources::{CanonicalEvent, CanonicalSession, SourceError};
use serde_json::Value;

/// One page of session history.
#[derive(Debug, Clone)]
pub struct SessionPage {
    pub sessions: Vec<CanonicalSession>,
    pub next_offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct OpenCodeClient {
    pub base_url: String,
    http: reqwest::Client,
}

impl OpenCodeClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client builds with default config");
        Self {
            base_url: base_url.into(),
            http,
        }
    }

    /// Probe the server by hitting `/session`. Returns `Ok(())` on any
    /// successful HTTP response (even 0 results), `Err` otherwise.
    pub async fn health(&self) -> Result<(), SourceError> {
        let url = format!("{}/session", self.base_url);
        let resp = self.http.get(&url).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(SourceError::Upstream(format!(
                "GET {url} returned {}",
                resp.status()
            )))
        }
    }

    /// Page through `/session`.
    pub async fn list_sessions(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<SessionPage, SourceError> {
        let url = format!("{}/session", self.base_url);
        let resp = self
            .http
            .get(&url)
            .query(&[("limit", limit), ("offset", offset)])
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(SourceError::Upstream(format!(
                "GET {url} returned {}",
                resp.status()
            )));
        }
        let body: Value = resp.json().await?;

        let arr = body
            .as_array()
            .ok_or_else(|| SourceError::Decode("/session root is not an array".into()))?;

        let sessions: Vec<CanonicalSession> = arr
            .iter()
            .filter_map(|v| {
                let native = v.get("id").and_then(|x| x.as_str())?;
                Some(opencode_session_from_http(native, v))
            })
            .collect();

        let next_offset = if (arr.len() as u32) < limit {
            None
        } else {
            Some(offset + limit)
        };

        Ok(SessionPage {
            sessions,
            next_offset,
        })
    }

    /// Fetch the message history for one session.
    pub async fn messages(
        &self,
        native_session_id: &str,
        since: Option<f64>,
    ) -> Result<Vec<CanonicalEvent>, SourceError> {
        let url = format!(
            "{}/session/{}/message",
            self.base_url, native_session_id
        );
        let mut req = self.http.get(&url);
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

        let arr = body.as_array().ok_or_else(|| {
            SourceError::Decode("/session/:id/message root is not an array".into())
        })?;

        let placeholder = format!("opencode:{native_session_id}");
        let mut events = Vec::with_capacity(arr.len());
        let mut seq: i64 = 0;
        for m in arr {
            seq += 1;
            events.push(opencode_event_from_message(&placeholder, seq, m));
        }
        Ok(events)
    }
}