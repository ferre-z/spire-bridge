//! OS keychain bridge.
//!
//! Sensitive credentials (Hermes password, OAuth refresh tokens, etc.) live
//! in the OS-native credential store. We never write them to disk, never log
//! them, and never put them in renderer-accessible state.
//!
//! Service identifier is fixed: `com.spire-bridge.app`. Keys are short,
//! unique strings (see `Key`); adding a new secret is a code change (intentional
//! — no stringly-typed secrets anywhere else).
//!
//! The keyring crate returns `Err` when the system keychain is unavailable
//! (Linux without `dbus`, headless CI, etc.). We surface that as `AppError::Auth`
//! so the UI can show an actionable message instead of a cryptic panic.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Service name registered with the OS keychain. Must match the app's
/// `bundle_identifier` in `tauri.conf.json` for macOS, and our
/// consistent identity on Linux/Windows.
pub const KEYRING_SERVICE: &str = "com.spire-bridge.app";

/// Discrete credential slots. Adding a new secret = adding a variant here
/// so `Settings` (renderer side) can reflect it without a stringly-typed API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    /// Hermes agent HTTP basic-auth password (read from renderer settings).
    HermesPassword,
    /// OAuth refresh token for the Hermes source (out of scope for v1, but the
    /// slot exists so we don't churn the keyring layout when it's wired).
    HermesOAuthRefresh,
}

impl Key {
    /// String form used as the keychain "username" field. Stable across versions.
    pub fn as_str(&self) -> &'static str {
        match self {
            Key::HermesPassword => "hermes_password",
            Key::HermesOAuthRefresh => "hermes_oauth_refresh",
        }
    }

    /// Whether this key is set has been observed at least once.
    pub fn is_configured(&self) -> bool {
        // Out-of-band peek so the UI can show "configured" vs "missing".
        // Returns false on any error (dbus missing, no entry, etc.) — that's
        // intentionally pessimistic; the renderer treats false as "ask the user".
        match keyring::Entry::new(KEYRING_SERVICE, self.as_str()) {
            Ok(entry) => entry.get_password().is_ok(),
            Err(_) => false,
        }
    }
}

/// Trait so tests can swap in an in-memory backend without touching `dbus`.
///
/// `keyring` itself doesn't expose a trait (as of v3.6) — but we wrap our two
/// operations in this so unit tests run offline.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: Key) -> AppResult<String>;
    fn set(&self, key: Key, value: &str) -> AppResult<()>;
    fn delete(&self, key: Key) -> AppResult<()>;
}

/// Production backend: delegates to `keyring` 3.x.
pub struct SystemKeyring;

impl SecretStore for SystemKeyring {
    fn get(&self, key: Key) -> AppResult<String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, key.as_str())
            .map_err(|e| AppError::Auth(format!("keyring open failed: {e}")))?;
        match entry.get_password() {
            Ok(s) => Ok(s),
            Err(keyring::Error::NoEntry) => Err(AppError::NotFound(key.as_str().into())),
            Err(e) => Err(AppError::Auth(format!("keyring get failed: {e}"))),
        }
    }

    fn set(&self, key: Key, value: &str) -> AppResult<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, key.as_str())
            .map_err(|e| AppError::Auth(format!("keyring open failed: {e}")))?;
        entry
            .set_password(value)
            .map_err(|e| AppError::Auth(format!("keyring set failed: {e}")))?;
        Ok(())
    }

    fn delete(&self, key: Key) -> AppResult<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, key.as_str())
            .map_err(|e| AppError::Auth(format!("keyring open failed: {e}")))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            // Deleting a missing entry is a no-op.
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Auth(format!("keyring delete failed: {e}"))),
        }
    }
}

/// In-memory backend for tests. Drop on app shutdown clears everything.
#[derive(Default)]
pub struct InMemoryKeyring {
    inner: parking_lot::Mutex<std::collections::HashMap<&'static str, String>>,
}

impl SecretStore for InMemoryKeyring {
    fn get(&self, key: Key) -> AppResult<String> {
        let g = self.inner.lock();
        g.get(key.as_str())
            .cloned()
            .ok_or_else(|| AppError::NotFound(key.as_str().into()))
    }

    fn set(&self, key: Key, value: &str) -> AppResult<()> {
        self.inner.lock().insert(key.as_str(), value.to_string());
        Ok(())
    }

    fn delete(&self, key: Key) -> AppResult<()> {
        self.inner.lock().remove(key.as_str());
        Ok(())
    }
}

/// Shared, read-mostly handle to the active secret backend. Tauri state.
pub type SharedSecretStore = Arc<dyn SecretStore>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_round_trip() {
        let s = InMemoryKeyring::default();
        assert!(s.get(Key::HermesPassword).is_err());
        s.set(Key::HermesPassword, "hunter2").unwrap();
        assert_eq!(s.get(Key::HermesPassword).unwrap(), "hunter2");
        s.delete(Key::HermesPassword).unwrap();
        assert!(s.get(Key::HermesPassword).is_err());
        // Delete on missing is not an error.
        s.delete(Key::HermesPassword).unwrap();
    }

    #[test]
    fn keys_have_stable_string_form() {
        // Guard against accidental key rename — these strings land in the
        // user's keychain on first run, so breaking the mapping is a data-loss
        // event for them.
        assert_eq!(Key::HermesPassword.as_str(), "hermes_password");
        assert_eq!(Key::HermesOAuthRefresh.as_str(), "hermes_oauth_refresh");
    }

    #[test]
    fn service_name_is_locked() {
        assert_eq!(KEYRING_SERVICE, "com.spire-bridge.app");
    }
}

/// Convenience: store the Hermes password via the shared store.
pub fn hermes_set(store: &SharedSecretStore, password: &str) -> AppResult<()> {
    store.set(Key::HermesPassword, password)
}

/// Convenience: read the Hermes password via the shared store.
/// Returns `Ok(None)` when the secret isn't set, `Ok(Some(_))` when it is.
pub fn hermes_get(store: &SharedSecretStore) -> AppResult<Option<String>> {
    match store.get(Key::HermesPassword) {
        Ok(s) if s.is_empty() => Ok(None),
        Ok(s) => Ok(Some(s)),
        Err(e) => Err(e),
    }
}

/// Convenience: clear the Hermes password via the shared store.
pub fn hermes_clear(store: &SharedSecretStore) -> AppResult<()> {
    store.delete(Key::HermesPassword)
}
