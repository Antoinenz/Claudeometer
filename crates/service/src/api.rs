//! The service's HTTP API. Deliberately a smaller surface than the GUI's
//! embedded API (`src-tauri/src/api.rs`): no remote settings mutation,
//! because this process is meant to be left running on a network-reachable
//! box. `GET /usage` is the endpoint an agent curls to decide whether to
//! stop working before it gets cut off — same response shape as the GUI's
//! `/usage`, so anything already scripted against that keeps working here.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use claudeometer_core::UsageData;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Cache = Arc<Mutex<Option<(UsageData, u64)>>>;

#[derive(Clone)]
pub struct AppState {
    pub cache: Cache,
    pub api_key: Option<String>,
    pub session_key: Arc<String>,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

async fn require_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if let Some(ref expected) = state.api_key {
        let token = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        if token != Some(expected.as_str()) {
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

async fn handle_root() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "app": "claudeometer-service",
        "version": APP_VERSION,
        "endpoints": [
            { "method": "GET",  "path": "/usage" },
            { "method": "POST", "path": "/usage/refresh" }
        ]
    }))
}

async fn handle_get_usage(State(state): State<AppState>) -> Response {
    match state.cache.lock().unwrap().clone() {
        Some((data, ts)) => Json(json!({ "data": data, "fetched_at_ms": ts })).into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "No data yet — try POST /usage/refresh"})),
        )
            .into_response(),
    }
}

async fn handle_refresh(State(state): State<AppState>) -> Response {
    tokio::spawn(async move {
        crate::poller::poll_once(&state).await;
    });
    Json(json!({"status": "refresh triggered"})).into_response()
}

/// Binds and serves forever. Returns only if the listener fails to bind.
pub async fn run_server(state: AppState, bind: &str) -> Result<(), String> {
    let router = Router::new()
        .route("/", get(handle_root))
        .route("/usage", get(handle_get_usage))
        .route("/usage/refresh", post(handle_refresh))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| format!("failed to bind {bind}: {e}"))?;
    println!("claudeometer-service listening on http://{bind}");
    axum::serve(listener, router)
        .await
        .map_err(|e| format!("server error: {e}"))
}
