-- 0001_init.sql — Spire Bridge canonical schema.
-- Applied via refinery on first boot; never edited after ship.

CREATE TABLE IF NOT EXISTS agent_source (
    id    TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    icon  TEXT NOT NULL,
    color TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS session (
    id                TEXT PRIMARY KEY,
    source_id         TEXT NOT NULL REFERENCES agent_source(id),
    native_id         TEXT NOT NULL,
    title             TEXT,
    project_dir       TEXT,
    cwd               TEXT,
    git_branch        TEXT,
    model             TEXT,
    started_at        REAL NOT NULL,
    ended_at          REAL,
    end_reason        TEXT,
    input_tokens      INTEGER NOT NULL DEFAULT 0,
    output_tokens     INTEGER NOT NULL DEFAULT 0,
    cache_read        INTEGER NOT NULL DEFAULT 0,
    cache_write       INTEGER NOT NULL DEFAULT 0,
    reasoning_tokens  INTEGER NOT NULL DEFAULT 0,
    cost_usd          REAL    NOT NULL DEFAULT 0,
    message_count     INTEGER NOT NULL DEFAULT 0,
    tool_call_count   INTEGER NOT NULL DEFAULT 0,
    parent_session_id TEXT,
    source_path       TEXT    NOT NULL DEFAULT '',
    updated_at        REAL    NOT NULL DEFAULT (unixepoch()),
    UNIQUE(source_id, native_id)
);
CREATE INDEX IF NOT EXISTS session_started_idx ON session(started_at DESC);
CREATE INDEX IF NOT EXISTS session_source_idx  ON session(source_id);

CREATE TABLE IF NOT EXISTS event (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id       TEXT    NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    seq              INTEGER NOT NULL,
    occurred_at      REAL    NOT NULL,
    kind             TEXT    NOT NULL,
    payload          TEXT    NOT NULL DEFAULT '{}',
    duration_ms      INTEGER,
    tool_name        TEXT,
    tool_input_size  INTEGER,
    tool_result_size INTEGER,
    cost_usd         REAL    NOT NULL DEFAULT 0,
    tokens_in        INTEGER NOT NULL DEFAULT 0,
    tokens_out       INTEGER NOT NULL DEFAULT 0,
    model            TEXT,
    UNIQUE(session_id, seq)
);
CREATE INDEX IF NOT EXISTS event_session_seq_idx  ON event(session_id, seq);
CREATE INDEX IF NOT EXISTS event_session_time_idx ON event(session_id, occurred_at);
CREATE INDEX IF NOT EXISTS event_kind_idx         ON event(kind);
CREATE INDEX IF NOT EXISTS event_tool_idx         ON event(tool_name);

CREATE TABLE IF NOT EXISTS host (
    id        TEXT PRIMARY KEY,
    label     TEXT NOT NULL,
    hostname  TEXT NOT NULL,
    added_at  REAL NOT NULL DEFAULT (unixepoch())
);