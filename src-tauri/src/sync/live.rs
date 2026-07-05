//! Live broadcast hub.
//!
//! The plan: every source hands the engine a `mpsc::Receiver<CanonicalEvent>`
//! from [`Source::live_events`]. We spawn one task per source that pumps
//! those events into a single `tokio::sync::broadcast::Sender`, which the
//! renderer (and future log/error taps) subscribes to.
//!
//! Why broadcast and not mpsc? Two reasons:
//!  1. The renderer can drop and re-subscribe without us having to
//!     re-open the upstream tail.
//!  2. Phase 2 will add a second subscriber (audit log) and we don't
//!     want to refactor at that point.
//!
//! If the broadcast buffer fills up (slow subscriber), we drop the
//! lagging receiver (the default `SendError` behaviour) and keep going.
//! Live events are advisory — they're not the source of truth, the
//! store is. So a dropped subscriber just means a slightly stale UI
//! until they reconnect.

use crate::sources::{CanonicalEvent, Source, SourceError};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Capacity of the broadcast bus. 1024 events ≈ a few minutes of busy
/// Claude work; bigger buffer = more memory + longer catch-up time
/// when a subscriber reconnects.
const BROADCAST_CAPACITY: usize = 1024;

/// Wrap `broadcast::Sender<CanonicalEvent>` with a backpressure-safe
/// `subscribe()` and a `publish()` that swallows the inevitable
/// `RecvError::Lagged` for callers who don't want to deal with it.
pub struct LiveHub {
    tx: broadcast::Sender<CanonicalEvent>,
}

impl LiveHub {
    /// Build an empty hub. Call `pump_source` for each source you want
    /// to subscribe to.
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        Self { tx }
    }

    /// Subscribe to the live event stream. Each call returns an
    /// independent receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<CanonicalEvent> {
        self.tx.subscribe()
    }

    /// Number of currently subscribed receivers (mostly for diagnostics).
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Publish an event to all subscribers. Returns the number of
    /// receivers that got the event. A return of `0` just means
    /// nobody is listening yet — that's fine, the event was still
    /// persisted by the caller (sync engine persists first, broadcasts
    /// second).
    pub fn publish(&self, event: CanonicalEvent) -> usize {
        // `send` only errors when there are zero subscribers; we treat
        // that as success because the store has the truth.
        match self.tx.send(event) {
            Ok(n) => n,
            Err(_) => 0,
        }
    }

    /// Spawn a task that pumps every event from `source`'s
    /// `live_events()` channel into the broadcast hub. Each source
    /// gets its own task so a slow source can't block another.
    pub fn pump_source(self: &Arc<Self>, source: Arc<dyn Source>) {
        let hub = Arc::clone(self);
        tokio::spawn(async move {
            let mut rx = match source.live_events().await {
                Ok(rx) => rx,
                Err(e) => {
                    warn!(source = source.id(), error = %e, "live_events() failed; source offline");
                    return;
                }
            };
            debug!(source = source.id(), "live pump started");
            while let Some(event) = rx.recv().await {
                let delivered = hub.publish(event);
                if delivered == 0 {
                    debug!(source = source.id(), "no live subscribers; event stored only");
                }
            }
            debug!(source = source.id(), "live pump exited (source channel closed)");
        });
    }
}

impl Default for LiveHub {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: convert a `SourceError` into a runtime error so the
/// future returned by `pump_source` can `?` on it. Currently unused
/// (we just log) but kept here so future refactors can flip to
/// returning a `Result` without API churn.
#[allow(dead_code)]
pub fn source_err_string(e: SourceError) -> String {
    e.to_string()
}

/// Re-export so callers don't have to chase down the mpsc import.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::EventKind;

    fn ev(seq: i64) -> CanonicalEvent {
        CanonicalEvent {
            session_id: "ses_test".into(),
            seq,
            occurred_at: 1.0 + seq as f64,
            kind: EventKind::AssistantText,
            payload: serde_json::json!({"i": seq}),
            duration_ms: None,
            tool_name: None,
            tool_input_size: None,
            tool_result_size: None,
            cost_usd: 0.0,
            tokens_in: 0,
            tokens_out: 0,
            model: None,
        }
    }

    #[tokio::test]
    async fn publish_delivers_to_subscriber() {
        let hub = LiveHub::new();
        let mut rx = hub.subscribe();
        hub.publish(ev(1));
        let got = rx.recv().await.unwrap();
        assert_eq!(got.seq, 1);
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive() {
        let hub = LiveHub::new();
        let mut a = hub.subscribe();
        let mut b = hub.subscribe();
        hub.publish(ev(7));
        assert_eq!(a.recv().await.unwrap().seq, 7);
        assert_eq!(b.recv().await.unwrap().seq, 7);
    }

    #[tokio::test]
    async fn publish_with_no_subscribers_is_ok() {
        let hub = LiveHub::new();
        // No panic, no error: this is the "store is truth" path.
        assert_eq!(hub.publish(ev(1)), 0);
    }
}