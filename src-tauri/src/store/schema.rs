//! Query helpers for sessions and events.
//!
//! These are the data-shape primitives that the IPC layer will eventually
//! wrap with `tauri::command` handlers (Task 6). For Task 3 the goal is
//! to keep them self-contained — `Session` and `Event` rows here mirror
//! the SQL columns 1:1. Task 4 (`sources/mod.rs`) introduces the public
//! `CanonicalSession`/`CanonicalEvent` types; once that lands these row
//! structs get a thin `From` conversion layer.
//!
//! Prepared statements are cached in `OnceCell` per `Connection` via the
//! `CACHED` helper. The store model is `Arc<Mutex<Connection>>` so we
//! serialise access at the `lock()` boundary, which makes per-conn
//! statement caching safe and simple.

use crate::error::{AppError, AppResult};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

/// Filter shape passed to [`list_sessions`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SessionFilter {
    pub source: Option<String>,
    pub since: Option<f64>,
    pub until: Option<f64>,
    pub search: Option<String>,
}

/// Aggregate stats for the dashboard "since N days ago" view.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_cost_usd: f64,
    pub session_count: i64,
    pub error_count: i64,
    pub top_tools: Vec<(String, i64)>,
    pub hourly_buckets: Vec<(i64, f64)>, // (hour_epoch, cost_usd)
}

/// One session row, mirroring the `session` table columns.
///
/// `raw_json` is intentionally a `String` here rather than
/// `serde_json::Value` — the source path will sometimes stuff non-JSON
/// "raw text" into this column (e.g. tails of Claude OTel payloads),
/// and forcing a parse on read is wasted work for listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub source_id: String,
    pub native_id: String,
    pub title: Option<String>,
    pub project_dir: Option<String>,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    pub started_at: f64,
    pub ended_at: Option<f64>,
    pub end_reason: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub reasoning_tokens: i64,
    pub cost_usd: f64,
    pub message_count: i64,
    pub tool_call_count: i64,
    pub parent_session_id: Option<String>,
    pub raw_json: Option<String>,
    pub source_path: String,
    pub updated_at: f64,
}

impl Session {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            source_id: row.get("source_id")?,
            native_id: row.get("native_id")?,
            title: row.get("title")?,
            project_dir: row.get("project_dir")?,
            cwd: row.get("cwd")?,
            git_branch: row.get("git_branch")?,
            model: row.get("model")?,
            started_at: row.get("started_at")?,
            ended_at: row.get("ended_at")?,
            end_reason: row.get("end_reason")?,
            input_tokens: row.get("input_tokens")?,
            output_tokens: row.get("output_tokens")?,
            cache_read: row.get("cache_read")?,
            cache_write: row.get("cache_write")?,
            reasoning_tokens: row.get("reasoning_tokens")?,
            cost_usd: row.get("cost_usd")?,
            message_count: row.get("message_count")?,
            tool_call_count: row.get("tool_call_count")?,
            parent_session_id: row.get("parent_session_id")?,
            raw_json: row.get("raw_json")?,
            source_path: row.get("source_path")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

/// One event row, mirroring the `event` table columns. Payload is the
/// redacted JSON text already (callers must redact before writing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<i64>,
    pub session_id: String,
    pub seq: i64,
    pub occurred_at: f64,
    pub kind: String,
    pub payload: String, // already redacted JSON text
    pub duration_ms: Option<i64>,
    pub tool_name: Option<String>,
    pub tool_input_size: Option<i64>,
    pub tool_result_size: Option<i64>,
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub model: Option<String>,
}

impl Event {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            session_id: row.get("session_id")?,
            seq: row.get("seq")?,
            occurred_at: row.get("occurred_at")?,
            kind: row.get("kind")?,
            payload: row.get("payload")?,
            duration_ms: row.get("duration_ms")?,
            tool_name: row.get("tool_name")?,
            tool_input_size: row.get("tool_input_size")?,
            tool_result_size: row.get("tool_result_size")?,
            cost_usd: row.get("cost_usd")?,
            tokens_in: row.get("tokens_in")?,
            tokens_out: row.get("tokens_out")?,
            model: row.get("model")?,
        })
    }
}

/// Upsert a session by `(source_id, native_id)`. All mutating fields
/// are updated; counters are explicitly overridden by the caller (the
/// sync engine has the latest delta).
pub fn upsert_session(conn: &Connection, s: &Session) -> AppResult<()> {
    conn.execute(
        r#"
        INSERT INTO session (
            id, source_id, native_id, title, project_dir, cwd, git_branch, model,
            started_at, ended_at, end_reason,
            input_tokens, output_tokens, cache_read, cache_write, reasoning_tokens,
            cost_usd, message_count, tool_call_count,
            parent_session_id, raw_json, source_path, updated_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
            ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16,
            ?17, ?18, ?19,
            ?20, ?21, ?22, ?23
        )
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
            raw_json        = excluded.raw_json,
            source_path     = excluded.source_path,
            updated_at      = unixepoch()
        "#,
        params![
            s.id, s.source_id, s.native_id, s.title, s.project_dir, s.cwd, s.git_branch, s.model,
            s.started_at, s.ended_at, s.end_reason,
            s.input_tokens, s.output_tokens, s.cache_read, s.cache_write, s.reasoning_tokens,
            s.cost_usd, s.message_count, s.tool_call_count,
            s.parent_session_id, s.raw_json, s.source_path,
        ],
    )?;
    Ok(())
}

/// Upsert an event by `(session_id, seq)`. The caller is expected to
/// have already run [`crate::store::redact::redact`] on `payload`.
pub fn upsert_event(conn: &Connection, e: &Event) -> AppResult<()> {
    conn.execute(
        r#"
        INSERT INTO event (
            session_id, seq, occurred_at, kind, payload,
            duration_ms, tool_name, tool_input_size, tool_result_size,
            cost_usd, tokens_in, tokens_out, model
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(session_id, seq) DO UPDATE SET
            occurred_at      = excluded.occurred_at,
            kind             = excluded.kind,
            payload          = excluded.payload,
            duration_ms      = excluded.duration_ms,
            tool_name        = excluded.tool_name,
            tool_input_size  = excluded.tool_input_size,
            tool_result_size = excluded.tool_result_size,
            cost_usd         = excluded.cost_usd,
            tokens_in        = excluded.tokens_in,
            tokens_out       = excluded.tokens_out,
            model            = excluded.model
        "#,
        params![
            e.session_id, e.seq, e.occurred_at, e.kind, e.payload,
            e.duration_ms, e.tool_name, e.tool_input_size, e.tool_result_size,
            e.cost_usd, e.tokens_in, e.tokens_out, e.model,
        ],
    )?;
    Ok(())
}

/// List sessions, newest-first, with optional filtering.
pub fn list_sessions(
    conn: &Connection,
    filter: &SessionFilter,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Session>> {
    let mut sql = String::from(
        "SELECT id, source_id, native_id, title, project_dir, cwd, git_branch, model,
                started_at, ended_at, end_reason,
                input_tokens, output_tokens, cache_read, cache_write, reasoning_tokens,
                cost_usd, message_count, tool_call_count,
                parent_session_id, raw_json, source_path, updated_at
           FROM session
          WHERE 1 = 1",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(src) = &filter.source {
        sql.push_str(" AND source_id = ?");
        args.push(Box::new(src.clone()));
    }
    if let Some(since) = filter.since {
        sql.push_str(" AND started_at >= ?");
        args.push(Box::new(since));
    }
    if let Some(until) = filter.until {
        sql.push_str(" AND started_at <= ?");
        args.push(Box::new(until));
    }
    if let Some(q) = &filter.search {
        sql.push_str(" AND (title LIKE ? OR project_dir LIKE ? OR cwd LIKE ?)");
        let needle = format!("%{q}%");
        args.push(Box::new(needle.clone()));
        args.push(Box::new(needle.clone()));
        args.push(Box::new(needle));
    }

    sql.push_str(" ORDER BY started_at DESC LIMIT ? OFFSET ?");
    args.push(Box::new(limit));
    args.push(Box::new(offset));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(args.iter()), Session::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Look up a single session by primary key. Returns `AppError::NotFound`
/// when the row is missing so callers don't have to unwrap `Option`.
pub fn get_session(conn: &Connection, id: &str) -> AppResult<Session> {
    let row = conn
        .query_row(
            "SELECT id, source_id, native_id, title, project_dir, cwd, git_branch, model,
                    started_at, ended_at, end_reason,
                    input_tokens, output_tokens, cache_read, cache_write, reasoning_tokens,
                    cost_usd, message_count, tool_call_count,
                    parent_session_id, raw_json, source_path, updated_at
               FROM session
              WHERE id = ?1",
            params![id],
            Session::from_row,
        )
        .optional()?;
    row.ok_or_else(|| AppError::NotFound(format!("session {id}")))
}

/// List events for a session, ordered by `seq` ascending.
pub fn list_events(
    conn: &Connection,
    session_id: &str,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, seq, occurred_at, kind, payload,
                duration_ms, tool_name, tool_input_size, tool_result_size,
                cost_usd, tokens_in, tokens_out, model
           FROM event
          WHERE session_id = ?1
          ORDER BY seq ASC
          LIMIT ?2 OFFSET ?3",
    )?;
    let rows = stmt
        .query_map(params![session_id, limit, offset], Event::from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Dashboard roll-up. `since` is a unixepoch (seconds). Implementations
/// are deliberately SQL-side so we avoid pulling 10k rows into memory.
pub fn dashboard_stats(conn: &Connection, since: f64) -> AppResult<DashboardStats> {
    let total_cost_usd: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM session WHERE started_at >= ?1",
            params![since],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let session_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM session WHERE started_at >= ?1",
            params![since],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let error_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event
              WHERE occurred_at >= ?1 AND kind IN ('ApiError', 'ApiRefusal')",
            params![since],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut top_tools_stmt = conn.prepare(
        "SELECT tool_name, COUNT(*) AS c
           FROM event
          WHERE occurred_at >= ?1 AND tool_name IS NOT NULL
          GROUP BY tool_name
          ORDER BY c DESC
          LIMIT 5",
    )?;
    let top_tools: Vec<(String, i64)> = top_tools_stmt
        .query_map(params![since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // 24 hourly buckets covering the trailing 24h from `since`. Each bucket
    // rounds `since + n*3600` down to a whole hour for grouping.
    let mut bucket_stmt = conn.prepare(
        "SELECT CAST((occurred_at - ?1) / 3600 AS INTEGER) AS bucket_hour,
                COALESCE(SUM(cost_usd), 0.0)
           FROM event
          WHERE occurred_at >= ?1 AND occurred_at < ?1 + 86400
          GROUP BY bucket_hour
          ORDER BY bucket_hour ASC",
    )?;
    let hourly_buckets: Vec<(i64, f64)> = bucket_stmt
        .query_map(params![since], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(DashboardStats {
        total_cost_usd,
        session_count,
        error_count,
        top_tools,
        hourly_buckets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_filter_default_is_empty() {
        let f = SessionFilter::default();
        assert!(f.source.is_none());
        assert!(f.since.is_none());
        assert!(f.search.is_none());
    }

    #[test]
    fn dashboard_stats_default_is_zeroed() {
        let s = DashboardStats::default();
        assert_eq!(s.total_cost_usd, 0.0);
        assert_eq!(s.session_count, 0);
        assert!(s.top_tools.is_empty());
        assert!(s.hourly_buckets.is_empty());
    }
}
