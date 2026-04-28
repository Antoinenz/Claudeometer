use crate::claude::{fetch_claude_usage, verify_api_key, UsageData};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_store::StoreExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthState {
    pub mode: String, // "none" | "session_key" | "api_key"
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub launch_at_startup: bool,
    pub minimize_to_tray: bool,
    pub desktop_notifications: bool,
    pub notification_threshold: u8,
    pub poll_interval_secs: u64,
    pub ntfy_enabled: bool,
    pub ntfy_server: String,
    pub ntfy_topic: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            minimize_to_tray: true,
            desktop_notifications: true,
            notification_threshold: 80,
            poll_interval_secs: 60,
            ntfy_enabled: false,
            ntfy_server: "https://ntfy.sh".to_string(),
            ntfy_topic: "claudeometer".to_string(),
        }
    }
}

#[tauri::command]
pub async fn get_auth_state(app: AppHandle) -> AuthState {
    let store = app.store("store.json").unwrap();
    let mode = store
        .get("auth_mode")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "none".to_string());
    let email = store
        .get("email")
        .and_then(|v| v.as_str().map(str::to_string));
    AuthState { mode, email }
}

#[tauri::command]
pub async fn save_session_key(app: AppHandle, key: String) -> Result<AuthState, String> {
    let usage = fetch_claude_usage(&key).await?;
    let email = usage.email.clone();
    let store = app.store("store.json").unwrap();
    store.set("auth_mode", "session_key");
    store.set("session_key", key.clone());
    if let Some(ref e) = email {
        store.set("email", e.clone());
    }
    store.save().map_err(|e| e.to_string())?;
    Ok(AuthState {
        mode: "session_key".to_string(),
        email,
    })
}

#[tauri::command]
pub async fn save_api_key(app: AppHandle, key: String) -> Result<AuthState, String> {
    verify_api_key(&key).await?;
    let store = app.store("store.json").unwrap();
    store.set("auth_mode", "api_key");
    store.set("api_key", key.clone());
    store.save().map_err(|e| e.to_string())?;
    Ok(AuthState {
        mode: "api_key".to_string(),
        email: None,
    })
}

#[tauri::command]
pub async fn logout(app: AppHandle) -> Result<(), String> {
    let store = app.store("store.json").unwrap();
    store.set("auth_mode", "none");
    store.delete("session_key");
    store.delete("api_key");
    store.delete("email");
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn fetch_usage(app: AppHandle) -> Result<UsageData, String> {
    let store = app.store("store.json").unwrap();
    let mode = store
        .get("auth_mode")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();

    match mode.as_str() {
        "session_key" => {
            let key = store
                .get("session_key")
                .and_then(|v| v.as_str().map(str::to_string))
                .ok_or("No session key stored")?;
            fetch_claude_usage(&key).await
        }
        "api_key" => {
            let key = store
                .get("api_key")
                .and_then(|v| v.as_str().map(str::to_string))
                .ok_or("No API key stored")?;
            verify_api_key(&key).await
        }
        _ => Err("Not authenticated".to_string()),
    }
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Settings {
    let store = app.store("store.json").unwrap();
    let raw = store.get("settings");
    match raw {
        Some(v) => serde_json::from_value(v).unwrap_or_default(),
        None => Settings::default(),
    }
}

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    settings: Settings,
) -> Result<(), String> {
    // Apply autostart
    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let mgr = app.autolaunch();
        if settings.launch_at_startup {
            mgr.enable().map_err(|e| e.to_string())?;
        } else {
            mgr.disable().map_err(|e| e.to_string())?;
        }
    }

    let store = app.store("store.json").unwrap();
    store.set(
        "settings",
        serde_json::to_value(&settings).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn send_ntfy(
    server: String,
    topic: String,
    title: String,
    body: String,
) -> Result<(), String> {
    let url = format!("{}/{}", server.trim_end_matches('/'), topic);
    reqwest::Client::new()
        .post(&url)
        .header("Title", &title)
        .header("Priority", "default")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("ntfy error: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn show_desktop_notification(app: AppHandle, title: String, body: String) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}
