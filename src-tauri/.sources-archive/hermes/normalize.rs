//! Hermes → canonical normalisation.
//!
//! Hermes Agent exposes a JSON-RPC WebSocket on
//! `ws://127.0.0.1:9119/api/events` plus a small REST surface for
//! historical replay (`GET /api/sessions`, `GET /api/profiles/sessions`).
//!
//! Wire format: every message is `{"method": "...", "params": {...}}`
//! (JSON-RPC 2.0 style, but Hermes uses its own method namespace rather
//! than `rpc.call`). The normaliser pulls out the payload fields by
//! method name.

use crate::sources::{CanonicalEvent, CanonicalSession, EventKind};
use serde_json::Value;

/// Hermes session shape returned by `GET /api/sessions`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HermesSessionWire {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub project_dir: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub started_at: Option<f64>,
    #[serde(default)]
    pub ended_at: Option<f64>,
    #[serde(default)]
    pub end_reason: Option<String>,
    #[serde(default)]
    pub input_tokens: Option<i64>,
    #[serde(default)]
    pub output_tokens: Option<i64>,
    #[serde(default)]
    pub cache_read: Option<i64>,
    #[serde(default)]
    pub cache_write: Option<i64>,
    #[serde(default)]
    pub reasoning_tokens: Option<i64>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub message_count: Option<i64>,
    #[serde(default)]
    pub tool_call_count: Option<i64>,
    #[serde(default)]
    pub parent_session_id: Option<String>,
}

impl HermesSessionWire {
    pub fn into_canonical(self) -> CanonicalSession {
        CanonicalSession {
            id: String::new(),
            source_id: "hermes".to_string(),
            native_id: self.id,
            title: self.title,
            project_dir: self.project_dir.or_else(|| self.cwd.clone()),
            cwd: self.cwd,
            git_branch: self.git_branch,
            model: self.model,
            started_at: self.started_at.unwrap_or(0.0),
            ended_at: self.ended_at,
            end_reason: self.end_reason,
            input_tokens: self.input_tokens.unwrap_or(0),
            output_tokens: self.output_tokens.unwrap_or(0),
            cache_read: self.cache_read.unwrap_or(0),
            cache_write: self.cache_write.unwrap_or(0),
            reasoning_tokens: self.reasoning_tokens.unwrap_or(0),
            cost_usd: self.cost_usd.unwrap_or(0.0),
            message_count: self.message_count.unwrap_or(0),
            tool_call_count: self.tool_call_count.unwrap_or(0),
            parent_session_id: self.parent_session_id,
            source_path: "hermes://api/sessions".to_string(),
        }
    }
}

/// Map one Hermes JSON-RPC message to a canonical event.
///
/// `record` is the full `{"method": "...", "params": {...}}` envelope.
pub fn hermes_event_from_rpc(session_id: &str, seq: i64, record: &Value) -> CanonicalEvent {
    let obj = record.as_object();

    let method = obj
        .and_then(|o| o.get("method"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let params = obj.and_then(|o| o.get("params")).cloned().unwrap_or(Value::Null);

    let kind = match method {
        "session.user_prompt" | "user.message" => EventKind::UserPrompt,
        "session.assistant_text" | "assistant.message" => EventKind::AssistantText,
        "session.tool_call" | "tool.call" | "tool.start" => EventKind::ToolCall,
        "session.tool_result" | "tool.result" | "tool.end" => EventKind::ToolResult,
        "session.api_request" | "api.request" => EventKind::ApiRequest,
        "session.api_error" | "api.error" => EventKind::ApiError,
        "session.api_refusal" | "api.refusal" => EventKind::ApiRefusal,
        "session.compaction" | "session.compact" => EventKind::Compaction,
        "session.auth" | "auth.changed" => EventKind::Auth,
        "session.permission_decision" | "permission.set" => EventKind::PermissionDecision,
        "session.subagent_start" | "subagent.start" => EventKind::SubagentStart,
        "session.subagent_end" | "subagent.end" => EventKind::SubagentEnd,
        _ => EventKind::Unknown,
    };

    let occurred_at = obj
        .and_then(|o| o.get("timestamp").or_else(|| o.get("ts")))
        .and_then(crate::sources::claude::normalize::parse_timestamp)
        .or_else(|| {
            params
                .as_object()
                .and_then(|p| p.get("timestamp").or_else(|| p.get("ts")))
                .and_then(crate::sources::claude::normalize::parse_timestamp)
        })
        .unwrap_or(0.0);

    let tool_name = params
        .as_object()
        .and_then(|p| p.get("tool_name").or_else(|| p.get("name")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let tool_input_size = params
        .as_object()
        .and_then(|p| p.get("tool_input").or_else(|| p.get("input")))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.len() as i64)
            } else if v.is_object() || v.is_array() {
                serde_json::to_string(v).ok().map(|s| s.len() as i64)
            } else {
                None
            }
        });

    let tool_result_size = params
        .as_object()
        .and_then(|p| p.get("tool_result").or_else(|| p.get("result")))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.len() as i64)
            } else if v.is_object() || v.is_array() {
                serde_json::to_string(v).ok().map(|s| s.len() as i64)
            } else {
                None
            }
        });

    let duration_ms = params
        .as_object()
        .and_then(|p| p.get("duration_ms").or_else(|| p.get("durationMs")))
        .and_then(|v| v.as_i64());

    let cost_usd = params
        .as_object()
        .and_then(|p| p.get("cost_usd").or_else(|| p.get("costUsd")))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let tokens_in = params
        .as_object()
        .and_then(|p| p.get("input_tokens").or_else(|| p.get("inputTokens")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let tokens_out = params
        .as_object()
        .and_then(|p| p.get("output_tokens").or_else(|| p.get("outputTokens")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let model = params
        .as_object()
        .and_then(|p| p.get("model"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    // The session id Hermes sends for a per-session event lives in
    // params.session_id. We default to the placeholder the caller passed
    // if absent.
    let resolved_session = params
        .as_object()
        .and_then(|p| p.get("session_id").or_else(|| p.get("sessionId")))
        .and_then(|v| v.as_str())
        .unwrap_or(session_id);

    CanonicalEvent {
        session_id: resolved_session.to_string(),
        seq,
        occurred_at,
        kind,
        payload: record.clone(),
        duration_ms,
        tool_name,
        tool_input_size,
        tool_result_size,
        cost_usd,
        tokens_in,
        tokens_out,
        model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn session_wire_round_trip() {
        let wire = HermesSessionWire {
            id: "h-1".into(),
            title: Some("title".into()),
            cwd: Some("/x".into()),
            project_dir: Some("/x".into()),
            git_branch: Some("main".into()),
            model: Some("hermes-1".into()),
            started_at: Some(1.0),
            ended_at: Some(2.0),
            end_reason: Some("completed".into()),
            input_tokens: Some(10),
            output_tokens: Some(20),
            cache_read: Some(0),
            cache_write: Some(0),
            reasoning_tokens: Some(0),
            cost_usd: Some(0.05),
            message_count: Some(3),
            tool_call_count: Some(1),
            parent_session_id: None,
        };
        let c = wire.into_canonical();
        assert_eq!(c.source_id, "hermes");
        assert_eq!(c.native_id, "h-1");
        assert_eq!(c.cost_usd, 0.05);
        assert_eq!(c.input_tokens, 10);
    }

    #[test]
    fn rpc_user_prompt_maps_to_user_prompt() {
        let env = json!({
            "method": "session.user_prompt",
            "params": {"session_id": "h-1", "text": "hi"},
            "timestamp": 1_700_000_000.0
        });
        let ev = hermes_event_from_rpc("h-1", 1, &env);
        assert_eq!(ev.kind, EventKind::UserPrompt);
        assert_eq!(ev.session_id, "h-1");
    }

    #[test]
    fn rpc_tool_call_extracts_tool_name_and_cost() {
        let env = json!({
            "method": "session.tool_call",
            "params": {
                "session_id": "h-2",
                "tool_name": "grep",
                "tool_input": {"q": "foo"},
                "cost_usd": 0.0001,
                "duration_ms": 42
            },
            "timestamp": 1_700_000_010.0
        });
        let ev = hermes_event_from_rpc("h-2", 1, &env);
        assert_eq!(ev.kind, EventKind::ToolCall);
        assert_eq!(ev.tool_name.as_deref(), Some("grep"));
        assert!(ev.tool_input_size.unwrap() > 0);
        assert_eq!(ev.duration_ms, Some(42));
        assert!((ev.cost_usd - 0.0001).abs() < 1e-9);
    }

    #[test]
    fn rpc_unknown_method_falls_through() {
        let env = json!({"method": "future.event", "params": {}});
        let ev = hermes_event_from_rpc("h-3", 1, &env);
        assert_eq!(ev.kind, EventKind::Unknown);
    }
}