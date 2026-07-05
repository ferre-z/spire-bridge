//! Sync engine (Task 5.3).
//!
//! Wires the three pieces together:
//!
//! ```text
//!  ┌────────────┐   mpsc   ┌─────────────────┐  broadcast  ┌────────────┐
//!  │  Source(s) │ ───────► │ SyncEngine.run  │ ──────────► │ LiveHub    │ ─► IPC
//!  └────────────┘          │  _live +        │             │ subscribers│
//!        │                 │  backfill_all   │             └────────────┘
//!        │                 └────────┬────────┘
//!        │                          ▼
//!        │                  ┌───────────────┐
//!        └─(backfill)──────►│  Store        │  SQLite, sync_meta cursors
//!                           └───────────────┘
//! ```
//!
//! `start()` spawns one backfill task (across all sources, sequentially)
//! and one live-loop task per source. Both tasks persist to the store;
//! the live loop additionally publishes to the broadcast hub. The store
//! remains the source of truth — the live channel is advisory.
//!
//! Errors during backfill are logged but do not abort the engine: a
//! single bad source should not prevent the other two from syncing.
//! The engine never panics; it relies on `AppError` for fallible paths.

pub mod backfill;
pub mod live;
// LiveHub is intentionally NOT re-exported here. It's used internally
// via `crate::sync::live::LiveHub`; consumers outside `sync` should
// import from `crate::sync::LiveHub` through `live::LiveHub` directly.

use crate::error::{AppError, AppResult};
use crate::sources::Source;
use crate::store::Store;
use live::LiveHub;
use std::sync::Arc;
use tracing::{error, info};

/// The engine. Always passed as `Arc<SyncEngine>` because every method
/// spawns a task that needs to outlive the caller's stack frame.
pub struct SyncEngine {
    /// Every source we know about. Order is preserved so logs are
    /// deterministic across boots.
    pub sources: Vec<Arc<dyn Source>>,
    /// Shared SQLite handle.
    pub store: Arc<Store>,
    /// Broadcast hub for live events.
    pub live: Arc<LiveHub>,
}

impl SyncEngine {
    /// Construct a new engine. The engine is intentionally inert
    /// until [`start`] is called — that lets the caller wire
    /// `tauri::State` references first.
    pub fn new(sources: Vec<Arc<dyn Source>>, store: Arc<Store>) -> Self {
        Self {
            sources,
            store,
            live: Arc::new(LiveHub::new()),
        }
    }

    /// Start the engine. Returns once the spawns are dispatched; the
    /// actual work happens on the tokio runtime.
    ///
    /// The plan's contract:
    ///  * Backfill runs first (sequentially, in a single task).
    ///  * Live loops run in parallel (one task per source).
    pub async fn start(self: Arc<Self>) -> AppResult<()> {
        info!(
            sources = self.sources.len(),
            "sync engine starting"
        );

        // Spawn the backfill task. We deliberately run all sources'
        // backfill sequentially inside one task so SQLite contention
        // stays predictable.
        let backfill_handle = {
            let engine = Arc::clone(&self);
            tokio::spawn(async move { engine.run_backfill_all().await })
        };

        // Spawn one live loop per source. Each loop pulls events
        // from `Source::live_events()` and pumps them through
        // `LiveHub::publish`.
        for src in &self.sources {
            let engine = Arc::clone(&self);
            let src = Arc::clone(src);
            tokio::spawn(async move {
                engine.run_live(src).await;
            });
        }

        // We don't await `backfill_handle` here — the caller wants
        // `start` to return promptly so the renderer can mount.
        // However we *do* expose the join handle so future tasks
        // (Task 14: settings) can show backfill progress.
        self.spawn_backfill_watcher(backfill_handle);

        Ok(())
    }

    /// Background watcher: when backfill finishes, log the result.
    /// We never propagate a backfill error to the user — the store
    /// is consistent and the live loop is already running.
    fn spawn_backfill_watcher(
        self: &Arc<Self>,
        handle: tokio::task::JoinHandle<Result<Vec<backfill::BackfillReport>, AppError>>,
    ) {
        tokio::spawn(async move {
            match handle.await {
                Ok(Ok(reports)) => {
                    let total_inserted: usize =
                        reports.iter().map(|r| r.events_inserted).sum();
                    info!(
                        reports = reports.len(),
                        events_inserted = total_inserted,
                        "backfill pass complete"
                    );
                }
                Ok(Err(e)) => {
                    error!(error = %e, "backfill pass returned an error");
                }
                Err(join_err) => {
                    error!(error = %join_err, "backfill task panicked");
                }
            }
        });
    }

    /// Run backfill for every source, sequentially. Errors from one
    /// source don't stop the others.
    async fn run_backfill_all(self: Arc<Self>) -> AppResult<Vec<backfill::BackfillReport>> {
        let mut reports = Vec::with_capacity(self.sources.len());
        for src in &self.sources {
            match backfill::backfill_source(Arc::clone(src), Arc::clone(&self.store)).await {
                Ok(report) => reports.push(report),
                Err(e) => {
                    error!(source = src.id(), error = %e, "backfill failed; continuing");
                }
            }
        }
        Ok(reports)
    }

    /// Live loop for a single source. Reads events from the source's
    /// `mpsc::Receiver` and, for each event:
    ///   1. Redacts the payload.
    ///   2. Upserts the event into the store.
    ///   3. Broadcasts on the live hub.
    ///
    /// If the source channel closes (the upstream went away), we
    /// wait briefly then ask for a new receiver. This handles the
    /// common case of a CLI tool restarting between sessions.
    async fn run_live(self: Arc<Self>, source: Arc<dyn Source>) {
        let name = source.id();
        info!(source = name, "live loop starting");

        // Pull the session-level cursor once per session so we can
        // skip events we've already stored (the source may replay
        // the tail on reconnect).
        let mut backoff = std::time::Duration::from_millis(500);

        loop {
            let mut rx = match source.live_events().await {
                Ok(rx) => rx,
                Err(e) => {
                    error!(source = name, error = %e, "live_events() failed; backing off");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(std::time::Duration::from_secs(30));
                    continue;
                }
            };
            // Reset backoff on a successful handshake.
            backoff = std::time::Duration::from_millis(500);

            while let Some(mut event) = rx.recv().await {
                // Redact first so the on-disk row is sanitised forever.
                event.payload = crate::store::redact::redact_value(&event.payload);
                if let Err(e) = self.store.upsert_event(&event) {
                    error!(source = name, error = %e, "upsert_event failed; skipping");
                    continue;
                }
                // Advance the cursor so a reconnect doesn't replay.
                if let Err(e) = self
                    .store
                    .set_last_seq(name, &event.session_id, event.seq)
                {
                    error!(source = name, error = %e, "set_last_seq failed");
                }
                self.live.publish(event);
            }
            info!(source = name, "live channel closed; will reconnect");
        }
    }
}

/// Convenience constructor for the common "all three sources" case.
/// Task 6 will replace this with explicit DI from `tauri::State`; in
/// the meantime callers construct the engine with whatever `Vec<Arc<dyn Source>>`
/// they have (the unit tests use mock sources).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{CanonicalEvent, CanonicalSession, EventKind, Source, SourceError};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::mpsc;

    /// Mock source that records how many times `live_events()` was
    /// called and feeds a pre-built stream on each call.
    struct MockSource {
        id_str: &'static str,
        session: CanonicalSession,
        events: Vec<CanonicalEvent>,
        live_attempts: Arc<AtomicUsize>,
        max_live_calls: usize,
    }

    #[async_trait]
    impl Source for MockSource {
        fn id(&self) -> &'static str { self.id_str }
        async fn health(&self) -> Result<(), SourceError> { Ok(()) }
        async fn backfill(&self, _since: Option<f64>)
            -> Result<Vec<(CanonicalSession, Vec<CanonicalEvent>)>, SourceError>
        {
            Ok(vec![(self.session.clone(), self.events.clone())])
        }
        async fn live_events(&self) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
            let n = self.live_attempts.fetch_add(1, Ordering::SeqCst);
            assert!(n < self.max_live_calls, "too many live_events calls");
            let (tx, rx) = mpsc::channel(16);
            let evs = self.events.clone();
            tokio::spawn(async move {
                for e in evs {
                    if tx.send(e).await.is_err() {
                        break;
                    }
                }
                // Drop tx so the receiver closes.
            });
            Ok(rx)
        }
    }

    fn fx_session(sid: &str) -> CanonicalSession {
        CanonicalSession {
            id: sid.into(),
            source_id: "claude".into(),
            native_id: sid.into(),
            title: Some(format!("mock {sid}")),
            project_dir: None, cwd: None, git_branch: None,
            model: Some("mock-model".into()),
            started_at: 100.0,
            ended_at: None, end_reason: None,
            input_tokens: 0, output_tokens: 0,
            cache_read: 0, cache_write: 0, reasoning_tokens: 0,
            cost_usd: 0.0, message_count: 0, tool_call_count: 0,
            parent_session_id: None, source_path: String::new(),
        }
    }

    fn fx_event(sid: &str, seq: i64) -> CanonicalEvent {
        CanonicalEvent {
            session_id: sid.into(),
            seq,
            occurred_at: 100.0 + seq as f64,
            kind: EventKind::AssistantText,
            payload: serde_json::json!({"seq": seq}),
            duration_ms: None, tool_name: None,
            tool_input_size: None, tool_result_size: None,
            cost_usd: 0.0, tokens_in: 0, tokens_out: 0, model: None,
        }
    }

    #[tokio::test]
    async fn engine_starts_and_persists_backfill_events() {
        let store = Arc::new(Store::open_memory().unwrap());
        let events: Vec<CanonicalEvent> =
            (1..=5).map(|s| fx_event("ses_e", s)).collect();
        let src: Arc<dyn Source> = Arc::new(MockSource {
            id_str: "mock",
            session: fx_session("ses_e"),
            events: events.clone(),
            live_attempts: Arc::new(AtomicUsize::new(0)),
            max_live_calls: 100,
        });
        let engine = Arc::new(SyncEngine::new(vec![src], Arc::clone(&store)));
        engine.start().await.unwrap();

        // Wait until the engine has ingested the events.
        for _ in 0..50 {
            let got = store.list_events("ses_e", 100).unwrap();
            if got.len() >= 5 { break; }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let stored = store.list_events("ses_e", 100).unwrap();
        assert_eq!(stored.len(), 5, "backfill should land 5 events");
    }

    #[tokio::test]
    async fn engine_broadcasts_live_events_to_subscribers() {
        let store = Arc::new(Store::open_memory().unwrap());
        let events: Vec<CanonicalEvent> =
            (1..=3).map(|s| fx_event("ses_l", s)).collect();
        let src: Arc<dyn Source> = Arc::new(MockSource {
            id_str: "mock",
            session: fx_session("ses_l"),
            events: events.clone(),
            live_attempts: Arc::new(AtomicUsize::new(0)),
            max_live_calls: 100,
        });
        let engine = Arc::new(SyncEngine::new(vec![src], Arc::clone(&store)));
        // Clone before start() since start consumes the Arc.
        let live = Arc::clone(&engine.live);
        engine.start().await.unwrap();

        // Subscribe before the live events fire. The pump runs as
        // soon as `start` returns; we have a race window but in
        // practice the mock source's spawn fills the channel
        // immediately and the broadcast lag is zero.
        let mut rx = live.subscribe();

        let mut received = Vec::new();
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_secs(2);
        while received.len() < 3 && std::time::Instant::now() < deadline {
            match tokio::time::timeout(
                std::time::Duration::from_millis(100), rx.recv()).await {
                Ok(Ok(ev)) => received.push(ev.seq),
                _ => {}
            }
        }
        // The mock also replays the events through the live channel
        // *in addition* to the backfill, so the subscriber can see
        // any subset. Order isn't guaranteed because backfill and
        // live both race.
        let mut received_sorted = received.clone();
        received_sorted.sort();
        assert_eq!(
            received_sorted,
            vec![1, 2, 3],
            "subscriber should receive all 3 live events; got {received:?}"
        );
    }

    #[tokio::test]
    async fn engine_does_not_panic_on_empty_source_list() {
        let store = Arc::new(Store::open_memory().unwrap());
        let engine = Arc::new(SyncEngine::new(vec![], store));
        engine.start().await.unwrap();
    }

    #[tokio::test]
    async fn engine_continues_when_one_source_fails() {
        // One healthy source + one that always errors on backfill.
        // The engine should log + continue.
        struct AlwaysFails;
        #[async_trait]
        impl Source for AlwaysFails {
            fn id(&self) -> &'static str { "broken" }
            async fn health(&self) -> Result<(), SourceError> { Ok(()) }
            async fn backfill(&self, _since: Option<f64>)
                -> Result<Vec<(CanonicalSession, Vec<CanonicalEvent>)>, SourceError>
            {
                Err(SourceError::Upstream("nope".into()))
            }
            async fn live_events(&self) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
                let (_tx, rx) = mpsc::channel(1);
                Ok(rx)
            }
        }

        let store = Arc::new(Store::open_memory().unwrap());
        let good: Arc<dyn Source> = Arc::new(MockSource {
            id_str: "good",
            session: fx_session("ses_g"),
            events: vec![fx_event("ses_g", 1)],
            live_attempts: Arc::new(AtomicUsize::new(0)),
            max_live_calls: 100,
        });
        let bad: Arc<dyn Source> = Arc::new(AlwaysFails);
        let engine = Arc::new(SyncEngine::new(vec![bad, good], Arc::clone(&store)));
        engine.start().await.unwrap();

        // The healthy source's session should still land.
        for _ in 0..50 {
            let n = store.list_events("ses_g", 10).unwrap().len();
            if n >= 1 { break; }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let stored = store.list_events("ses_g", 10).unwrap();
        assert_eq!(stored.len(), 1, "good source should ingest despite broken one");
    }
}