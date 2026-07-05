//! IPC command surface for Spire Bridge.
//!
//! Phase 1 ships a minimal set of commands: list_sessions, get_session,
//! dashboard_stats. Live event streaming is handled via `tauri::ipc::Channel`
//! from the renderer side; secrets live in `crate::secrets`.
//!
//! Commands are registered in `lib.rs::run` via
//! `tauri::generate_handler!`. The frontend invokes them by name
//! (snake_case) via `invoke<ReturnType>("command_name", { args })`.

use crate::error::AppResult;
use crate::sources::{CanonicalEvent, CanonicalSession};
use crate::store::Store;
use crate::sync::live::LiveHub;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod commands;
pub use commands::{dashboard_stats, get_session, list_sessions};

/// Application state injected into every command via `tauri::State<AppState>`.
///
/// All fields are wrapped in `Arc` so commands can clone them into spawned
/// tasks (e.g. live-event pumps) without taking the lock.
pub struct AppState {
    pub store: Arc<Store>,
    pub live: Arc<LiveHub>,
    pub secrets: crate::secrets::SharedSecretStore,
}

/// Filter for `list_sessions`. All fields optional — `None` means "no filter".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionFilter {
    pub source: Option<String>,
    pub since: Option<f64>,
    pub until: Option<f64>,
    pub search: Option<String>,
}

/// Combined session + its events, returned by `get_session`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub session: CanonicalSession,
    pub events: Vec<CanonicalEvent>,
}

/// Lightweight dashboard summary returned by `dashboard_stats`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardStatsLite {
    pub total_cost_usd: f64,
    pub session_count: i64,
    pub error_count: i64,
}