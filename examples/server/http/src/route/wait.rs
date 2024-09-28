use std::time::Duration;

use axum::{extract::Path, response::Result, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::state::AppState;

pub fn route_wait() -> Router<AppState> {
    Router::new()
        .route("/:duration", get(wait))
        .route("/:duration/s", get(wait))
        .route("/:duration/ms", get(wait_ms))
        .route("/:duration/ns", get(wait_ns))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitResponse {}

#[tracing::instrument]
pub async fn wait(Path(duration): Path<u64>) -> Result<Json<WaitResponse>> {
    sleep(Duration::from_secs(duration)).await;
    Ok(Json(WaitResponse {}))
}

#[tracing::instrument]
pub async fn wait_ms(Path(duration): Path<u64>) -> Result<Json<WaitResponse>> {
    sleep(Duration::from_millis(duration)).await;
    Ok(Json(WaitResponse {}))
}

#[tracing::instrument]
pub async fn wait_ns(Path(duration): Path<u64>) -> Result<Json<WaitResponse>> {
    sleep(Duration::from_nanos(duration)).await;
    Ok(Json(WaitResponse {}))
}
