-- 0003_sync_meta.sql — sync engine cursors.
--
-- Generic key/value store used by the sync engine to persist resumption
-- cursors per source. Keys look like:
--   source:<id>:last_seq:<session_native_id>   — last event seq ingested
--   source:<id>:backfill_done                  — '1' once full backfill succeeded
--   source:<id>:last_live_at                   — wall-clock of last live event
--
-- The sync engine treats absence of a key as "never synced"; presence
-- with any value (including '0') means we have a cursor to resume from.

CREATE TABLE IF NOT EXISTS sync_meta (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at REAL NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS sync_meta_key_idx ON sync_meta(key);