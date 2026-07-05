#!/usr/bin/env bash
#
# gen-types.sh — regenerate (or verify) the canonical TS shape files committed
# under src-tauri/gen/schemas/. These mirror the Rust canonical types in
# src-tauri/src/sources/mod.rs (`CanonicalSession`, `CanonicalEvent`,
# `EventKind`) and the store row types in src-tauri/src/store/schema.rs
# (`Session`, `Event`).
#
# The committed fallbacks exist so reviewers and CI can read the wire shapes
# without re-running Rust. When the `ts-rs` integration lands (see
# docs/plans/phase-1-iteration-2.md task T2-3), Stage 1 below will switch
# to invoking `cargo test --features ts-rs` and emit `target/bindings/*.ts`.
#
# Usage:
#   ./scripts/gen-types.sh           # regenerate into gen/schemas/
#   ./scripts/gen-types.sh --check   # fail if anything is out of date
#
# At runtime, the Rust `serde` structs are the SOURCE OF TRUTH. If you
# change one, regenerate with `./scripts/gen-types.sh`, commit the JSON,
# and update src/types/canonical.ts to mirror.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GEN_DIR="$ROOT/src-tauri/gen/schemas"

mkdir -p "$GEN_DIR"

CHECK_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --check) CHECK_ONLY=1 ;;
    -h|--help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# ---------------------------------------------------------------------------
# Stage 1 — emit would-be-regenerated JSON into TMP_DIR. When ts-rs lands
# (Iteration 2 task T2-3), replace this block with `cargo test --features
# ts-rs` and copy from `target/bindings/*.json`.
# ---------------------------------------------------------------------------

cat > "$TMP_DIR/CanonicalSession.canonical.json" <<'JSON'
{
  "$comment": "Mirrors spire_bridge_lib::sources::CanonicalSession. Regenerate via ./scripts/gen-types.sh after editing the Rust source.",
  "type": "object",
  "required": [
    "id", "source_id", "native_id", "started_at",
    "input_tokens", "output_tokens", "cache_read", "cache_write",
    "reasoning_tokens", "cost_usd", "message_count",
    "tool_call_count", "source_path"
  ],
  "properties": {
    "id":               { "type": "string" },
    "source_id":        { "type": "string", "description": "One of: 'claude', 'opencode', 'hermes'." },
    "native_id":        { "type": "string" },
    "title":            { "type": ["string", "null"] },
    "project_dir":      { "type": ["string", "null"] },
    "cwd":              { "type": ["string", "null"] },
    "git_branch":       { "type": ["string", "null"] },
    "model":            { "type": ["string", "null"] },
    "started_at":       { "type": "number", "description": "Unix seconds (f64)." },
    "ended_at":         { "type": ["number", "null"] },
    "end_reason":       { "type": ["string", "null"] },
    "input_tokens":     { "type": "integer" },
    "output_tokens":    { "type": "integer" },
    "cache_read":       { "type": "integer" },
    "cache_write":      { "type": "integer" },
    "reasoning_tokens": { "type": "integer" },
    "cost_usd":         { "type": "number" },
    "message_count":    { "type": "integer" },
    "tool_call_count":  { "type": "integer" },
    "parent_session_id":{ "type": ["string", "null"] },
    "source_path":      { "type": "string", "description": "Original file or URL the session came from." }
  }
}
JSON

cat > "$TMP_DIR/CanonicalEvent.canonical.json" <<'JSON'
{
  "$comment": "Mirrors spire_bridge_lib::sources::CanonicalEvent. Regenerate via ./scripts/gen-types.sh after editing the Rust source.",
  "type": "object",
  "required": [
    "session_id", "seq", "occurred_at", "kind",
    "payload", "cost_usd", "tokens_in", "tokens_out"
  ],
  "properties": {
    "session_id":        { "type": "string" },
    "seq":               { "type": "integer" },
    "occurred_at":       { "type": "number", "description": "Unix seconds (f64)." },
    "kind":              { "$ref": "EventKind.canonical.json" },
    "payload":           { "description": "Free-form JSON. Source-specific fields preserved verbatim." },
    "duration_ms":       { "type": ["integer", "null"] },
    "tool_name":         { "type": ["string", "null"] },
    "tool_input_size":   { "type": ["integer", "null"] },
    "tool_result_size":  { "type": ["integer", "null"] },
    "cost_usd":          { "type": "number" },
    "tokens_in":         { "type": "integer" },
    "tokens_out":        { "type": "integer" },
    "model":             { "type": ["string", "null"] }
  }
}
JSON

cat > "$TMP_DIR/EventKind.canonical.json" <<'JSON'
{
  "$comment": "Mirrors spire_bridge_lib::sources::EventKind. serde(rename_all = 'snake_case').",
  "type": "string",
  "enum": [
    "user_prompt",
    "assistant_text",
    "tool_call",
    "tool_result",
    "api_request",
    "api_error",
    "api_refusal",
    "compaction",
    "auth",
    "permission_decision",
    "subagent_start",
    "subagent_end",
    "unknown"
  ]
}
JSON

cat > "$TMP_DIR/SessionRow.canonical.json" <<'JSON'
{
  "$comment": "Mirrors spire_bridge_lib::store::schema::Session (DB row). Differs from CanonicalSession only by raw_json + updated_at.",
  "type": "object",
  "required": [
    "id", "source_id", "native_id", "started_at",
    "input_tokens", "output_tokens", "cache_read", "cache_write",
    "reasoning_tokens", "cost_usd", "message_count",
    "tool_call_count", "source_path", "updated_at"
  ],
  "properties": {
    "id":                 { "type": "string" },
    "source_id":          { "type": "string" },
    "native_id":          { "type": "string" },
    "title":              { "type": ["string", "null"] },
    "project_dir":        { "type": ["string", "null"] },
    "cwd":                { "type": ["string", "null"] },
    "git_branch":         { "type": ["string", "null"] },
    "model":              { "type": ["string", "null"] },
    "started_at":         { "type": "number" },
    "ended_at":           { "type": ["number", "null"] },
    "end_reason":         { "type": ["string", "null"] },
    "input_tokens":       { "type": "integer" },
    "output_tokens":      { "type": "integer" },
    "cache_read":         { "type": "integer" },
    "cache_write":        { "type": "integer" },
    "reasoning_tokens":   { "type": "integer" },
    "cost_usd":           { "type": "number" },
    "message_count":      { "type": "integer" },
    "tool_call_count":    { "type": "integer" },
    "parent_session_id":  { "type": ["string", "null"] },
    "raw_json":           { "type": ["string", "null"] },
    "source_path":        { "type": "string" },
    "updated_at":         { "type": "number" }
  }
}
JSON

cat > "$TMP_DIR/EventRow.canonical.json" <<'JSON'
{
  "$comment": "Mirrors spire_bridge_lib::store::schema::Event (DB row). Payload is already-redacted JSON text.",
  "type": "object",
  "required": [
    "session_id", "seq", "occurred_at", "kind",
    "payload", "cost_usd", "tokens_in", "tokens_out"
  ],
  "properties": {
    "id":                { "type": ["integer", "null"] },
    "session_id":        { "type": "string" },
    "seq":               { "type": "integer" },
    "occurred_at":       { "type": "number" },
    "kind":              { "$ref": "EventKind.canonical.json" },
    "payload":           { "type": "string" },
    "duration_ms":       { "type": ["integer", "null"] },
    "tool_name":         { "type": ["string", "null"] },
    "tool_input_size":   { "type": ["integer", "null"] },
    "tool_result_size":  { "type": ["integer", "null"] },
    "cost_usd":          { "type": "number" },
    "tokens_in":         { "type": "integer" },
    "tokens_out":        { "type": "integer" },
    "model":             { "type": ["string", "null"] }
  }
}
JSON

# ---------------------------------------------------------------------------
# Stage 2 — diff-vs-commit (--check) or copy (default).
# ---------------------------------------------------------------------------

DIFF_RC=0
for f in CanonicalSession CanonicalEvent EventKind SessionRow EventRow; do
  src="$GEN_DIR/$f.canonical.json"
  dst="$TMP_DIR/$f.canonical.json"
  if [ ! -f "$src" ]; then
    if [ "$CHECK_ONLY" -eq 1 ]; then
      echo "✗ $f.canonical.json missing — first run without --check to seed it." >&2
      DIFF_RC=1
      continue
    fi
  elif ! diff -u "$src" "$dst" >/dev/null 2>&1; then
    if [ "$CHECK_ONLY" -eq 1 ]; then
      echo "✗ $f.canonical.json is out of date." >&2
      echo "  Run: ./scripts/gen-types.sh" >&2
      DIFF_RC=1
    fi
  fi
done

if [ "$CHECK_ONLY" -eq 1 ]; then
  if [ "$DIFF_RC" -ne 0 ]; then
    exit 1
  fi
  echo "✓ all canonical type fallbacks up to date"
  exit 0
fi

for f in CanonicalSession CanonicalEvent EventKind SessionRow EventRow; do
  cp "$TMP_DIR/$f.canonical.json" "$GEN_DIR/$f.canonical.json"
done

echo "✓ regenerated 5 canonical type fallbacks in $GEN_DIR"
echo "  next: review the diff and commit them (do NOT push)."
