//! Claude Code adapter.
//!
//! Exposes a single [`ClaudeSource`] that implements [`Source`]. Phase 1
//! is read-only and reads JSONL transcripts from
//! `~/.claude/projects/<project>/*.jsonl`. The OTel receiver
//! ([`otel`]) is a stub pending the Phase 2 protobuf work.

pub mod jsonl;
pub mod normalize;
pub mod otel;

use crate::sources::{CanonicalEvent, CanonicalSession, Source, SourceError};
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

/// Source adapter for Claude Code. Construct via [`ClaudeSource::new`]
/// or [`ClaudeSource::with_dir`] to override the projects directory.
pub struct ClaudeSource {
    /// Override for the projects dir (defaults to `~/.claude/projects`).
    dir: Option<PathBuf>,
    /// Polling interval for the live tail.
    poll_interval: Duration,
    /// Channel buffer for live events.
    channel_capacity: usize,
}

impl ClaudeSource {
    pub fn new() -> Self {
        Self {
            dir: None,
            poll_interval: Duration::from_millis(750),
            channel_capacity: 1024,
        }
    }

    /// Override the directory to scan for JSONL transcripts. Useful for
    /// tests.
    pub fn with_dir(dir: PathBuf) -> Self {
        Self {
            dir: Some(dir),
            ..Self::new()
        }
    }

    fn projects_dir(&self) -> Option<PathBuf> {
        self.dir.clone().or_else(jsonl::default_claude_dir)
    }
}

impl Default for ClaudeSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Source for ClaudeSource {
    fn id(&self) -> &'static str {
        "claude"
    }

    async fn health(&self) -> Result<(), SourceError> {
        match self.projects_dir() {
            Some(p) if p.is_dir() => Ok(()),
            Some(_) => Err(SourceError::NotRunning),
            None => Err(SourceError::NotRunning),
        }
    }

    async fn backfill(
        &self,
        since: Option<f64>,
    ) -> Result<Vec<(CanonicalSession, Vec<CanonicalEvent>)>, SourceError> {
        let dir = self
            .projects_dir()
            .ok_or(SourceError::NotRunning)?;
        let entries = jsonl::backfill_dir(&dir)?;

        let mut out = Vec::new();
        for bf in entries {
            if let Some(since) = since {
                if let Some(last) = bf.events.last() {
                    if last.occurred_at < since {
                        continue;
                    }
                }
            }
            out.push((bf.session, bf.events));
        }
        Ok(out)
    }

    async fn live_events(&self) -> Result<mpsc::Receiver<CanonicalEvent>, SourceError> {
        let dir = self
            .projects_dir()
            .ok_or(SourceError::NotRunning)?;

        let (tx, rx) = mpsc::channel(self.channel_capacity);

        // Collect every JSONL file currently in the directory; the tailer
        // will follow them.
        let entries = jsonl::backfill_dir(&dir)?;
        let files: Vec<(PathBuf, String)> = entries
            .into_iter()
            .map(|bf| (bf.file, bf.native_session_id))
            .collect();

        let placeholder_id_for = |native: &str| format!("claude:{native}");
        let tick = self.poll_interval;

        tokio::spawn(async move {
            let _ = jsonl::tail_files(files, placeholder_id_for, tx, tick).await;
        });

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_claude() {
        assert_eq!(ClaudeSource::new().id(), "claude");
    }

    #[tokio::test]
    async fn health_reports_missing_when_dir_absent() {
        let s = ClaudeSource::with_dir(PathBuf::from("/nonexistent/__definitely_not_here__"));
        assert!(s.health().await.is_err());
    }
}