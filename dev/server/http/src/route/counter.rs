use std::{fmt::Display, ops::Mul};

use axum::{
    extract::{Path, State},
    response::Result,
    routing::get,
    Json, Router,
};
use num::{BigInt, One, Zero};
use serde::{Deserialize, Serialize};

use crate::{
    error::{
        counter::CounterError,
        kind::{Retriable, Unreachable},
        AppError, Logged,
    },
    state::AppState,
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
pub async fn counter<T>(State(AppState { counter, .. }): State<AppState>) -> Result<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let read = counter.read().map_err(|e| AppError::<Retriable>::wrap(Logged(e.to_string())))?;
    let count = read.clone().count.try_into().map_err(CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

#[tracing::instrument]
pub async fn increment<T>(state: State<AppState>) -> Result<Json<CounterResponse<T>>>
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
) -> Result<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|e| AppError::<Retriable>::wrap(Logged(e.to_string())))?;
    write.count += &value.parse().map_err(|e| CounterError::CannotParse(e, value))?;
    let count = write.clone().count.try_into().map_err(CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

#[tracing::instrument]
pub async fn decrement<T>(state: State<AppState>) -> Result<Json<CounterResponse<T>>>
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
) -> Result<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt>,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|e| AppError::<Retriable>::wrap(Logged(e.to_string())))?;
    write.count -= &value.parse().map_err(|e| CounterError::CannotParse(e, value))?;
    let count = write.clone().count.try_into().map_err(CounterError::Overflow)?;
    Ok(Json(CounterResponse { count }))
}

#[tracing::instrument]
pub async fn reset<T>(State(AppState { counter, .. }): State<AppState>) -> Result<Json<CounterResponse<T>>>
where
    T: TryFrom<BigInt> + One + Display,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    let mut write = counter.write().map_err(|e| AppError::<Retriable>::wrap(Logged(e.to_string())))?;
    write.count = BigInt::zero();
    let count = write.clone().count.try_into().map_err(AppError::<Unreachable>::wrap)?;
    Ok(Json(CounterResponse { count }))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::{
        error::{
            kind::{BadRequest, Kind},
            ErrorResponseInner, APP_DEFAULT_ERROR_CODE,
        },
        route::{app_with, tests::call_with_assert},
    };

    use super::*;

    #[tokio::test]
    async fn test_counter() {
        let mut app = app_with(Default::default());

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
            call_with_assert(&mut app, req, StatusCode::OK, exp).await;
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
            call_with_assert(&mut app, req, StatusCode::OK, exp).await;
        }
    }

    #[tokio::test]
    async fn test_counter_overflow() {
        let mut app = app_with(Default::default());

        let req = Request::builder().uri(format!("/counter/increment/{}", i64::MAX)).body(Body::empty()).unwrap();
        call_with_assert(&mut app, req, StatusCode::OK, CounterResponse { count: i64::MAX }).await;

        let req = Request::builder().uri("/counter/increment").body(Body::empty()).unwrap();
        call_with_assert(
            &mut app,
            req,
            APP_DEFAULT_ERROR_CODE,
            ErrorResponseInner { msg: BadRequest::msg().to_string(), detail: CounterError::Overflow(()).to_string() },
        )
        .await;

        let req = Request::builder().uri("/counter/decrement").body(Body::empty()).unwrap();
        call_with_assert(&mut app, req, StatusCode::OK, CounterResponse { count: i64::MAX }).await;
    }

    #[tokio::test]
    async fn test_counter_parse_error() {
        let mut app = app_with(Default::default());

        let req = Request::builder().uri("/counter/increment/abc").body(Body::empty()).unwrap();
        call_with_assert(
            &mut app,
            req,
            APP_DEFAULT_ERROR_CODE,
            ErrorResponseInner {
                msg: BadRequest::msg().to_string(),
                detail: CounterError::CannotParse((), "abc".to_string()).to_string(),
            },
        )
        .await;

        let req = Request::builder().uri("/counter/increment").body(Body::empty()).unwrap();
        call_with_assert(&mut app, req, StatusCode::OK, CounterResponse { count: 1 }).await;
    }
}
