//! Claude Code → canonical normalisation.
//!
//! Two wire formats come in:
//!
//! 1. **OTLP/HTTP JSON** — OpenTelemetry envelope emitted by Claude Code's
//!    experimental OTel bridge. Decoded in [`otel`], but the per-event shape
//!    is normalised here.
//! 2. **JSONL transcript** — one event per line in
//!    `~/.claude/projects/<project>/*.jsonl`. This is the source we read
//!    in Phase 1 because it works without any OTLP infrastructure and
//!    contains a faithful record of prompts, tool calls, and tool results.
//!
//! Both share the same downstream [`CanonicalEvent`] shape, so the
//! normaliser handles both flavours through [`claude_event_from_jsonl`]
//! and [`claude_event_from_otel`].

use crate::sources::{CanonicalEvent, CanonicalSession, EventKind};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

/// Best-effort mapping from a Claude JSONL record (or OTel-derived record)
/// to a [`CanonicalSession`] header. Claude doesn't always surface the
/// project / model fields, so they stay `None` when absent.
pub fn claude_session_from_jsonl(
    native_id: &str,
    source_path: &str,
    record: &Value,
) -> CanonicalSession {
    let obj = record.as_object();

    let title = obj
        .and_then(|o| o.get("title"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let cwd = obj
        .and_then(|o| o.get("cwd"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let project_dir = obj
        .and_then(|o| o.get("project_dir").or_else(|| o.get("project")))
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
        .map(str::to_owned);

    let started_at = obj
        .and_then(|o| o.get("started_at").or_else(|| o.get("timestamp")))
        .and_then(parse_timestamp)
        .unwrap_or(0.0);

    CanonicalSession {
        id: String::new(), // assigned by store on first insert
        source_id: "claude".to_string(),
        native_id: native_id.to_string(),
        title,
        project_dir,
        cwd,
        git_branch,
        model,
        started_at,
        ended_at: None,
        end_reason: None,
        input_tokens: 0,
        output_tokens: 0,
        cache_read: 0,
        cache_write: 0,
        reasoning_tokens: 0,
        cost_usd: 0.0,
        message_count: 0,
        tool_call_count: 0,
        parent_session_id: None,
        source_path: source_path.to_string(),
    }
}

/// Map one JSONL record to a [`CanonicalEvent`].
///
/// Claude Code's transcript format uses a `type` field with values like
/// `"user"`, `"assistant"`, `"tool_use"`, `"tool_result"`, `"system"`,
/// `"result"`, etc. Anything we don't recognise falls through to
/// `EventKind::Unknown` rather than erroring — the UI still gets a row.
pub fn claude_event_from_jsonl(
    session_id: &str,
    seq: i64,
    record: &Value,
) -> CanonicalEvent {
    let obj = record.as_object();

    let occurred_at = obj
        .and_then(|o| o.get("timestamp"))
        .and_then(parse_timestamp)
        .unwrap_or(0.0);

    let raw_kind = obj
        .and_then(|o| o.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let kind = match raw_kind {
        "user" => EventKind::UserPrompt,
        "assistant" => EventKind::AssistantText,
        "tool_use" => EventKind::ToolCall,
        "tool_result" => EventKind::ToolResult,
        "system" => EventKind::Auth,
        "result" => EventKind::ApiError, // session-end summary line
        "compaction" => EventKind::Compaction,
        "permission_decision" => EventKind::PermissionDecision,
        "subagent_start" => EventKind::SubagentStart,
        "subagent_end" => EventKind::SubagentEnd,
        "api_refusal" => EventKind::ApiRefusal,
        _ => EventKind::Unknown,
    };

    let tool_name = obj
        .and_then(|o| o.get("tool_name").or_else(|| o.get("name")))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    let tool_input_size = obj
        .and_then(|o| o.get("tool_input"))
        .and_then(|v| v.as_str().map(str::len).or_else(|| serde_json::to_string(v).ok().map(|s| s.len())))
        .map(|n| n as i64);

    let tool_result_size = obj
        .and_then(|o| o.get("tool_result").or_else(|| o.get("content")))
        .and_then(|v| v.as_str().map(str::len).or_else(|| serde_json::to_string(v).ok().map(|s| s.len())))
        .map(|n| n as i64);

    let duration_ms = obj
        .and_then(|o| o.get("duration_ms"))
        .and_then(|v| v.as_i64());

    let tokens_in = obj
        .and_then(|o| o.get("usage").and_then(|u| u.get("input_tokens")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let tokens_out = obj
        .and_then(|o| o.get("usage").and_then(|u| u.get("output_tokens")))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let cost_usd = obj
        .and_then(|o| o.get("cost_usd").or_else(|| o.get("cost")))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let model = obj
        .and_then(|o| o.get("model"))
        .and_then(|v| v.as_str())
        .map(str::to_owned);

    CanonicalEvent {
        session_id: session_id.to_string(),
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

/// Map one OTel-derived record (after the OTLP envelope has been decoded
/// in [`crate::sources::claude::otel`]) to a [`CanonicalEvent`].
///
/// OTel uses `event.name` for the operation; Claude Code's bridge sets
/// `gen_ai.*` attributes. We extract the canonical fields, fall back to
/// `Unknown` for anything new.
pub fn claude_event_from_otel(
    session_id: &str,
    seq: i64,
    record: &Value,
) -> CanonicalEvent {
    // OTel records share most of the same fields as the JSONL ones once
    // we've peeled the envelope — reuse the JSONL mapper.
    let mut event = claude_event_from_jsonl(session_id, seq, record);

    // OTel sometimes uses `attributes.gen_ai.response.model`.
    if event.model.is_none() {
        event.model = record
            .pointer("/attributes/gen_ai.response.model")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
    }

    if event.occurred_at == 0.0 {
        if let Some(ns) = record
            .pointer("/timeUnixNano")
            .and_then(|v| v.as_u64())
        {
            event.occurred_at = (ns as f64) / 1_000_000_000.0;
        }
    }

    event
}

/// Parse a timestamp that may arrive as either a Unix-seconds number or
/// an ISO-8601 string. Returns `None` on either missing or unparseable.
pub fn parse_timestamp(v: &Value) -> Option<f64> {
    if let Some(n) = v.as_f64() {
        return Some(n);
    }
    if let Some(s) = v.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp() as f64);
        }
        // Try plain ISO-8601 without timezone → assume UTC.
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
            return Some(Utc.from_utc_datetime(&naive).timestamp() as f64);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jsonl_user_prompt_maps_to_user_prompt_kind() {
        let rec = json!({
            "type": "user",
            "timestamp": "2026-07-05T10:00:00Z",
            "message": "hi"
        });
        let ev = claude_event_from_jsonl("sess-1", 1, &rec);
        assert_eq!(ev.kind, EventKind::UserPrompt);
        assert!(ev.occurred_at > 0.0);
    }

    #[test]
    fn jsonl_tool_use_captures_tool_name_and_input_size() {
        let rec = json!({
            "type": "tool_use",
            "timestamp": 1720000000.0,
            "name": "Bash",
            "tool_input": "{\"command\":\"ls\"}"
        });
        let ev = claude_event_from_jsonl("sess-1", 2, &rec);
        assert_eq!(ev.kind, EventKind::ToolCall);
        assert_eq!(ev.tool_name.as_deref(), Some("Bash"));
        assert_eq!(ev.tool_input_size, Some(20));
    }

    #[test]
    fn jsonl_unknown_type_falls_through_safely() {
        let rec = json!({"type": "future_kind", "timestamp": 1.0});
        let ev = claude_event_from_jsonl("sess-1", 3, &rec);
        assert_eq!(ev.kind, EventKind::Unknown);
    }
}