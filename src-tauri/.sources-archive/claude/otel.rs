//! Claude OTel receiver (port 4318).
//!
//! Phase 1 starts the JSONL path only (`jsonl.rs`). This module exists as
//! a placeholder for the OTLP/HTTP JSON server we'll bolt on in Phase 2:
//!
//! - bind `127.0.0.1:4318` only (privacy rule, no `0.0.0.0`)
//! - accept `Content-Type: application/json` OTLP envelopes
//! - peel the envelope in [`decode_envelope`] and hand the inner records
//!   to the normaliser in [`super::normalize::claude_event_from_otel`]
//!
//! Why deferred: protobuf support requires `prost` + a build step, and
//! the JSONL transcript already gives us the same record fidelity without
//! the dep weight. When Claude's OTel output stabilises we'll flip this
//! module on.

use crate::sources::claude::normalize::claude_event_from_otel;
use crate::sources::{CanonicalEvent, EventKind, SourceError};
use serde_json::Value;

/// Decode a raw OTLP/HTTP-JSON body into per-event [`CanonicalEvent`]s.
///
/// The shape is `{"resourceSpans": [{"scopeSpans": [{"spans": [...]}]}]}`
/// — a tree of spans; each span may carry events under `span.events`. We
/// flatten both spans-with-event-name and span-events into a single
/// stream so the UI doesn't have to care which one it got.
///
/// Returns `Err(SourceError::Decode)` if the envelope isn't a JSON
/// object — protobuf envelopes are rejected for Phase 1.
pub fn decode_envelope(
    session_placeholder: &str,
    body: &[u8],
    seq_base: i64,
) -> Result<Vec<CanonicalEvent>, SourceError> {
    let value: Value = serde_json::from_slice(body).map_err(SourceError::Json)?;
    let root = value
        .as_object()
        .ok_or_else(|| SourceError::Decode("otlp root is not a JSON object".into()))?;

    let mut out = Vec::new();
    let mut seq = seq_base;

    // resourceSpans → scopeSpans → spans[]
    if let Some(resource_spans) = root.get("resourceSpans").and_then(|v| v.as_array()) {
        for rs in resource_spans {
            if let Some(scope_spans) = rs.get("scopeSpans").and_then(|v| v.as_array()) {
                for ss in scope_spans {
                    if let Some(spans) = ss.get("spans").and_then(|v| v.as_array()) {
                        for span in spans {
                            flatten_span(session_placeholder, &mut seq, span, &mut out);
                        }
                    }
                }
            }
        }
    }

    Ok(out)
}

fn flatten_span(
    session_id: &str,
    seq: &mut i64,
    span: &Value,
    out: &mut Vec<CanonicalEvent>,
) {
    // If the span itself is a Claude "event" (name matches a known kind),
    // emit it.
    if let Some(name) = span.get("name").and_then(|v| v.as_str()) {
        if claude_known_kind(name).is_some() {
            *seq += 1;
            out.push(claude_event_from_otel(session_id, *seq, span));
        }
    }

    // Then descend into span.events.
    if let Some(events) = span.get("events").and_then(|v| v.as_array()) {
        for ev in events {
            *seq += 1;
            // Wrap so the normaliser sees a `{type, ...}` shape.
            let mut wrapped = serde_json::Map::new();
            if let Some(name) = ev.get("name").and_then(|v| v.as_str()) {
                wrapped.insert("type".into(), Value::String(name.to_string()));
            }
            if let Some(attrs) = ev.get("attributes").and_then(|v| v.as_array()) {
                for a in attrs {
                    if let (Some(k), Some(v)) = (
                        a.get("key").and_then(|v| v.as_str()),
                        a.get("value").and_then(|v| v.get("stringValue")),
                    ) {
                        if let Some(s) = v.as_str() {
                            wrapped.insert(k.to_string(), Value::String(s.to_string()));
                        }
                    }
                }
            }
            if let Some(ts) = ev.get("timeUnixNano").cloned() {
                wrapped.insert("timeUnixNano".into(), ts);
            }
            let payload = Value::Object(wrapped);
            out.push(claude_event_from_otel(session_id, *seq, &payload));
        }
    }
}

fn claude_known_kind(name: &str) -> Option<EventKind> {
    match name {
        "user_prompt" => Some(EventKind::UserPrompt),
        "assistant_text" => Some(EventKind::AssistantText),
        "tool_call" => Some(EventKind::ToolCall),
        "tool_result" => Some(EventKind::ToolResult),
        "api_request" => Some(EventKind::ApiRequest),
        "api_error" => Some(EventKind::ApiError),
        "api_refusal" => Some(EventKind::ApiRefusal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_minimal_envelope() {
        let body = serde_json::to_vec(&serde_json::json!({
            "resourceSpans": [{
                "scopeSpans": [{
                    "spans": [{
                        "name": "user_prompt",
                        "timeUnixNano": 1_700_000_000_000_000_000_u64,
                        "attributes": []
                    }]
                }]
            }]
        }))
        .unwrap();
        let events = decode_envelope("claude:test", &body, 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::UserPrompt);
    }

    #[test]
    fn rejects_non_json_body() {
        let err = decode_envelope("claude:test", b"not json", 0);
        assert!(err.is_err());
    }
}