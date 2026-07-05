//! Historical backfill (Task 5.2).
//!
//! On startup the engine calls [`backfill_source`] for every registered
//! source. That, in turn:
//!
//!  1. Calls `source.backfill(None)` to ask for the full history.
//!  2. For each `(CanonicalSession, Vec<CanonicalEvent>)` pair:
//!     - Upsert the session (idempotent).
//!     - Skip events whose `seq` is `<= last_seq_for(source, native_id)`
//!       so we can resume across restarts.
//!     - Batch the remaining events into groups of [`BATCH_SIZE`] and
//!       upsert them inside a single SQLite transaction. Inside a txn
//!       the per-row overhead collapses and we get atomicity for free.
//!  3. Persist the highest `seq` seen back to `sync_meta` so the next
//!     boot can resume.
//!
//! Secret redaction (Task 3) is applied here, before the event hits
//! SQLite. We do it once on ingest — once it's in the store the
//! payload is sanitised forever.

use crate::error::{AppError, AppResult};
use crate::sources::{CanonicalEvent, CanonicalSession, Source};
use crate::store::redact;
use crate::store::Store;
use std::sync::Arc;
use tracing::{info, warn};

/// Per-transaction event batch size. 100 is the plan's stated number;
/// it's small enough to keep individual txns quick but large enough to
/// amortise commit overhead.
const BATCH_SIZE: usize = 100;

/// Outcome stats for a single source's backfill pass. The engine
/// surfaces these in the boot log so we can see at a glance whether
/// anything is stuck.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct BackfillReport {
    pub source: String,
    pub sessions: usize,
    pub events_inserted: usize,
    pub events_skipped: usize,
    pub cursor_advanced: bool,
}

/// Run a single source's backfill pass end-to-end. Errors from the
/// source (network, parse) are logged and returned as `Err` so the
/// caller can decide whether to abort the boot or press on.
pub async fn backfill_source(
    source: Arc<dyn Source>,
    store: Arc<Store>,
) -> AppResult<BackfillReport> {
    let name = source.id();
    info!(source = name, "backfill starting");
    let t0 = std::time::Instant::now();

    let data = source.backfill(None).await.map_err(|e| {
        warn!(source = name, error = %e, "backfill() failed");
        AppError::Upstream(format!("{name} backfill: {e}"))
    })?;

    let mut report = BackfillReport {
        source: name.to_string(),
        ..Default::default()
    };

    for (session, events) in data {
        report.sessions += 1;
        // Upsert the session row first so the FK on `event` is satisfied.
        store.upsert_session(&session)?;

        // Pull the resume cursor once per session; saves a SQLite
        // round-trip per event.
        let last_seq = store.last_seq_for(name, &session.native_id)?.unwrap_or(0);

        let mut max_seq_seen = last_seq;
        let mut to_insert: Vec<CanonicalEvent> = Vec::new();

        for ev in events {
            if ev.seq <= last_seq {
                report.events_skipped += 1;
                continue;
            }
            if ev.seq > max_seq_seen {
                max_seq_seen = ev.seq;
            }
            to_insert.push(redact_event(ev));
        }

        // Sort by seq ascending so batch writes are monotonically
        // increasing and the UNIQUE(session_id, seq) index hits a
        // warm path.
        to_insert.sort_by_key(|e| e.seq);

        // Batch upsert in chunks of BATCH_SIZE, each its own transaction.
        let inserted = insert_in_batches(&store, &to_insert, BATCH_SIZE)?;
        report.events_inserted += inserted;

        // Persist the cursor only after the writes committed.
        if max_seq_seen > last_seq {
            store.set_last_seq(name, &session.native_id, max_seq_seen)?;
            report.cursor_advanced = true;
        }
    }

    info!(
        source = name,
        sessions = report.sessions,
        inserted = report.events_inserted,
        skipped = report.events_skipped,
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "backfill done"
    );
    Ok(report)
}

/// Insert a slice of events in batches of `batch_size`. Each batch is
/// wrapped in `BEGIN .. COMMIT` so we get atomic writes (and the per-row
/// commit overhead amortises). Returns the number of rows that
/// actually landed (deduping is silently absorbed by `upsert_event`).
fn insert_in_batches(
    store: &Store,
    events: &[CanonicalEvent],
    batch_size: usize,
) -> AppResult<usize> {
    if events.is_empty() {
        return Ok(0);
    }
    store.insert_batch(events, batch_size)
}

/// Redact the payload of a single event (in place JSON).
fn redact_event(mut ev: CanonicalEvent) -> CanonicalEvent {
    ev.payload = redact::redact_value(&ev.payload);
    ev
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{EventKind, Source, SourceError};
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    /// Mock source that yields one session with N events.
    struct FixedSource {
        sid: &'static str,
        session: CanonicalSession,
        events: Vec<CanonicalEvent>,
    }

    #[async_trait]
    impl Source for FixedSource {
        fn id(&self) -> &'static str { self.sid }
        async fn health(&self) -> Result<(), SourceError> { Ok(()) }
        async fn backfill(&self, _since: Option<f64>)
            -> Result<Vec<(CanonicalSession, Vec<CanonicalEvent>)>, SourceError>
        {
            Ok(vec![(self.session.clone(), self.events.clone())])
        }
        async fn live_events(&self) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
            let (_tx, rx) = mpsc::channel(1);
            Ok(rx)
        }
    }

    fn fixture_session(sid: &str, native: &str, started: f64) -> CanonicalSession {
        CanonicalSession {
            id: sid.into(),
            source_id: "claude".into(), // seeded in agent_source by 0002
            native_id: native.into(),
            title: Some("mock session".into()),
            project_dir: None, cwd: None, git_branch: None,
            model: Some("mock-model".into()),
            started_at: started,
            ended_at: None, end_reason: None,
            input_tokens: 0, output_tokens: 0,
            cache_read: 0, cache_write: 0, reasoning_tokens: 0,
            cost_usd: 0.0, message_count: 0, tool_call_count: 0,
            parent_session_id: None, source_path: String::new(),
        }
    }

    fn fixture_event(sid: &str, seq: i64) -> CanonicalEvent {
        CanonicalEvent {
            session_id: sid.into(),
            seq,
            occurred_at: 1.0 + seq as f64,
            kind: EventKind::AssistantText,
            payload: serde_json::json!({"seq": seq}),
            duration_ms: None, tool_name: None,
            tool_input_size: None, tool_result_size: None,
            cost_usd: 0.0, tokens_in: 0, tokens_out: 0, model: None,
        }
    }

    #[tokio::test]
    async fn backfill_inserts_events_and_writes_cursor() {
        let store = Arc::new(Store::open_memory().unwrap());
        let session = fixture_session("ses_1", "native_1", 100.0);
        let events: Vec<CanonicalEvent> =
            (1..=5).map(|s| fixture_event("ses_1", s)).collect();
        let src: Arc<dyn Source> = Arc::new(FixedSource {
            sid: "mock", session, events,
        });

        let report = backfill_source(src, Arc::clone(&store)).await.unwrap();
        assert_eq!(report.sessions, 1);
        assert_eq!(report.events_inserted, 5);
        assert_eq!(report.events_skipped, 0);
        assert!(report.cursor_advanced);

        // Verify they're in the store.
        let stored = store.list_events("ses_1", 100).unwrap();
        assert_eq!(stored.len(), 5);

        // Verify the cursor.
        let cursor = store.last_seq_for("mock", "native_1").unwrap();
        assert_eq!(cursor, Some(5));
    }

    #[tokio::test]
    async fn backfill_resumes_from_cursor() {
        let store = Arc::new(Store::open_memory().unwrap());
        // First pass: ingest 5 events.
        let session = fixture_session("ses_2", "native_2", 100.0);
        let events: Vec<CanonicalEvent> =
            (1..=5).map(|s| fixture_event("ses_2", s)).collect();
        let src1: Arc<dyn Source> = Arc::new(FixedSource {
            sid: "mock", session: session.clone(), events,
        });
        backfill_source(src1, Arc::clone(&store)).await.unwrap();

        // Second pass: re-emit the same events + 3 new ones.
        let mut events2: Vec<CanonicalEvent> =
            (1..=5).map(|s| fixture_event("ses_2", s)).collect();
        events2.extend((6..=8).map(|s| fixture_event("ses_2", s)));
        let src2: Arc<dyn Source> = Arc::new(FixedSource {
            sid: "mock", session, events: events2,
        });
        let report = backfill_source(src2, Arc::clone(&store)).await.unwrap();
        assert_eq!(report.events_inserted, 3);
        assert_eq!(report.events_skipped, 5);
        assert_eq!(report.cursor_advanced, true);

        let stored = store.list_events("ses_2", 100).unwrap();
        assert_eq!(stored.len(), 8);
        let cursor = store.last_seq_for("mock", "native_2").unwrap();
        assert_eq!(cursor, Some(8));
    }

    #[tokio::test]
    async fn backfill_redacts_payload() {
        let store = Arc::new(Store::open_memory().unwrap());
        let mut ev = fixture_event("ses_3", 1);
        ev.payload = serde_json::json!({"token": "sk-abcdefghijklmnopqrstuvwxyz1234567890"});
        let session = fixture_session("ses_3", "native_3", 100.0);
        let src: Arc<dyn Source> = Arc::new(FixedSource {
            sid: "mock", session, events: vec![ev],
        });
        backfill_source(src, Arc::clone(&store)).await.unwrap();
        let stored = store.list_events("ses_3", 10).unwrap();
        assert_eq!(stored.len(), 1);
        let s = stored[0].payload.to_string();
        assert!(s.contains("[REDACTED]"), "expected redaction in {s}");
    }

    #[tokio::test]
    async fn empty_backfill_is_a_no_op() {
        let store = Arc::new(Store::open_memory().unwrap());
        let src: Arc<dyn Source> = Arc::new(FixedSource {
            sid: "mock",
            session: fixture_session("ses_x", "native_x", 0.0),
            events: vec![],
        });
        let report = backfill_source(src, Arc::clone(&store)).await.unwrap();
        assert_eq!(report.events_inserted, 0);
        assert_eq!(report.events_skipped, 0);
        assert!(!report.cursor_advanced);
    }

    #[tokio::test]
    async fn batch_size_boundary() {
        // 250 events should yield 3 batches: 100, 100, 50.
        let store = Arc::new(Store::open_memory().unwrap());
        let session = fixture_session("ses_b", "native_b", 100.0);
        let events: Vec<CanonicalEvent> =
            (1..=250).map(|s| fixture_event("ses_b", s)).collect();
        let src: Arc<dyn Source> = Arc::new(FixedSource {
            sid: "mock", session, events,
        });
        let report = backfill_source(src, Arc::clone(&store)).await.unwrap();
        assert_eq!(report.events_inserted, 250);
        let stored = store.list_events("ses_b", 1000).unwrap();
        assert_eq!(stored.len(), 250);
    }
}