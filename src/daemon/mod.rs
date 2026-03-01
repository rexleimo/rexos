use std::net::SocketAddr;
use std::time::Instant;

use anyhow::Context;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

#[derive(Debug, Clone)]
struct AppState {
    started_at: Instant,
}

#[derive(Debug, Serialize)]
struct HealthzResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    status: &'static str,
    uptime_ms: u128,
}

pub fn app() -> Router {
    let state = AppState {
        started_at: Instant::now(),
    };

    Router::new()
        .route("/healthz", get(healthz))
        .route("/status", get(status))
        .with_state(state)
}

pub async fn serve(addr: SocketAddr) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    axum::serve(listener, app())
        .await
        .context("serve http")?;
    Ok(())
}

async fn healthz() -> Json<HealthzResponse> {
    Json(HealthzResponse { status: "ok" })
}

async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "ok",
        uptime_ms: state.started_at.elapsed().as_millis(),
    })
}

