use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;
use tower_http::cors::CorsLayer;

use crate::commands::{self, Settings, UsageCache};

#[derive(Clone)]
struct ApiState {
    app: AppHandle,
}

fn load_settings(app: &AppHandle) -> Settings {
    app.store("store.json")
        .ok()
        .and_then(|s| s.get("settings"))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

// ── Auth middleware ──────────────────────────────────────────────────────────

async fn require_auth(State(state): State<ApiState>, request: Request, next: Next) -> Response {
    let settings = load_settings(&state.app);
    if settings.api_require_auth {
        let token = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::to_owned);
        let valid = !settings.api_key.is_empty()
            && token.as_deref() == Some(settings.api_key.as_str());
        if !valid {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Unauthorized",
                    "hint": "Pass your API key as 'Authorization: Bearer <key>'"
                })),
            )
                .into_response();
        }
    }
    next.run(request).await
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn handle_root(_state: State<ApiState>) -> impl IntoResponse {
    let version = env!("APP_VERSION");
    Json(json!({
        "status": "ok",
        "app": "Claudeometer",
        "version": version,
        "endpoints": [
            { "method": "GET",   "path": "/usage",         "permission": "read_usage"     },
            { "method": "POST",  "path": "/usage/refresh", "permission": "refresh"        },
            { "method": "GET",   "path": "/settings",      "permission": "read_settings"  },
            { "method": "PATCH", "path": "/settings",      "permission": "write_settings" },
        ]
    }))
}

async fn handle_get_usage(State(state): State<ApiState>) -> Response {
    let settings = load_settings(&state.app);
    if !settings.api_allow_read_usage {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "read_usage not enabled in API permissions"})),
        )
            .into_response();
    }
    match state.app.try_state::<UsageCache>() {
        Some(cache) => match cache.0.lock().unwrap().clone() {
            Some((data, ts)) => Json(json!({ "data": data, "fetched_at_ms": ts })).into_response(),
            None => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "No data yet — try POST /usage/refresh"})),
            )
                .into_response(),
        },
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Cache unavailable"})),
        )
            .into_response(),
    }
}

async fn handle_refresh(State(state): State<ApiState>) -> Response {
    let settings = load_settings(&state.app);
    if !settings.api_allow_refresh {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "refresh not enabled in API permissions"})),
        )
            .into_response();
    }
    let handle = state.app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = handle.emit("refresh-started", ());
        match commands::do_tray_refresh(&handle).await {
            Ok(()) => {
                let _ = handle.emit("refresh-cooldown", ());
            }
            Err(e) => {
                let _ = handle.emit("usage-error", e);
            }
        }
        let _ = handle.emit("refresh-done", ());
    });
    Json(json!({"status": "refresh triggered"})).into_response()
}

async fn handle_get_settings(State(state): State<ApiState>) -> Response {
    let settings = load_settings(&state.app);
    if !settings.api_allow_read_settings {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "read_settings not enabled in API permissions"})),
        )
            .into_response();
    }
    let mut v = serde_json::to_value(&settings).unwrap_or_default();
    if let Some(obj) = v.as_object_mut() {
        obj.remove("api_key");
    }
    Json(v).into_response()
}

#[derive(serde::Deserialize)]
struct SettingsPatch {
    auto_poll: Option<bool>,
    poll_interval_secs: Option<u64>,
    foreground_poll: Option<bool>,
    precise_timestamp: Option<bool>,
    hide_cooldown_badge: Option<bool>,
    show_reset_tooltip: Option<bool>,
}

async fn handle_patch_settings(
    State(state): State<ApiState>,
    Json(patch): Json<SettingsPatch>,
) -> Response {
    let settings = load_settings(&state.app);
    if !settings.api_allow_write_settings {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "write_settings not enabled in API permissions"})),
        )
            .into_response();
    }
    let mut updated = settings;
    if let Some(v) = patch.auto_poll {
        updated.auto_poll = v;
    }
    if let Some(v) = patch.poll_interval_secs {
        updated.poll_interval_secs = v;
    }
    if let Some(v) = patch.foreground_poll {
        updated.foreground_poll = v;
    }
    if let Some(v) = patch.precise_timestamp {
        updated.precise_timestamp = v;
    }
    if let Some(v) = patch.hide_cooldown_badge {
        updated.hide_cooldown_badge = v;
    }
    if let Some(v) = patch.show_reset_tooltip {
        updated.show_reset_tooltip = v;
    }

    let store = match state.app.store("store.json") {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let val = match serde_json::to_value(&updated) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    store.set("settings", val);
    let _ = store.save();
    Json(json!({"status": "updated"})).into_response()
}

// ── Server lifecycle ──────────────────────────────────────────────────────────

async fn run_server(app: AppHandle, bind: String) {
    let state = ApiState { app };
    let router = Router::new()
        .route("/", get(handle_root))
        .route("/usage", get(handle_get_usage))
        .route("/usage/refresh", post(handle_refresh))
        .route(
            "/settings",
            get(handle_get_settings).patch(handle_patch_settings),
        )
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(CorsLayer::permissive())
        .with_state(state);

    if let Ok(listener) = tokio::net::TcpListener::bind(&bind).await {
        let _ = axum::serve(listener, router).await;
    }
}

pub fn apply(app: &AppHandle, settings: &Settings) {
    let handle_state = match app.try_state::<crate::ApiServerHandle>() {
        Some(h) => h,
        None => return,
    };
    let mut lock = handle_state.0.lock().unwrap();
    if let Some(h) = lock.take() {
        h.abort();
    }
    if !settings.api_enabled {
        return;
    }
    let bind = if settings.api_local_only {
        format!("127.0.0.1:{}", settings.api_port)
    } else {
        format!("0.0.0.0:{}", settings.api_port)
    };
    let app_clone = app.clone();
    *lock = Some(tauri::async_runtime::spawn(async move {
        run_server(app_clone, bind).await;
    }));
}
