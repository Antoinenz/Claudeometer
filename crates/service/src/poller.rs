//! Background polling loop — the headless equivalent of
//! `src-tauri/src/lib.rs::start_polling`/`poll_once`, minus the Tauri event
//! bus (there are no windows to notify here). Logs to stdout/stderr with a
//! timestamp so `journalctl --user -u claudeometer -f` (or the macOS/Windows
//! equivalents) show useful history.

use crate::api::{now_ms, AppState};
use claudeometer_core::fetch_claude_usage;
use std::time::Duration;

pub fn spawn(state: AppState, interval_secs: u64) {
    tokio::spawn(async move {
        loop {
            poll_once(&state).await;
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

pub async fn poll_once(state: &AppState) {
    let now = chrono::Utc::now().to_rfc3339();
    match fetch_claude_usage(&state.session_key).await {
        Ok(usage) => {
            let five_hour = usage
                .five_hour
                .as_ref()
                .map(|w| format!("{:.0}%", w.utilization * 100.0))
                .unwrap_or_else(|| "—".to_string());
            let seven_day = usage
                .seven_day
                .as_ref()
                .map(|w| format!("{:.0}%", w.utilization * 100.0))
                .unwrap_or_else(|| "—".to_string());
            println!("[{now}] poll ok — 5h {five_hour}, 7d {seven_day}");
            *state.cache.lock().unwrap() = Some((usage, now_ms()));
        }
        Err(e) => {
            eprintln!("[{now}] poll failed: {e}");
        }
    }
}
