-- 0001_init.sql — Spire Bridge canonical schema (Phase 1).
--
-- Read-only v1: we observe sessions + events from Claude Code, OpenCode,
-- and Hermes. The renderer never mutates these tables directly — all writes
-- flow through the Rust store / sync engine.
--
-- PRAGMA foreign_keys = ON ensures session <-> event CASCADE deletes behave
-- correctly even though rusqlite's default mode disables FK enforcement.
--
-- journal_mode = WAL trades slightly larger on-disk footprint for much
-- better concurrent read performance, which matters once the renderer hits
-- this DB through Tauri IPC while the sync engine is writing.

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE agent_source (
  id    TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  icon  TEXT NOT NULL,
  color TEXT NOT NULL
);

CREATE TABLE session (
  id              TEXT PRIMARY KEY,
  source_id       TEXT NOT NULL REFERENCES agent_source(id),
  native_id       TEXT NOT NULL,
  title           TEXT,
  project_dir     TEXT,
  cwd             TEXT,
  git_branch      TEXT,
  model           TEXT,
  started_at      REAL NOT NULL,
  ended_at        REAL,
  end_reason      TEXT,
  input_tokens    INTEGER NOT NULL DEFAULT 0,
  output_tokens   INTEGER NOT NULL DEFAULT 0,
  cache_read      INTEGER NOT NULL DEFAULT 0,
  cache_write     INTEGER NOT NULL DEFAULT 0,
  reasoning_tokens INTEGER NOT NULL DEFAULT 0,
  cost_usd        REAL NOT NULL DEFAULT 0,
  message_count   INTEGER NOT NULL DEFAULT 0,
  tool_call_count INTEGER NOT NULL DEFAULT 0,
  parent_session_id TEXT,
  raw_json        TEXT,
  source_path     TEXT NOT NULL DEFAULT '',
  updated_at      REAL NOT NULL DEFAULT (unixepoch()),
  UNIQUE(source_id, native_id)
);
CREATE INDEX session_started_idx ON session(started_at DESC);
CREATE INDEX session_source_idx  ON session(source_id);

CREATE TABLE event (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id       TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
  seq              INTEGER NOT NULL,
  occurred_at      REAL NOT NULL,
  kind             TEXT NOT NULL,
  payload          TEXT NOT NULL,
  duration_ms      INTEGER,
  tool_name        TEXT,
  tool_input_size  INTEGER,
  tool_result_size INTEGER,
  cost_usd         REAL NOT NULL DEFAULT 0,
  tokens_in        INTEGER NOT NULL DEFAULT 0,
  tokens_out       INTEGER NOT NULL DEFAULT 0,
  model            TEXT,
  UNIQUE(session_id, seq)
);
CREATE INDEX event_session_seq_idx  ON event(session_id, seq);
CREATE INDEX event_session_time_idx ON event(session_id, occurred_at);
CREATE INDEX event_kind_idx         ON event(kind);
CREATE INDEX event_tool_idx         ON event(tool_name);

CREATE TABLE host (
  id        TEXT PRIMARY KEY,
  label     TEXT NOT NULL,
  hostname  TEXT NOT NULL,
  added_at  REAL NOT NULL DEFAULT (unixepoch())
);
