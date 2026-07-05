//! OpenCode → canonical normalisation.
//!
//! OpenCode exposes a REST API and an SSE event stream. The HTTP fetcher
//! returns rich session detail (title, model, token usage); the SSE stream
//! yields real-time events (message_created, message_updated,
//! tool_call_started, etc.). We normalise both into the canonical shape.

use crate::sources::{CanonicalEvent, CanonicalSession, EventKind};
use serde_json::Value;

/// Map a `/session` payload (list element or detail) into a canonical
/// session header.
pub fn opencode_session_from_http(
    native_id: &str,
    record: &Value,
) -> CanonicalSession {
    let obj = record.as_object();

    let title = obj
        .and_then(|o| o.get("title").or_else(|| o.get("name")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let cwd = obj
        .and_then(|o| o.get("cwd"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let project_dir = obj
        .and_then(|o| o.get("projectDir").or_else(|| o.get("project_dir")))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| cwd.clone());

    let git_branch = obj
        .and_then(|o| o.get("gitBranch").or_else(|| o.get("git_branch")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let model = obj
        .and_then(|o| o.get("model"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            obj.and_then(|o| o.get("modelID"))
                .and_then(|v| v.as_str())
        })
        .map(str::to_owned);

    let started_at = obj
        .and_then(|o| {
            o.get("startedAt")
                .or_else(|| o.get("started_at"))
                .or_else(|| o.get("createdAt"))
                .or_else(|| o.get("created_at"))
        })
        .and_then(crate::sources::claude::normalize::parse_timestamp)
        .unwrap_or(0.0);

    let ended_at = obj
        .and_then(|o| o.get("endedAt").or_else(|| o.get("ended_at")))
        .and_then(crate::sources::claude::normalize::parse_timestamp);

    let end_reason = obj
        .and_then(|o| o.get("endReason").or_else(|| o.get("end_reason")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    CanonicalSession {
        id: String::new(),
        source_id: "opencode".to_string(),
        native_id: native_id.to_string(),
        title,
        project_dir,
        cwd,
        git_branch,
        model,
        started_at,
        ended_at,
        end_reason,
        input_tokens: int_field(obj, "inputTokens"),
        output_tokens: int_field(obj, "outputTokens"),
        cache_read: int_field(obj, "cacheRead"),
        cache_write: int_field(obj, "cacheWrite"),
        reasoning_tokens: int_field(obj, "reasoningTokens"),
        cost_usd: float_field(obj, "costUsd").unwrap_or(0.0),
        message_count: int_field(obj, "messageCount"),
        tool_call_count: int_field(obj, "toolCallCount"),
        parent_session_id: obj
            .and_then(|o| o.get("parentSessionID"))
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        source_path: format!("opencode://session/{native_id}"),
    }
}

/// Map one `/message` payload to a canonical event.
pub fn opencode_event_from_message(
    session_id: &str,
    seq: i64,
    record: &Value,
) -> CanonicalEvent {
    let obj = record.as_object();

    let occurred_at = obj
        .and_then(|o| o.get("createdAt").or_else(|| o.get("created_at")))
        .and_then(crate::sources::claude::normalize::parse_timestamp)
        .unwrap_or(0.0);

    let role = obj
        .and_then(|o| o.get("role"))
        .and_then(|v| v.as_str())
        .unwrap_or("assistant");

    let kind = match role {
        "user" => EventKind::UserPrompt,
        "assistant" => EventKind::AssistantText,
        "tool" => EventKind::ToolResult,
        _ => EventKind::Unknown,
    };

    let tool_name = obj
        .and_then(|o| o.get("toolName").or_else(|| o.get("tool_name")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    CanonicalEvent {
        session_id: session_id.to_string(),
        seq,
        occurred_at,
        kind,
        payload: record.clone(),
        duration_ms: obj
            .and_then(|o| o.get("durationMs"))
            .and_then(|v| v.as_i64()),
        tool_name,
        tool_input_size: size_field(obj, "input"),
        tool_result_size: size_field(obj, "output"),
        cost_usd: float_field(obj, "costUsd").unwrap_or(0.0),
        tokens_in: int_field(obj, "inputTokens"),
        tokens_out: int_field(obj, "outputTokens"),
        model: obj
            .and_then(|o| o.get("model"))
            .and_then(|v| v.as_str())
            .map(str::to_owned),
    }
}

/// Map one SSE event (raw `data:` JSON) into a canonical event.
///
/// OpenCode SSE events carry a `type` field with values like
/// `message.created`, `message.updated`, `session.idle`,
/// `session.compacted`, etc. We map the subset the UI surfaces; new
/// types fall through to `Unknown`.
pub fn opencode_event_from_sse(
    session_id: &str,
    seq: i64,
    record: &Value,
) -> CanonicalEvent {
    let obj = record.as_object();

    let occurred_at = obj
        .and_then(|o| o.get("at").or_else(|| o.get("timestamp")))
        .and_then(crate::sources::claude::normalize::parse_timestamp)
        .unwrap_or(0.0);

    let kind_str = obj
        .and_then(|o| o.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let kind = match kind_str {
        "message.created" | "message.updated" => EventKind::AssistantText,
        "message.completed" => EventKind::AssistantText,
        "tool_call.started" | "tool_call.finished" => EventKind::ToolCall,
        "tool_call.failed" => EventKind::ApiError,
        "session.idle" => EventKind::ApiRequest,
        "session.compacted" | "session.compact" => EventKind::Compaction,
        "permission.granted" | "permission.denied" => EventKind::PermissionDecision,
        "auth.changed" | "auth.refreshed" => EventKind::Auth,
        "subagent.started" | "subagent.created" => EventKind::SubagentStart,
        "subagent.ended" | "subagent.completed" => EventKind::SubagentEnd,
        _ => EventKind::Unknown,
    };

    let tool_name = obj
        .and_then(|o| o.get("tool").or_else(|| o.get("toolName")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    CanonicalEvent {
        session_id: session_id.to_string(),
        seq,
        occurred_at,
        kind,
        payload: record.clone(),
        duration_ms: obj
            .and_then(|o| o.get("duration_ms"))
            .and_then(|v| v.as_i64()),
        tool_name,
        tool_input_size: size_field(obj, "input"),
        tool_result_size: size_field(obj, "output"),
        cost_usd: float_field(obj, "cost_usd").unwrap_or(0.0),
        tokens_in: int_field(obj, "input_tokens"),
        tokens_out: int_field(obj, "output_tokens"),
        model: obj
            .and_then(|o| o.get("model"))
            .and_then(|v| v.as_str())
            .map(str::to_owned),
    }
}

// --- helpers ------------------------------------------------------------

fn int_field(obj: Option<&serde_json::Map<String, Value>>, key: &str) -> i64 {
    obj.and_then(|o| o.get(key))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

fn float_field(obj: Option<&serde_json::Map<String, Value>>, key: &str) -> Option<f64> {
    obj.and_then(|o| o.get(key)).and_then(|v| v.as_f64())
}

fn size_field(obj: Option<&serde_json::Map<String, Value>>, key: &str) -> Option<i64> {
    obj.and_then(|o| o.get(key)).and_then(|v| {
        if let Some(s) = v.as_str() {
            Some(s.len() as i64)
        } else if v.is_object() || v.is_array() {
            serde_json::to_string(v).ok().map(|s| s.len() as i64)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn session_extracts_known_fields() {
        let body = json!({
            "id": "oc-abc",
            "title": "Fix login bug",
            "cwd": "/home/u/proj",
            "projectDir": "/home/u/proj",
            "gitBranch": "feat/login",
            "model": "anthropic/claude-sonnet-4",
            "startedAt": 1_700_000_000.0,
            "endedAt": 1_700_000_120.0,
            "endReason": "completed",
            "inputTokens": 100,
            "outputTokens": 200,
            "costUsd": 0.0123
        });
        let s = opencode_session_from_http("oc-abc", &body);
        assert_eq!(s.source_id, "opencode");
        assert_eq!(s.native_id, "oc-abc");
        assert_eq!(s.title.as_deref(), Some("Fix login bug"));
        assert_eq!(s.model.as_deref(), Some("anthropic/claude-sonnet-4"));
        assert_eq!(s.input_tokens, 100);
        assert_eq!(s.output_tokens, 200);
        assert!((s.cost_usd - 0.0123).abs() < 1e-9);
        assert_eq!(s.started_at, 1_700_000_000.0);
    }

    #[test]
    fn session_handles_missing_optional_fields() {
        let body = json!({"id": "oc-xyz"});
        let s = opencode_session_from_http("oc-xyz", &body);
        assert_eq!(s.title, None);
        assert_eq!(s.cost_usd, 0.0);
        assert_eq!(s.ended_at, None);
    }

    #[test]
    fn message_user_maps_to_user_prompt() {
        let body = json!({
            "role": "user",
            "createdAt": "2026-07-05T10:00:00Z",
            "input": {"text": "hi"}
        });
        let ev = opencode_event_from_message("oc-1", 1, &body);
        assert_eq!(ev.kind, EventKind::UserPrompt);
        assert!(ev.occurred_at > 0.0);
    }

    #[test]
    fn sse_tool_call_started_maps_to_tool_call() {
        let body = json!({
            "type": "tool_call.started",
            "at": 1_700_000_005.0,
            "tool": "Bash",
            "input": {"command": "ls"}
        });
        let ev = opencode_event_from_sse("oc-1", 2, &body);
        assert_eq!(ev.kind, EventKind::ToolCall);
        assert_eq!(ev.tool_name.as_deref(), Some("Bash"));
        assert!(ev.tool_input_size.unwrap() > 0);
    }

    #[test]
    fn sse_compaction_event_recognised() {
        let body = json!({"type": "session.compacted", "at": 1_700_000_010.0});
        let ev = opencode_event_from_sse("oc-1", 3, &body);
        assert_eq!(ev.kind, EventKind::Compaction);
    }
}