//! Spire Bridge SQLite store.
//!
//! Phase 1 ships an in-memory-friendly minimal API:
//! - `Store::open(path)` — creates DB file, applies migrations, returns handle
//! - `Store::upsert_session` / `upsert_event` — idempotent writes
//! - `Store::list_sessions` / `get_session` / `list_events` — reads
//! - `Store::dashboard_stats` — aggregate for the overview screen
//!
//! Concurrency: a single `Mutex<Connection>` is enough for v1 (we run one
//! sync engine, one renderer). Move to a connection pool if contention
//! becomes a problem (it won't with 5+ hosts / 20+ agents).
//!
//! Refinery integration: migrations are loaded at startup via the
//! `migrations/` directory next to `Cargo.toml`.

use crate::error::{AppError, AppResult};
use crate::sources::{CanonicalEvent, CanonicalSession, EventKind};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub mod redact;
pub mod schema;

/// All SQL migrations, in version order. Keep this in sync with the
/// `migrations/` directory; we read the SQL at compile time so the store
/// is portable without a filesystem read at runtime.
const MIGRATION_0001: &str = include_str!("../../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../../migrations/0002_seed_sources.sql");
const MIGRATION_0003: &str = include_str!("../../migrations/0003_sync_meta.sql");

/// One row in `meta` (lazy-created). Used for sync cursors (last seq, last
/// timestamp) per source — Task 5 leans on this.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// Borrow the underlying SQLite connection for the duration of `f`.
    /// The closure runs synchronously; callers wrap it in
    /// `tokio::task::spawn_blocking` when on the Tauri runtime.
    ///
    /// Used by IPC handlers (Task 6) and sync engine helpers that want
    /// raw `&Connection` access (for `store::schema::*` free functions).
    pub fn with_conn<R>(
        &self,
        f: impl FnOnce(&Connection) -> AppResult<R>,
    ) -> AppResult<R> {
        let conn = self.conn.lock();
        f(&conn)
    }

    /// Borrow the underlying connection's mutex guard. Used by the IPC
    /// layer for ad-hoc queries that don't have a helper method yet.
    pub fn conn_ref(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }

    /// Open (or create) the SQLite database at `path` and apply migrations.
    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;")?;
        // Naive migration runner — schema_version table tracks the highest
        // applied version. Refinery would be nicer; for v1 a 50-line loop
        // ships in half the time and has zero new deps.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);",
        )?;
        let applied: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if applied < 1 {
            conn.execute_batch(MIGRATION_0001)?;
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
        }
        if applied < 2 {
            conn.execute_batch(MIGRATION_0002)?;
            conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
        }
        if applied < 3 {
            conn.execute_batch(MIGRATION_0003)?;
            conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])?;
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an in-memory database with migrations applied — handy for tests.
    #[cfg(test)]
    pub fn open_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE schema_version (version INTEGER PRIMARY KEY);",
        )?;
        conn.execute_batch(MIGRATION_0001)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
        conn.execute_batch(MIGRATION_0002)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
        conn.execute_batch(MIGRATION_0003)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Idempotent session write. Updates aggregate fields if a row with
    /// `(source_id, native_id)` already exists.
    pub fn upsert_session(&self, s: &CanonicalSession) -> AppResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO session (
                id, source_id, native_id, title, project_dir, cwd, git_branch, model,
                started_at, ended_at, end_reason,
                input_tokens, output_tokens, cache_read, cache_write, reasoning_tokens,
                cost_usd, message_count, tool_call_count, parent_session_id, source_path,
                updated_at
             ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?, unixepoch())
             ON CONFLICT(source_id, native_id) DO UPDATE SET
                title           = excluded.title,
                project_dir     = excluded.project_dir,
                cwd             = excluded.cwd,
                git_branch      = excluded.git_branch,
                model           = excluded.model,
                started_at      = excluded.started_at,
                ended_at        = excluded.ended_at,
                end_reason      = excluded.end_reason,
                input_tokens    = excluded.input_tokens,
                output_tokens   = excluded.output_tokens,
                cache_read      = excluded.cache_read,
                cache_write     = excluded.cache_write,
                reasoning_tokens= excluded.reasoning_tokens,
                cost_usd        = excluded.cost_usd,
                message_count   = excluded.message_count,
                tool_call_count = excluded.tool_call_count,
                parent_session_id = excluded.parent_session_id,
                source_path     = excluded.source_path,
                updated_at      = unixepoch()",
            params![
                s.id, s.source_id, s.native_id, s.title, s.project_dir, s.cwd,
                s.git_branch, s.model, s.started_at, s.ended_at, s.end_reason,
                s.input_tokens, s.output_tokens, s.cache_read, s.cache_write,
                s.reasoning_tokens, s.cost_usd, s.message_count, s.tool_call_count,
                s.parent_session_id, s.source_path,
            ],
        )?;
        Ok(())
    }

    /// Idempotent event write. `(session_id, seq)` is the unique key; a
    /// duplicate insert is silently ignored so live replay is safe.
    pub fn upsert_event(&self, e: &CanonicalEvent) -> AppResult<()> {
        let conn = self.conn.lock();
        let payload_str = serde_json::to_string(&e.payload)?;
        let res = conn.execute(
            "INSERT OR IGNORE INTO event (
                session_id, seq, occurred_at, kind, payload,
                duration_ms, tool_name, tool_input_size, tool_result_size,
                cost_usd, tokens_in, tokens_out, model
             ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                e.session_id, e.seq, e.occurred_at, e.kind.as_str(), payload_str,
                e.duration_ms, e.tool_name, e.tool_input_size, e.tool_result_size,
                e.cost_usd, e.tokens_in, e.tokens_out, e.model,
            ],
        );
        match res {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
                // UNIQUE(session_id, seq) — already inserted, fine.
                Ok(())
            }
            Err(e) => Err(AppError::Sqlite(e)),
        }
    }

    /// List sessions, newest first. `limit` caps the result; `offset`
    /// paginates.
    pub fn list_sessions(&self, limit: u32, offset: u32) -> AppResult<Vec<CanonicalSession>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, source_id, native_id, title, project_dir, cwd, git_branch, model,
                    started_at, ended_at, end_reason,
                    input_tokens, output_tokens, cache_read, cache_write, reasoning_tokens,
                    cost_usd, message_count, tool_call_count, parent_session_id, source_path
             FROM session ORDER BY started_at DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit as i64, offset as i64], row_to_session)?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn count_sessions(&self) -> AppResult<i64> {
        let conn = self.conn.lock();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM session", [], |r| r.get(0))?;
        Ok(n)
    }

    pub fn get_session(&self, id: &str) -> AppResult<Option<CanonicalSession>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, source_id, native_id, title, project_dir, cwd, git_branch, model,
                    started_at, ended_at, end_reason,
                    input_tokens, output_tokens, cache_read, cache_write, reasoning_tokens,
                    cost_usd, message_count, tool_call_count, parent_session_id, source_path
             FROM session WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![id], row_to_session).optional()?;
        Ok(row)
    }

    pub fn list_events(&self, session_id: &str, limit: u32) -> AppResult<Vec<CanonicalEvent>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT session_id, seq, occurred_at, kind, payload,
                    duration_ms, tool_name, tool_input_size, tool_result_size,
                    cost_usd, tokens_in, tokens_out, model
             FROM event WHERE session_id = ?1 ORDER BY seq ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![session_id, limit as i64], row_to_event)?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    /// Aggregate for the overview screen.
    pub fn dashboard_stats(&self, since: f64) -> AppResult<DashboardStats> {
        let conn = self.conn.lock();
        let total_cost: f64 = conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM session WHERE started_at >= ?1",
            params![since],
            |r| r.get(0),
        )?;
        let session_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM session WHERE started_at >= ?1",
            params![since],
            |r| r.get(0),
        )?;
        let error_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM event WHERE occurred_at >= ?1 AND kind = 'api_error'",
            params![since],
            |r| r.get(0),
        )?;
        Ok(DashboardStats {
            total_cost_usd: total_cost,
            session_count,
            error_count,
        })
    }

    // -- sync_meta cursor helpers (Task 5) -----------------------------
    //
    // The sync engine needs to remember, per (source, session) pair, the
    // last `seq` it persisted so the next backfill can resume. The
    // key/value shape is intentionally generic so the engine can store
    // arbitrary cursors without a schema change every time we add one.

    /// Read a string value from `sync_meta`. Returns `None` if the key
    /// was never written.
    pub fn meta_get(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.conn.lock();
        let v: Option<String> = conn
            .query_row(
                "SELECT value FROM sync_meta WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .ok();
        Ok(v)
    }

    /// Write a string value to `sync_meta` (UPSERT).
    pub fn meta_set(&self, key: &str, value: &str) -> AppResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sync_meta (key, value, updated_at) VALUES (?1, ?2, unixepoch())
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a key from `sync_meta`. No-op if missing.
    pub fn meta_delete(&self, key: &str) -> AppResult<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM sync_meta WHERE key = ?1", params![key])?;
        Ok(())
    }

    /// Convenience: read the last-seen `seq` for a (source, session) pair.
    /// Returns `None` if we have never ingested this session before.
    pub fn last_seq_for(&self, source_id: &str, native_session_id: &str) -> AppResult<Option<i64>> {
        let key = format!("source:{source_id}:last_seq:{native_session_id}");
        match self.meta_get(&key)? {
            Some(s) => Ok(s.parse::<i64>().ok()),
            None => Ok(None),
        }
    }

    /// Convenience: persist the last-seen `seq` for a (source, session) pair.
    pub fn set_last_seq(&self, source_id: &str, native_session_id: &str, seq: i64) -> AppResult<()> {
        let key = format!("source:{source_id}:last_seq:{native_session_id}");
        self.meta_set(&key, &seq.to_string())
    }

    /// Insert a batch of events inside a single SQLite transaction.
    /// Used by the sync engine (Task 5) for backfill ingest. Each
    /// chunk of up to `batch_size` rows is one `BEGIN .. COMMIT`,
    /// which is roughly 10× faster than per-row commits on SQLite
    /// with WAL.
    ///
    /// Returns the number of rows that actually landed (i.e. were
    /// not absorbed by the UNIQUE(session_id, seq) dedupe).
    ///
    /// The caller is responsible for ensuring each event's payload is
    /// already redacted — this method writes whatever it's given.
    pub fn insert_batch(
        &self,
        events: &[crate::sources::CanonicalEvent],
        batch_size: usize,
    ) -> AppResult<usize> {
        if events.is_empty() {
            return Ok(0);
        }
        let batch_size = batch_size.max(1);
        let mut conn = self.conn.lock();
        let mut inserted_total = 0usize;
        for chunk in events.chunks(batch_size) {
            let tx = conn.transaction()?;
            for e in chunk {
                let payload_str = serde_json::to_string(&e.payload)?;
                // `INSERT OR IGNORE` + UNIQUE(session_id, seq) gives us
                // idempotent ingest: a replay never blows up.
                let res = tx.execute(
                    "INSERT OR IGNORE INTO event (
                        session_id, seq, occurred_at, kind, payload,
                        duration_ms, tool_name, tool_input_size, tool_result_size,
                        cost_usd, tokens_in, tokens_out, model
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        e.session_id, e.seq, e.occurred_at, e.kind.as_str(), payload_str,
                        e.duration_ms, e.tool_name, e.tool_input_size, e.tool_result_size,
                        e.cost_usd, e.tokens_in, e.tokens_out, e.model,
                    ],
                );
                match res {
                    Ok(0) => { /* ignored — duplicate seq */ }
                    Ok(_) => inserted_total += 1,
                    Err(e) => return Err(AppError::Sqlite(e)),
                }
            }
            tx.commit()?;
        }
        Ok(inserted_total)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DashboardStats {
    pub total_cost_usd: f64,
    pub session_count: i64,
    pub error_count: i64,
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<CanonicalSession> {
    Ok(CanonicalSession {
        id:               row.get(0)?,
        source_id:        row.get(1)?,
        native_id:        row.get(2)?,
        title:            row.get(3)?,
        project_dir:      row.get(4)?,
        cwd:              row.get(5)?,
        git_branch:       row.get(6)?,
        model:            row.get(7)?,
        started_at:       row.get(8)?,
        ended_at:         row.get(9)?,
        end_reason:       row.get(10)?,
        input_tokens:     row.get(11)?,
        output_tokens:    row.get(12)?,
        cache_read:       row.get(13)?,
        cache_write:      row.get(14)?,
        reasoning_tokens: row.get(15)?,
        cost_usd:         row.get(16)?,
        message_count:    row.get(17)?,
        tool_call_count:  row.get(18)?,
        parent_session_id: row.get(19)?,
        source_path:      row.get(20)?,
    })
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<CanonicalEvent> {
    let payload_str: String = row.get(4)?;
    let payload = serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
    Ok(CanonicalEvent {
        session_id:       row.get(0)?,
        seq:              row.get(1)?,
        occurred_at:      row.get(2)?,
        kind:             EventKind::from_token(&row.get::<_, String>(3)?),
        payload,
        duration_ms:      row.get(5)?,
        tool_name:        row.get(6)?,
        tool_input_size:  row.get(7)?,
        tool_result_size: row.get(8)?,
        cost_usd:         row.get(9)?,
        tokens_in:        row.get(10)?,
        tokens_out:       row.get(11)?,
        model:            row.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::EventKind;

    fn fixture_session(id: &str, started_at: f64) -> CanonicalSession {
        CanonicalSession {
            id: id.into(),
            source_id: "claude".into(),
            native_id: id.into(),
            title: Some(format!("session {id}")),
            project_dir: None, cwd: None, git_branch: None, model: Some("claude-opus-4".into()),
            started_at, ended_at: None, end_reason: None,
            input_tokens: 100, output_tokens: 50,
            cache_read: 0, cache_write: 0, reasoning_tokens: 10,
            cost_usd: 0.01, message_count: 1, tool_call_count: 1,
            parent_session_id: None, source_path: String::new(),
        }
    }

    #[test]
    fn migrations_apply_on_in_memory_open() {
        let store = Store::open_memory().expect("memory open");
        let n = store.count_sessions().unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn seed_sources_present() {
        let store = Store::open_memory().unwrap();
        let conn = store.conn.lock();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM agent_source", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 3);
    }

    #[test]
    fn upsert_and_list_sessions_orders_newest_first() {
        let store = Store::open_memory().unwrap();
        store.upsert_session(&fixture_session("ses_a", 100.0)).unwrap();
        store.upsert_session(&fixture_session("ses_b", 300.0)).unwrap();
        store.upsert_session(&fixture_session("ses_c", 200.0)).unwrap();
        let list = store.list_sessions(10, 0).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].id, "ses_b");
        assert_eq!(list[1].id, "ses_c");
        assert_eq!(list[2].id, "ses_a");
    }

    #[test]
    fn upsert_session_is_idempotent() {
        let store = Store::open_memory().unwrap();
        store.upsert_session(&fixture_session("ses_a", 100.0)).unwrap();
        let mut s = fixture_session("ses_a", 100.0);
        s.cost_usd = 99.0;
        store.upsert_session(&s).unwrap();
        let back = store.get_session("ses_a").unwrap().unwrap();
        assert_eq!(back.cost_usd, 99.0);
    }

    #[test]
    fn upsert_event_dedupes_on_seq_collision() {
        let store = Store::open_memory().unwrap();
        store.upsert_session(&fixture_session("ses_a", 100.0)).unwrap();
        let e = CanonicalEvent {
            session_id: "ses_a".into(), seq: 1, occurred_at: 100.5,
            kind: EventKind::UserPrompt, payload: serde_json::json!({"text": "hi"}),
            duration_ms: None, tool_name: None,
            tool_input_size: None, tool_result_size: None,
            cost_usd: 0.0, tokens_in: 0, tokens_out: 0, model: None,
        };
        store.upsert_event(&e).unwrap();
        store.upsert_event(&e).unwrap(); // duplicate
        let list = store.list_events("ses_a", 100).unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn dashboard_stats_aggregate_cost_and_errors() {
        let store = Store::open_memory().unwrap();
        store.upsert_session(&fixture_session("ses_a", 100.0)).unwrap();
        store.upsert_session(&fixture_session("ses_b", 200.0)).unwrap();
        let err = CanonicalEvent {
            session_id: "ses_a".into(), seq: 1, occurred_at: 105.0,
            kind: EventKind::ApiError, payload: serde_json::json!({"msg": "boom"}),
            duration_ms: Some(500), tool_name: None,
            tool_input_size: None, tool_result_size: None,
            cost_usd: 0.0, tokens_in: 0, tokens_out: 0, model: None,
        };
        store.upsert_event(&err).unwrap();
        let stats = store.dashboard_stats(0.0).unwrap();
        assert_eq!(stats.session_count, 2);
        assert_eq!(stats.error_count, 1);
        assert!((stats.total_cost_usd - 0.02).abs() < 1e-9);
    }
}