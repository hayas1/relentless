use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{error::AppResult, state::AppState};

pub fn route_counter() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(counter))
        .route("/increment", get(increment))
        .route("/increment/", get(increment))
        .route("/increment/:value", get(increment_with))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Counter {
    pub count: usize,
}

pub async fn counter(State(AppState { counter, .. }): State<AppState>) -> AppResult<impl IntoResponse> {
    let read = counter.read().map_err(|e| anyhow!(e.to_string()))?;
    Ok(Json(read.clone()))
}
pub async fn increment(state: State<AppState>) -> AppResult<impl IntoResponse> {
    increment_with(state, Path(1)).await
}
pub async fn increment_with(
    State(AppState { counter, .. }): State<AppState>,
    Path(value): Path<usize>,
) -> AppResult<impl IntoResponse> {
    let mut write = counter.write().map_err(|e| anyhow!(e.to_string()))?;
    write.count += value;
    Ok(Json(write.clone()))
}
