//! Secret redaction.
//!
//! Strips common API key formats out of payloads before we persist them
//! to disk. The renderer never sees raw credentials — every stored event
//! passes through `redact()` first.
//!
//! Patterns are intentionally conservative; false positives (rare) are
//! preferred over false negatives (leaked keys).

use once_cell::sync::Lazy;
use regex::Regex;

static PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"sk-[A-Za-z0-9_-]{20,}").unwrap(),
        Regex::new(r"sk-ant-[A-Za-z0-9_-]{20,}").unwrap(),
        Regex::new(r"sk-proj-[A-Za-z0-9_-]{20,}").unwrap(),
        Regex::new(r"ghp_[A-Za-z0-9]{20,}").unwrap(),
        Regex::new(r"github_pat_[A-Za-z0-9_]{20,}").unwrap(),
        Regex::new(r"xoxb-[A-Za-z0-9-]{20,}").unwrap(),
        Regex::new(r"xoxp-[A-Za-z0-9-]{20,}").unwrap(),
        // Bearer headers: case-insensitive on the header name.
        Regex::new(r"(?i)(authorization:\s*bearer\s+)[A-Za-z0-9._\-]+").unwrap(),
        Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        // Generic "looks-like-a-key" 32+ char hex/base64 run, prefixed by
        // common field names. Catches vendor formats we haven't enumerated.
        Regex::new(r#"(?i)(api[_-]?key|token|secret)["']?\s*[:=]\s*["']?([A-Za-z0-9_\-]{32,})"#).unwrap(),
    ]
});

/// Returns `input` with all known secret patterns replaced by `[REDACTED]`.
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    for p in PATTERNS.iter() {
        out = p.replace_all(&out, "[REDACTED]").into_owned();
    }
    out
}

/// Convenience: redact a serialized JSON value (preserves structure).
pub fn redact_value(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::String(s) => serde_json::Value::String(redact(s)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(redact_value).collect())
        }
        serde_json::Value::Object(obj) => {
            serde_json::Value::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), redact_value(v)))
                    .collect(),
            )
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_openai_key() {
        let s = "hello sk-abcdefghijklmnopqrstuv world";
        assert!(redact(s).contains("[REDACTED]"));
    }

    #[test]
    fn redacts_anthropic_key() {
        let s = "config: sk-ant-api03-abcdefghijklmnopqrstuvwxyz";
        assert!(redact(s).contains("[REDACTED]"));
    }

    #[test]
    fn redacts_github_pat() {
        let s = "token=ghp_abcdefghijklmnopqrstuvwxyz1234";
        assert!(redact(s).contains("[REDACTED]"));
    }

    #[test]
    fn redacts_bearer_header() {
        let s = "Authorization: Bearer abc.def-ghi_jkl-mno";
        assert!(redact(s).contains("[REDACTED]"));
    }

    #[test]
    fn redacts_aws_access_key() {
        let s = "aws: AKIAIOSFODNN7EXAMPLE";
        assert!(redact(s).contains("[REDACTED]"));
    }

    #[test]
    fn leaves_normal_text_alone() {
        assert_eq!(redact("hello world"), "hello world");
        assert_eq!(redact("user said 'thanks!'"), "user said 'thanks!'");
    }

    #[test]
    fn redact_value_walks_json() {
        let v = serde_json::json!({
            "user": "ferre",
            "api_key": "sk-abcdefghijklmnopqrstuvwxyz1234",
            "nested": { "token": "ghp_long_enough_token_12345" },
            "items": ["sk-also_a_valid_key_xx_yyyyyy", "safe text"],
        });
        let redacted = redact_value(&v);
        let s = redacted.to_string();
        assert!(s.contains("[REDACTED]"));
        // Non-secret values pass through.
        assert!(s.contains("ferre"));
        assert!(s.contains("safe text"));
    }
}