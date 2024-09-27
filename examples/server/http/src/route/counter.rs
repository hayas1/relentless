use std::{fmt::Display, ops::Mul};

use axum::{
    extract::{Path, State},
    routing::get,
    Json,
};
use num::{BigInt, One, Zero};
use serde::{Deserialize, Serialize};

use crate::{
    error::{counter::CounterError, AppResult},
    state::AppState,
};

pub fn route_counter() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(counter::<i64>))
        .route("/s", get(counter::<BInt>))
        .route("/increment", get(increment::<i64>))
        .route("/increment/:value", get(increment_with::<i64>))
        .route("/increments", get(increment::<BInt>))
        .route("/increments/:value", get(increment_with::<BInt>))
        .route("/decrement", get(decrement::<i64>))
        .route("/decrement/:value", get(decrement_with::<i64>))
        .route("/decrements", get(decrement::<BInt>))
        .route("/decrements/:value", get(decrement_with::<BInt>))
        .route("/reset", get(reset::<i64>))
        .route("/resets", get(reset::<BInt>))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CounterState {
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

// TODO better implementation for increment/increments ?
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterResponse<T> {
    pub count: T,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BInt(#[serde(with = "bigint_string")] pub BigInt);
impl From<BigInt> for BInt {
    fn from(value: BigInt) -> Self {
        Self(value)
    }
}
impl One for BInt {
    fn one() -> Self {
        Self(BigInt::one())
    }
}
impl Mul for BInt {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}
impl Display for BInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub async fn counter<T>(State(AppState { counter, .. }): State<AppState>) -> AppResult<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let read = counter.read().map_err(|_| CounterError::Retriable)?;
    let count = read.clone().count.try_into().map_err(|_| CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

pub async fn increment<T>(state: State<AppState>) -> AppResult<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    increment_with::<T>(state, Path(T::one().to_string())).await
}
pub async fn increment_with<T>(
    State(AppState { counter, .. }): State<AppState>,
    Path(value): Path<String>,
) -> AppResult<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|_| CounterError::Retriable)?;
    write.count += &value.parse().map_err(|_| CounterError::CannotParse(value))?;
    let count = write.clone().count.try_into().map_err(|_| CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

pub async fn decrement<T>(state: State<AppState>) -> AppResult<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    decrement_with::<T>(state, Path(T::one().to_string())).await
}
pub async fn decrement_with<T>(
    State(AppState { counter, .. }): State<AppState>,
    Path(value): Path<String>,
) -> AppResult<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|_| CounterError::Retriable)?;
    write.count -= &value.parse().map_err(|_| CounterError::CannotParse(value))?;
    let count = write.clone().count.try_into().map_err(|_| CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

pub async fn reset<T>(State(AppState { counter, .. }): State<AppState>) -> AppResult<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|_| CounterError::Retriable)?;
    write.count = BigInt::zero();
    let count = write.clone().count.try_into().map_err(|_| CounterError::Unreachable)?;
    Ok(Json(CounterResponse { count }))
}
