//! Minimal IPC commands for Phase 1.
//!
//! Each command is a thin wrapper that takes a `tauri::State<AppState>`
//! and delegates to the store. Heavy work runs in `tokio::task::spawn_blocking`
//! so the Tauri runtime thread isn't blocked on SQLite I/O.

use super::{AppState, DashboardStatsLite, SessionDetail, SessionFilter};
use crate::error::AppResult;
use crate::sources::{CanonicalEvent, CanonicalSession, EventKind};
use std::sync::Arc;

/// `list_sessions(filter, limit, offset)` → `Vec<CanonicalSession>`.
#[tauri::command]
pub async fn list_sessions(
    state: tauri::State<'_, AppState>,
    filter: SessionFilter,
    limit: u32,
    offset: u32,
) -> AppResult<Vec<CanonicalSession>> {
    let store = Arc::clone(&state.store);
    tokio::task::spawn_blocking(move || {
        // `filter` is a coarse boolean today: if `filter.source` is set,
        // additionally filter by `source_id == filter.source`. The store
        // grows richer filtering once we have the GUI to validate against.
        let mut all = store.list_sessions(10000, 0)?;
        if let Some(source) = filter.source.as_deref() {
            all.retain(|s| s.source_id == source);
        }
        let start = (offset as usize).min(all.len());
        let end = (start + limit as usize).min(all.len());
        Ok(all[start..end].to_vec())
    })
    .await
    .map_err(|e| crate::error::AppError::Other(format!("join error: {e}")))?
}

/// `get_session(id)` → `SessionDetail { session, events }`.
#[tauri::command]
pub async fn get_session(
    state: tauri::State<'_, AppState>,
    id: String,
) -> AppResult<SessionDetail> {
    let store = Arc::clone(&state.store);
    let sid = id.clone();
    tokio::task::spawn_blocking(move || {
        let session = store
            .get_session(&sid)?
            .ok_or_else(|| crate::error::AppError::NotFound(format!("session {sid}")))?;
        let events = store.list_events(&sid, 1000)?;
        Ok(SessionDetail { session, events })
    })
    .await
    .map_err(|e| crate::error::AppError::Other(format!("join error: {e}")))?
}

/// `dashboard_stats(since)` → `DashboardStatsLite`.
#[tauri::command]
pub async fn dashboard_stats(
    state: tauri::State<'_, AppState>,
    since: f64,
) -> AppResult<DashboardStatsLite> {
    let store = Arc::clone(&state.store);
    tokio::task::spawn_blocking(move || {
        let s = store.dashboard_stats(since)?;
        Ok(DashboardStatsLite {
            total_cost_usd: s.total_cost_usd,
            session_count: s.session_count,
            error_count: s.error_count,
        })
    })
    .await
    .map_err(|e| crate::error::AppError::Other(format!("join error: {e}")))?
}

/// `get_settings()` → UI-shaped Settings (only Hermes password status for now).
#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
) -> AppResult<crate::ipc::Settings> {
    let secrets = Arc::clone(&state.secrets);
    tokio::task::spawn_blocking(move || {
        let hermes_password_set = crate::secrets::hermes_get(&secrets)
            .map(|opt| opt.is_some())
            .unwrap_or(false);
        let mut sources = std::collections::BTreeMap::new();
        sources.insert("claude".into(), true);
        sources.insert("opencode".into(), true);
        sources.insert("hermes".into(), true);
        Ok(crate::ipc::Settings { hermes_password_set, sources })
    })
    .await
    .map_err(|e| crate::error::AppError::Other(format!("join error: {e}")))?
}

/// `set_hermes_password(password)` → stores the secret in the OS keychain.
#[tauri::command]
pub async fn set_hermes_password(
    state: tauri::State<'_, AppState>,
    password: String,
) -> AppResult<()> {
    if password.is_empty() {
        return Err(crate::error::AppError::Other(
            "password cannot be empty".into(),
        ));
    }
    let secrets = Arc::clone(&state.secrets);
    tokio::task::spawn_blocking(move || crate::secrets::hermes_set(&secrets, &password))
        .await
        .map_err(|e| crate::error::AppError::Other(format!("join error: {e}")))??;
    Ok(())
}

#[allow(dead_code)]
fn _kind_roundtrip(k: &str) -> EventKind {
    EventKind::from_token(k)
}