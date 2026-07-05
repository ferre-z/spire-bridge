//! Canonical event/session types + `Source` trait.
//!
//! Every adapter (Claude Code, OpenCode, Hermes) normalises its upstream wire
//! format into the structures defined here. Higher layers (sync engine, IPC,
//! renderer) only ever speak these types — no per-source leakage.
//!
//! Conventions:
//! * Timestamps are Unix seconds (f64) so JSON round-trips through the
//!   renderer without timezone surprises.
//! * `payload` is a free-form `serde_json::Value` blob that holds the
//!   source-specific fields the renderer may want to surface verbatim
//!   (tool input schemas, raw error bodies, etc.).
//! * All public types are `Serialize + Deserialize` so they can ride Tauri
//!   IPC straight to the React layer.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

/// Canonical representation of one agent session, regardless of source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalSession {
    pub id: String,
    pub source_id: String,
    pub native_id: String,
    pub title: Option<String>,
    pub project_dir: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    pub started_at: f64,
    pub ended_at: Option<f64>,
    pub end_reason: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub reasoning_tokens: i64,
    pub cost_usd: f64,
    pub message_count: i64,
    pub tool_call_count: i64,
    pub parent_session_id: Option<String>,
    pub source_path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    UserPrompt, AssistantText, ToolCall, ToolResult,
    ApiRequest, ApiError, ApiRefusal, Compaction,
    Auth, PermissionDecision, SubagentStart, SubagentEnd,
    Unknown,
}

impl EventKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventKind::UserPrompt => "user_prompt",
            EventKind::AssistantText => "assistant_text",
            EventKind::ToolCall => "tool_call",
            EventKind::ToolResult => "tool_result",
            EventKind::ApiRequest => "api_request",
            EventKind::ApiError => "api_error",
            EventKind::ApiRefusal => "api_refusal",
            EventKind::Compaction => "compaction",
            EventKind::Auth => "auth",
            EventKind::PermissionDecision => "permission_decision",
            EventKind::SubagentStart => "subagent_start",
            EventKind::SubagentEnd => "subagent_end",
            EventKind::Unknown => "unknown",
        }
    }
    pub fn from_token(s: &str) -> Self {
        match s {
            "user_prompt" => EventKind::UserPrompt,
            "assistant_text" => EventKind::AssistantText,
            "tool_call" => EventKind::ToolCall,
            "tool_result" => EventKind::ToolResult,
            "api_request" => EventKind::ApiRequest,
            "api_error" => EventKind::ApiError,
            "api_refusal" => EventKind::ApiRefusal,
            "compaction" => EventKind::Compaction,
            "auth" => EventKind::Auth,
            "permission_decision" => EventKind::PermissionDecision,
            "subagent_start" => EventKind::SubagentStart,
            "subagent_end" => EventKind::SubagentEnd,
            _ => EventKind::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalEvent {
    pub session_id: String,
    pub seq: i64,
    pub occurred_at: f64,
    pub kind: EventKind,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub duration_ms: Option<i64>,
    pub tool_name: Option<String>,
    pub tool_input_size: Option<i64>,
    pub tool_result_size: Option<i64>,
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub model: Option<String>,
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("upstream: {0}")] Upstream(String),
    #[error("decode: {0}")] Decode(String),
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("http: {0}")] Http(#[from] reqwest::Error),
    #[error("websocket: {0}")] WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("json: {0}")] Json(#[from] serde_json::Error),
    #[error("not running")] NotRunning,
}

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> &'static str;
    async fn health(&self) -> Result<(), SourceError>;
    async fn backfill(&self, since: Option<f64>) -> Result<Vec<(CanonicalSession, Vec<CanonicalEvent>)>, SourceError>;
    async fn live_events(&self) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_round_trip() {
        for k in [EventKind::UserPrompt, EventKind::ToolCall, EventKind::Unknown] {
            assert_eq!(EventKind::from_token(k.as_str()), k);
        }
    }

    #[test]
    fn unknown_kind_for_garbage() {
        assert_eq!(EventKind::from_token("not_a_real_kind"), EventKind::Unknown);
    }

    #[test]
    fn canonical_session_serializes_to_json() {
        let s = CanonicalSession {
            id: "ses_abc".into(),
            source_id: "claude".into(),
            native_id: "ses_abc".into(),
            title: Some("gh CLI installation".into()),
            project_dir: None, cwd: Some("/home/u".into()), git_branch: None,
            model: Some("claude-opus-4".into()),
            started_at: 1783163383.519, ended_at: Some(1783163575.980),
            end_reason: Some("completed".into()),
            input_tokens: 8927, output_tokens: 190,
            cache_read: 25600, cache_write: 0, reasoning_tokens: 178,
            cost_usd: 0.42, message_count: 1, tool_call_count: 1,
            parent_session_id: None, source_path: "/home/u/.claude/projects/x.jsonl".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: CanonicalSession = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn canonical_event_with_tool_call() {
        let e = CanonicalEvent {
            session_id: "ses_x".into(), seq: 1, occurred_at: 1783163383.519,
            kind: EventKind::ToolCall,
            payload: serde_json::json!({"name": "Bash", "command": "gh --version"}),
            duration_ms: Some(230), tool_name: Some("Bash".into()),
            tool_input_size: Some(28), tool_result_size: Some(64),
            cost_usd: 0.0, tokens_in: 0, tokens_out: 0, model: None,
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"tool_call\""));
        assert!(json.contains("\"Bash\""));
    }
}
