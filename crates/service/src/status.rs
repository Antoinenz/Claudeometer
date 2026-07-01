//! `claudeometer-service status` — the "quick glance in the terminal" use
//! case. If a service is already running locally, read from it (cheap, no
//! extra hit against claude.ai); otherwise fall back to a one-shot direct
//! fetch using the stored session key.

use crate::config;
use claudeometer_core::{fetch_claude_usage, UsageData, UsageWindow};
use serde_json::Value;
use std::time::Duration;

pub async fn run(json: bool) -> Result<(), String> {
    let cfg = config::load_config();

    let (data, fetched_at_ms, source) =
        match fetch_from_running_service(&cfg.bind, cfg.api_key.as_deref()).await {
            Some((data, ts)) => (data, Some(ts), "running service"),
            None => {
                let key = config::load_session_key()?;
                let data = fetch_claude_usage(&key).await?;
                (data, None, "one-off fetch — no local service running")
            }
        };

    if json {
        let payload = serde_json::json!({
            "data": data,
            "fetched_at_ms": fetched_at_ms,
            "source": source,
        });
        println!("{}", serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?);
    } else {
        print_bars(&data, source);
    }
    Ok(())
}

async fn fetch_from_running_service(bind: &str, api_key: Option<&str>) -> Option<(UsageData, u64)> {
    let url = format!("http://{bind}/usage");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .build()
        .ok()?;
    let mut req = client.get(&url);
    if let Some(key) = api_key {
        req = req.bearer_auth(key);
    }
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: Value = resp.json().await.ok()?;
    let data: UsageData = serde_json::from_value(body.get("data")?.clone()).ok()?;
    let ts = body.get("fetched_at_ms")?.as_u64()?;
    Some((data, ts))
}

fn bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round().clamp(0.0, width as f64) as usize;
    format!("[{}{}]", "#".repeat(filled), ".".repeat(width - filled))
}

fn fmt_reset(resets_at: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(resets_at) {
        Ok(dt) => {
            let mins = (dt.timestamp() - chrono::Utc::now().timestamp()).max(0) / 60;
            if mins >= 60 * 24 {
                format!("resets in {}d {}h", mins / (60 * 24), (mins / 60) % 24)
            } else if mins >= 60 {
                format!("resets in {}h {}m", mins / 60, mins % 60)
            } else {
                format!("resets in {mins}m")
            }
        }
        Err(_) => String::new(),
    }
}

fn fmt_window(label: &str, w: &Option<UsageWindow>) {
    match w {
        Some(w) => {
            let pct = w.utilization * 100.0;
            let reset = w.resets_at.as_deref().map(fmt_reset).unwrap_or_default();
            println!("  {label:<16} {} {pct:>4.0}%  {reset}", bar(pct, 20));
        }
        None => println!("  {label:<16} (not available)"),
    }
}

fn print_bars(data: &UsageData, source: &str) {
    println!("Claude usage — {} tier ({source})", data.tier);
    fmt_window("5-hour", &data.five_hour);
    fmt_window("7-day", &data.seven_day);
    fmt_window("7-day (Sonnet)", &data.seven_day_sonnet);
}
