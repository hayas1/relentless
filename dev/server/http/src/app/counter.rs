use std::{fmt::Display, ops::Mul};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use num::{BigInt, One, Zero};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    app::AppState,
    error::{AppResult, AsStatusCode, IntoAppResult, Retriable},
};

pub fn route_counter() -> Router<AppState> {
    Router::new()
        .route("/", get(counter::<i64>))
        .route("/s", get(counter::<BInt>))
        .route("/increment", get(increment::<i64>))
        .route("/increment/{value}", get(increment_with::<i64>))
        .route("/increments", get(increment::<BInt>))
        .route("/increments/{value}", get(increment_with::<BInt>))
        .route("/decrement", get(decrement::<i64>))
        .route("/decrement/{value}", get(decrement_with::<i64>))
        .route("/decrements", get(decrement::<BInt>))
        .route("/decrements/{value}", get(decrement_with::<BInt>))
        .route("/reset", get(reset::<i64>))
        .route("/resets", get(reset::<BInt>))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
pub struct CounterResponse<T> {
    pub count: T,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
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

#[tracing::instrument]
pub async fn counter<T>(
    State(AppState { counter, .. }): State<AppState>,
) -> AppResult<Json<CounterResponse<T>>, CounterError>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let read = counter.read().map_err(|e| e.to_string()).response(Retriable.into())?;
    let count = read.clone().count.try_into().response(CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

#[tracing::instrument]
pub async fn increment<T>(state: State<AppState>) -> AppResult<Json<CounterResponse<T>>, CounterError>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    increment_with::<T>(state, Path(T::one().to_string())).await
}
#[tracing::instrument]
pub async fn increment_with<T>(
    State(AppState { counter, .. }): State<AppState>,
    Path(value): Path<String>,
) -> AppResult<Json<CounterResponse<T>>, CounterError>
where
    T: TryFrom<BigInt>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|e| e.to_string()).response(Retriable.into())?;
    write.count += &value.parse().response(CounterError::CannotParse(value))?;
    let count = write.clone().count.try_into().response(CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

#[tracing::instrument]
pub async fn decrement<T>(state: State<AppState>) -> AppResult<Json<CounterResponse<T>>, CounterError>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    decrement_with::<T>(state, Path(T::one().to_string())).await
}
#[tracing::instrument]
pub async fn decrement_with<T>(
    State(AppState { counter, .. }): State<AppState>,
    Path(value): Path<String>,
) -> AppResult<Json<CounterResponse<T>>, CounterError>
where
    T: TryFrom<BigInt>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|e| e.to_string()).response(Retriable.into())?;
    write.count -= &value.parse().response(CounterError::CannotParse(value))?;
    let count = write.clone().count.try_into().response(CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

#[tracing::instrument]
pub async fn reset<T>(
    State(AppState { counter, .. }): State<AppState>,
) -> AppResult<Json<CounterResponse<T>>, CounterError>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|e| e.to_string()).response(Retriable.into())?;
    write.count = BigInt::zero();
    let count = write.clone().count.try_into().unwrap_or_else(|_| unreachable!());
    Ok(Json(CounterResponse { count }))
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum CounterError {
    #[error(transparent)]
    Retriable(#[from] Retriable),

    #[error("overflow counter")]
    Overflow,

    #[error("cannot parse value as integer: {0}")]
    CannotParse(String),
}
impl AsStatusCode for CounterError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Retriable(r) => r.status_code(),
            Self::Overflow => StatusCode::INTERNAL_SERVER_ERROR,
            Self::CannotParse(_) => StatusCode::BAD_REQUEST,
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::{
        app::{tests::call, AppRouter},
        error::{ErrorResponse, APP_DEFAULT_ERROR_CODE},
    };

    use super::*;

    #[tokio::test]
    async fn test_counter() {
        let mut service = AppRouter::default().service();

        let scenario = [
            ("/counter", CounterResponse { count: 0 }),
            ("/counter/increment", CounterResponse { count: 1 }),
            ("/counter/reset", CounterResponse { count: 0 }),
            ("/counter/decrement", CounterResponse { count: -1 }),
            ("/counter/increment/5", CounterResponse { count: 4 }),
            ("/counter/decrement/13", CounterResponse { count: -9 }),
        ];
        for (uri, exp) in scenario {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let res = call(&mut service, req).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(&exp, res.body());
        }

        let scenario2 = [
            (
                "/counter/increments/99999999999999999999999",
                CounterResponse { count: BInt("99999999999999999999990".parse().unwrap()) },
            ),
            (
                "/counter/increments/99999999999999999999999",
                CounterResponse { count: BInt("199999999999999999999989".parse().unwrap()) },
            ),
            (
                "/counter/decrements/99999999999999999999999",
                CounterResponse { count: BInt("99999999999999999999990".parse().unwrap()) },
            ),
            ("/counter/decrements", CounterResponse { count: BInt("99999999999999999999989".parse().unwrap()) }),
        ];
        for (uri, exp) in scenario2 {
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let res = call(&mut service, req).await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(&exp, res.body());
        }
    }

    #[tokio::test]
    async fn test_counter_overflow() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri(format!("/counter/increment/{}", i64::MAX)).body(Body::empty()).unwrap();
        let res = call(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(&CounterResponse { count: i64::MAX }, res.body());

        let req = Request::builder().uri("/counter/increment").body(Body::empty()).unwrap();
        let res = call(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(matches!(
            res.body(),
            &ErrorResponse { ref error, serde: Some(CounterError::Overflow) }
            if error == "overflow counter"
        ));

        let req = Request::builder().uri("/counter/decrement").body(Body::empty()).unwrap();
        let res = call(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(&CounterResponse { count: i64::MAX }, res.body());
    }

    #[tokio::test]
    async fn test_counter_parse_error() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/counter/increment/abc").body(Body::empty()).unwrap();
        let res = call(&mut service, req).await.unwrap();
        assert_eq!(res.status(), APP_DEFAULT_ERROR_CODE);
        assert!(matches!(
            res.body(),
            &ErrorResponse { ref error, serde: Some(CounterError::CannotParse(ref v)) }
            if error == "cannot parse value as integer: abc" && v == "abc"
        ));

        let req = Request::builder().uri("/counter/increment").body(Body::empty()).unwrap();
        let res = call(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(&CounterResponse { count: 1 }, res.body());
    }
}
