/**
 * Canonical types shared between Rust core and React renderer.
 *
 * These mirror `src-tauri/src/sources/mod.rs::CanonicalSession` and
 * `CanonicalEvent`. Task 16 will land `pnpm gen:types` which runs
 * `ts-rs` to autogenerate this file from Rust; for now we keep it
 * hand-written so the frontend can ship without the codegen pipeline.
 *
 * Conventions:
 * - Timestamps are Unix seconds (f64) so JSON round-trips through
 *   IPC without timezone surprises (matches Rust).
 * - Optional fields use `T | null`, NOT `T | undefined`. Tauri IPC
 *   serialises `None` as `null` and that's what the renderer gets.
 * - `payload` is a free-form `Record<string, unknown>` blob. Sources
 *   stuff their source-specific fields in there; the UI surfaces
 *   them verbatim in tool-call cards.
 *
 * Strictness: `noUncheckedIndexedAccess` + `exactOptionalPropertyTypes`
 * are on (per Global Constraint #4), so callers that need to read
 * `payload.foo` must guard with `??` or `typeof === 'string'`.
 */

/** All known agent sources, seeded by migration 0002. */
export type SourceId = "claude_code" | "opencode" | "hermes" | string;

/**
 * Source metadata (label, icon name, accent colour). Mirrors the
 * `agent_source` table. The icon is a lucide-react key so the
 * sidebar can render it directly.
 */
export interface AgentSource {
  id: SourceId;
  label: string;
  icon: string;
  color: string;
}

/** Canonical session row. Field order mirrors Rust for diffing. */
export interface CanonicalSession {
  id: string;
  source_id: SourceId;
  native_id: string;
  title: string | null;
  project_dir: string | null;
  cwd: string | null;
  git_branch: string | null;
  model: string | null;
  started_at: number;
  ended_at: number | null;
  end_reason: string | null;
  input_tokens: number;
  output_tokens: number;
  cache_read: number;
  cache_write: number;
  reasoning_tokens: number;
  cost_usd: number;
  message_count: number;
  tool_call_count: number;
  parent_session_id: string | null;
  source_path: string;
}

/** All canonical event kinds. Matches `EventKind` in Rust. */
export type EventKind =
  | "user_prompt"
  | "assistant_text"
  | "tool_call"
  | "tool_result"
  | "api_request"
  | "api_error"
  | "api_refusal"
  | "compaction"
  | "auth"
  | "permission_decision"
  | "subagent_start"
  | "subagent_end"
  | "unknown";

/**
 * One live event. `payload` is opaque to the core — only the source
 * that produced it knows the shape. The renderer should narrow with
 * a `kind`-discriminated helper before reading fields.
 */
export interface CanonicalEvent {
  session_id: string;
  seq: number;
  occurred_at: number;
  kind: EventKind;
  payload: Record<string, unknown>;
  duration_ms: number | null;
  tool_name: string | null;
  tool_input_size: number | null;
  tool_result_size: number | null;
  cost_usd: number;
  tokens_in: number;
  tokens_out: number;
  model: string | null;
}

/** Filter shape for `list_sessions`. Mirrors Rust `SessionFilter`. */
export interface SessionFilter {
  source?: string | null;
  since?: number | null;
  until?: number | null;
  search?: string | null;
}

/** `get_session` returns both the session and its events. */
export interface SessionDetail {
  session: CanonicalSession;
  events: CanonicalEvent[];
}

/** Lightweight roll-up used by the overview screen. */
export interface DashboardStats {
  total_cost_usd: number;
  session_count: number;
  error_count: number;
}

/** UI-side settings. Mirrors the `Settings` struct in Rust. */
export interface Settings {
  hermes_password_set: boolean;
}

/** `stream_live_events` is invoked once to wire the bus; no payload. */
// (no args / return — the renderer then subscribes via `listen("live_event", …)`)
