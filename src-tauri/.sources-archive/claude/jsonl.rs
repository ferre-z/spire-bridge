//! JSONL transcript tailer for Claude Code.
//!
//! Claude Code writes one event per line to
//! `~/.claude/projects/<project-slug>/<session-id>.jsonl`. We watch that
//! directory with [`notify`], and for each newly-appended line we parse
//! and route through the normaliser.
//!
//! Phase 1 supports **read+tail** mode: `backfill` reads existing files
//! in full, then `live_events` keeps watching for new lines on the same
//! files. The watch is per-process — Phase 2 will move it to a long-lived
//! task owned by the sync engine.

use crate::sources::claude::normalize::claude_event_from_jsonl;
use crate::sources::{CanonicalEvent, CanonicalSession, SourceError};
use notify::{Event, EventKind as NotifyEventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Where Claude Code stores JSONL transcripts.
pub fn default_claude_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".claude").join("projects");
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(profile)
            .join(".claude")
            .join("projects");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Result of one round of [`backfill`] — the absolute paths the watcher
/// should subscribe to, paired with the parsed (session, events) tuple.
#[derive(Debug, Clone)]
pub struct Backfilled {
    pub file: PathBuf,
    pub native_session_id: String,
    pub session: CanonicalSession,
    pub events: Vec<CanonicalEvent>,
}

/// Walk every `*.jsonl` under the Claude projects dir and return a
/// (session metadata, events) pair per file.
pub fn backfill_dir(dir: &Path) -> Result<Vec<Backfilled>, SourceError> {
    let mut out = Vec::new();

    let projects = std::fs::read_dir(dir).map_err(SourceError::Io)?;
    for project in projects.flatten() {
        let project_path = project.path();
        if !project_path.is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(&project_path).map_err(SourceError::Io)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if let Some(bf) = read_file(&path)? {
                out.push(bf);
            }
        }
    }

    Ok(out)
}

/// Read a single JSONL file end-to-end and return its canonical session +
/// events.
pub fn read_file(path: &Path) -> Result<Option<Backfilled>, SourceError> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(SourceError::Io(e)),
    };

    // Session id is the file stem (matches Claude's convention).
    let native = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut events: Vec<CanonicalEvent> = Vec::new();
    let mut session: Option<CanonicalSession> = None;
    let mut seq: i64 = 0;

    let reader = BufReader::new(file);
    for (idx, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue, // skip malformed lines — partial writes happen
        };

        if session.is_none() {
            session = Some(crate::sources::claude::normalize::claude_session_from_jsonl(
                &native,
                path.to_string_lossy().as_ref(),
                &value,
            ));
        }

        seq += 1;
        // Session id is filled by the store on insert; we use a placeholder
        // here so downstream code can address the row.
        let placeholder_id = format!("claude:{native}");
        events.push(claude_event_from_jsonl(&placeholder_id, seq, &value));
        let _ = idx;
    }

    let session = match session {
        Some(s) => s,
        None => return Ok(None), // empty file
    };

    Ok(Some(Backfilled {
        file: path.to_path_buf(),
        native_session_id: native,
        session,
        events,
    }))
}

/// A simple live tailer: every `tick` it re-reads any file whose size has
/// grown since the last poll and pushes new events into `tx`. Returns
/// when `tx` is closed (consumer dropped).
///
/// Phase 1 uses polling because the JSONL files are append-only and the
/// `notify` crate's cross-platform debouncing for plain files is fragile.
/// We can swap in `notify` later for sub-second latency.
pub async fn tail_files(
    files: Vec<(PathBuf, String)>, // (path, native_session_id)
    placeholder_id_for: impl Fn(&str) -> String + Send + 'static,
    mut tx: mpsc::Sender<CanonicalEvent>,
    tick: Duration,
) -> Result<(), SourceError> {
    // Track last byte offset per file.
    let mut offsets: HashMap<PathBuf, u64> = HashMap::new();
    for (path, _) in &files {
        if let Ok(meta) = std::fs::metadata(path) {
            offsets.insert(path.clone(), meta.len());
        }
    }

    loop {
        tokio::time::sleep(tick).await;

        let mut progressed = false;
        for (path, native) in &files {
            let prev = offsets.get(path).copied().unwrap_or(0);
            let current = match std::fs::metadata(path) {
                Ok(m) => m.len(),
                Err(_) => continue,
            };
            if current <= prev {
                continue;
            }

            let new_lines = match read_range(path, prev, current) {
                Ok(n) => n,
                Err(_) => continue,
            };
            offsets.insert(path.clone(), current);
            progressed = true;

            let placeholder = placeholder_id_for(native);
            let mut seq: i64 = 0;
            for line in new_lines {
                if line.trim().is_empty() {
                    continue;
                }
                let value: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                seq += 1;
                let ev = claude_event_from_jsonl(&placeholder, seq, &value);
                if tx.send(ev).await.is_err() {
                    return Ok(()); // consumer gone
                }
            }
        }

        if !progressed && tx.is_closed() {
            return Ok(());
        }
    }
}

fn read_range(path: &Path, start: u64, end: u64) -> std::io::Result<Vec<String>> {
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::with_capacity((end - start).min(64 * 1024) as usize);
    use std::io::Read;
    f.take(end - start).read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    Ok(text.lines().map(str::to_owned).collect())
}

/// Build a `notify` watcher for the projects dir, forwarding any
/// `Modify(_)` events through the returned std-mpsc receiver.
///
/// Used by the sync engine (Task 5) when it wants push-based updates.
pub fn watcher(dir: &Path) -> notify::Result<(RecommendedWatcher, std_mpsc::Receiver<PathBuf>)> {
    let (tx, rx) = std_mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(ev) = res {
            if matches!(
                ev.kind,
                NotifyEventKind::Modify(_) | NotifyEventKind::Create(_)
            ) {
                for p in ev.paths {
                    let _ = tx.send(p);
                }
            }
        }
    })?;
    watcher.watch(dir, RecursiveMode::Recursive)?;
    Ok((watcher, rx))
}