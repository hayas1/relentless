use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json,
};
use num::{BigInt, Zero};
use serde::{Deserialize, Serialize};

use crate::{error::AppResult, state::AppState};

pub fn route_counter() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(counter))
        .route("/increment", get(increment))
        .route("/increment/:value", get(increment_with))
        .route("/decrement", get(decrement))
        .route("/decrement/:value", get(decrement_with))
        .route("/reset", get(reset))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Counter {
    #[serde(with = "bigint_string")]
    pub count: BigInt,
}
mod bigint_string {
    use super::*;
    pub fn serialize<S>(value: &BigInt, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        value.to_string().serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<BigInt, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
    }
}

pub async fn counter(State(AppState { counter, .. }): State<AppState>) -> AppResult<impl IntoResponse> {
    let read = counter.read().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(Json(read.clone()))
}

pub async fn increment(state: State<AppState>) -> AppResult<impl IntoResponse> {
    increment_with(state, Path("1".to_string())).await
}
pub async fn increment_with(
    State(AppState { counter, .. }): State<AppState>,
    Path(value): Path<String>,
) -> AppResult<impl IntoResponse> {
    let mut write = counter.write().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    write.count += &value.parse().map_err(anyhow::Error::from)?;
    Ok(Json(write.clone()))
}

pub async fn decrement(state: State<AppState>) -> AppResult<impl IntoResponse> {
    decrement_with(state, Path("1".to_string())).await
}
pub async fn decrement_with(
    State(AppState { counter, .. }): State<AppState>,
    Path(value): Path<String>,
) -> AppResult<impl IntoResponse> {
    let mut write = counter.write().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    write.count -= &value.parse().map_err(anyhow::Error::from)?;
    Ok(Json(write.clone()))
}

pub async fn reset(State(AppState { counter, .. }): State<AppState>) -> AppResult<impl IntoResponse> {
    let mut write = counter.write().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    write.count = BigInt::zero();
    Ok(Json(write.clone()))
}
