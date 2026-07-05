//! Spire Bridge — Tauri application library entry point.
//!
//! Phase 1 wires up the bare minimum needed to verify the dev loop:
//!   - Tauri 2 builder with the updater plugin registered (check deferred)
//!   - default window per `tauri.conf.json`
//!   - module surface for IPC commands (Task 6), sync engine (Task 5),
//!     sources (Task 4), and store (Task 3).

use tracing::info;

pub mod error;
pub mod sources;
pub mod store;
pub mod sync;
pub use error::{AppError, AppResult};

/// Entry point invoked from `main.rs`. Boots the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise structured logging early so plugin / Tauri init logs
    // are captured. Default to "info" unless RUST_LOG is set.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .try_init();

    info!("Spire Bridge starting up");

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|_app| {
            // Updater check deferred to Task 16 (CI/build pipeline) — the
            // tauri-plugin-updater 2.10 API moved on; revisit when we wire
            // the public update endpoint and signing keys.
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Spire Bridge");
}