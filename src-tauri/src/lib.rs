//! Spire Bridge — Tauri application library entry point.
//!
//! Phase 1 wires up the bare minimum needed to verify the dev loop:
//!   - Tauri 2 builder with the updater plugin registered (check deferred)
//!   - default window per `tauri.conf.json`
//!   - module surface for IPC commands (Task 6), sync engine (Task 5),
//!     sources (Task 4), and store (Task 3).

use tracing::info;

pub mod error;
pub mod ipc;
pub mod secrets;
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
        .invoke_handler(tauri::generate_handler![
            ipc::commands::list_sessions,
            ipc::commands::get_session,
            ipc::commands::dashboard_stats,
        ])
        .setup(move |app| {
                    use tauri::Emitter;

            // Build the AppState and stash it for IPC handlers. The actual
            // sync engine construction (sources + LiveHub + Store) is wired
            // up here in the Phase 2 cut; for Phase 1 we keep the Store +
            // LiveHub ready and let the sources be plugged in once the
            // adapters (Task 4 orphan files) get wired.
            use tauri::Manager;
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let db_path = data_dir.join("spire.db");
            if let Some(parent) = db_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let store = match store::Store::open(&db_path) {
                Ok(s) => std::sync::Arc::new(s),
                Err(e) => {
                    tracing::warn!(error = %e, "store open failed; running with empty state");
                    // No public in-memory Store API outside tests; open a
                    // throwaway file in tmpdir as a fallback so the IPC
                    // layer can still respond.
                    let fallback = std::env::temp_dir().join("spire-fallback.db");
                    std::sync::Arc::new(
                        store::Store::open(&fallback)
                            .expect("fallback store open"),
                    )
                }
            };
            let live = std::sync::Arc::new(sync::live::LiveHub::new());
            let keyring: secrets::SharedSecretStore =
                std::sync::Arc::new(secrets::SystemKeyring);
            app.manage(ipc::AppState {
                store,
                live: live.clone(),
                secrets: keyring,
            });
            // Forward every canonical event published to LiveHub
            // out to the renderer over Tauri IPC. The renderer
            // subscribes via `listen("bridge://event", cb)`.
            let handle = app.handle().clone();
            let mut rx = live.subscribe();
            tokio::spawn(async move {
                while let Ok(ev) = rx.recv().await {
                    if let Err(e) = handle.emit("bridge://event", &ev) {
                        tracing::warn!(error = %e, "event emit failed");
                    }
                }
            });
            info!("AppState registered; commands live");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Spire Bridge");
}