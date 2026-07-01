//! Builds the async runtime and drives the poller + HTTP server. Split out
//! from `main.rs` so it can be invoked two ways: directly, when the user
//! runs `claudeometer-service run` in a terminal or under systemd/launchd;
//! and from `svc::windows::service_main`, which runs on the Windows Service
//! Control Manager's dispatcher thread and is not itself async.

use crate::api::AppState;
use crate::config;
use crate::poller;
use std::sync::{Arc, Mutex};

/// Optional overrides so `claudeometer-service run --bind ... --interval ...`
/// can be used for a quick local test without editing the config file.
pub struct RunOverrides {
    pub bind: Option<String>,
    pub interval_secs: Option<u64>,
}

/// Builds a Tokio runtime and blocks forever serving the API + poller.
/// Safe to call from a plain `fn main()` or from a non-async OS callback
/// (e.g. the Windows service dispatcher thread) since it owns its runtime.
pub fn run_foreground_blocking(overrides: RunOverrides) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to start async runtime: {e}"))?;
    rt.block_on(run_async(overrides))
}

async fn run_async(overrides: RunOverrides) -> Result<(), String> {
    let mut cfg = config::load_config();
    if let Some(bind) = overrides.bind {
        cfg.bind = bind;
    }
    if let Some(interval) = overrides.interval_secs {
        cfg.poll_interval_secs = interval;
    }

    let session_key = config::load_session_key()?;

    let is_loopback = cfg.bind.starts_with("127.0.0.1") || cfg.bind.starts_with("localhost");
    if !is_loopback && cfg.api_key.is_none() {
        eprintln!(
            "WARNING: binding to {} with no api_key set in the config file.\n\
             Anyone who can reach this address can read your Claude usage data.\n\
             Set \"api_key\" in {} or bind to 127.0.0.1 instead.",
            cfg.bind,
            config::config_dir().join("config.json").display()
        );
    }

    let state = AppState {
        cache: Arc::new(Mutex::new(None)),
        api_key: cfg.api_key.clone(),
        session_key: Arc::new(session_key),
    };

    poller::spawn(state.clone(), cfg.poll_interval_secs);
    crate::api::run_server(state, &cfg.bind).await
}
