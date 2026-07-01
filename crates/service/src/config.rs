//! Config file + credential storage for the headless service.
//!
//! Deliberately much smaller than the GUI's `Settings` struct
//! (`src-tauri/src/commands.rs`) — a server-facing binary shouldn't expose
//! remote settings mutation, and most of the GUI's settings (tray, autostart,
//! desktop notifications) don't apply headless at all.

use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Same keyring service/account names the desktop app uses
/// (`src-tauri/src/commands.rs::KEYRING_SERVICE/KEYRING_ACCOUNT`) — a machine
/// that already has the GUI signed in needs no separate `login` step.
const KEYRING_SERVICE: &str = "claudeometer";
const KEYRING_ACCOUNT: &str = "session_key";

fn default_bind() -> String {
    "127.0.0.1:7842".to_string()
}

fn default_poll_interval() -> u64 {
    60
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    /// If set, GET/POST requests must send `Authorization: Bearer <key>`.
    /// Strongly recommended whenever `bind` is not loopback-only.
    #[serde(default)]
    pub api_key: Option<String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            poll_interval_secs: default_poll_interval(),
            api_key: None,
        }
    }
}

/// `~/.config/claudeometer` on Linux/macOS, `%APPDATA%\claudeometer` on
/// Windows. Created on first use.
pub fn config_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claudeometer");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn session_key_fallback_path() -> PathBuf {
    config_dir().join("session_key")
}

pub fn load_config() -> ServiceConfig {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &ServiceConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())
}

fn keyring_entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|e| format!("keyring unavailable: {e}"))
}

/// Write `key` with 0600 permissions on Unix. Best-effort on Windows, where
/// `%APPDATA%` is already private to the current user by default.
fn write_fallback_file(key: &str) -> Result<(), String> {
    let path = session_key_fallback_path();
    fs::write(&path, key).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Store the session key, preferring the OS keyring and transparently
/// falling back to a `chmod 600` file when no secret-service/keychain is
/// available (the common case on a headless Linux server with no dbus
/// session). Returns which backend was actually used, so callers can tell
/// the user.
pub fn save_session_key(key: &str) -> Result<&'static str, String> {
    match keyring_entry().and_then(|e| e.set_password(key).map_err(|e| e.to_string())) {
        Ok(()) => Ok("OS keyring"),
        Err(_) => {
            write_fallback_file(key)?;
            Ok("local config file (chmod 600) — no OS keyring available on this machine")
        }
    }
}

/// Load the session key, checking the keyring first, then the fallback file.
pub fn load_session_key() -> Result<String, String> {
    if let Ok(key) = keyring_entry().and_then(|e| e.get_password().map_err(|e| e.to_string())) {
        return Ok(key);
    }
    fs::read_to_string(session_key_fallback_path())
        .map_err(|_| "Not signed in — run `claudeometer-service login <session-key>` first".to_string())
}

pub fn delete_session_key() {
    if let Ok(entry) = keyring_entry() {
        let _ = entry.delete_password();
    }
    let _ = fs::remove_file(session_key_fallback_path());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises the exact decision this module exists for: try the OS
    /// keyring, and if that's unavailable (the common case on a headless
    /// server with no secret-service/dbus session — which is exactly the
    /// environment this test runs in), transparently fall back to the
    /// chmod-600 file, and round-trip correctly either way.
    #[test]
    fn session_key_roundtrip_via_keyring_or_fallback() {
        let key = "test-session-key-should-not-leak-anywhere-real";
        delete_session_key(); // start from a clean slate

        let backend = save_session_key(key).expect("save should succeed via keyring or file fallback");
        eprintln!("stored session key via: {backend}");

        let loaded = load_session_key().expect("load should find what we just saved");
        assert_eq!(loaded, key);

        if backend.starts_with("local config file") {
            let path = session_key_fallback_path();
            let meta = fs::metadata(&path).expect("fallback file should exist");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    meta.permissions().mode() & 0o777,
                    0o600,
                    "fallback session-key file must not be group/world readable"
                );
            }
        }

        delete_session_key();
        assert!(load_session_key().is_err(), "key should be gone after delete");
    }
}
