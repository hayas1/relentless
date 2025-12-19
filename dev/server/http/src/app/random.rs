use std::{
    convert::Infallible,
    fmt::{Debug, Display},
    ops::{Bound, RangeBounds},
};

use axum::{extract::Query, routing::get, Json, Router};
use num::Bounded;
use rand::{distr::SampleString, Rng};
use rand_distr::{
    uniform::SampleUniform, Alphanumeric, Binomial, BinomialError, Distribution, StandardNormal, StandardUniform,
    Uniform,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    app::AppState,
    error::{AppResult, AsStatusCode, IntoAppResult},
};

pub fn route_random() -> Router<AppState> {
    Router::new()
        .route("/", get(random_display::<(), f64, StandardUniform>))
        .route("/string", get(random_string::<(), String, Alphanumeric>))
        .route("/json", get(randjson))
        .route("/standard", get(random_display::<(), f64, StandardUniform>))
        .route("/standard/int", get(random_display::<(), i64, StandardUniform>))
        .route("/standard/float", get(random_display::<(), f64, StandardUniform>))
        .route("/standard/string", get(random_string::<(), String, StandardUniform>))
        .route("/alphanumeric", get(random_string::<(), String, Alphanumeric>))
        .route("/normal", get(random_display::<(), f64, StandardNormal>))
        .route("/normal/float", get(random_display::<(), f64, StandardNormal>))
        .route("/binomial", get(random_display::<Query<BinomialParameter>, _, Binomial>))
        .route("/binomial/int", get(random_display::<Query<BinomialParameter>, _, Binomial>))
        .route("/uniform", get(random_range::<Query<DistRangeParam<_>>, usize, Uniform<_>>))
        .route("/uniform/int", get(random_range::<Query<DistRangeParam<_>>, i64, Uniform<_>>))
        .route("/uniform/float", get(random_range::<Query<DistRangeParam<_>>, f64, Uniform<_>>))
}

pub trait DistributionParameter<D> {
    type Error;
    fn distribution(&self) -> Result<D, Self::Error>;
}
impl DistributionParameter<StandardUniform> for () {
    type Error = Infallible;
    fn distribution(&self) -> Result<StandardUniform, Self::Error> {
        Ok(StandardUniform)
    }
}
impl DistributionParameter<StandardNormal> for () {
    type Error = Infallible;
    fn distribution(&self) -> Result<StandardNormal, Self::Error> {
        Ok(StandardNormal)
    }
}
impl DistributionParameter<Alphanumeric> for () {
    type Error = Infallible;
    fn distribution(&self) -> Result<Alphanumeric, Self::Error> {
        Ok(Alphanumeric)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BinomialParameter {
    pub n: u64,
    pub p: f64,
}
impl Default for BinomialParameter {
    fn default() -> Self {
        Self { n: 10, p: 0.5 }
    }
}
impl DistributionParameter<Binomial> for Query<BinomialParameter> {
    type Error = BinomialError;
    fn distribution(&self) -> Result<Binomial, Self::Error> {
        Binomial::new(self.n, self.p)
    }
}

#[tracing::instrument]
pub async fn random_display<P, T, D>(param: P) -> AppResult<String, RandomError>
where
    P: DistributionParameter<D> + Debug,
    P::Error: Into<Box<dyn std::error::Error>>,
    T: Display,
    D: Distribution<T>,
{
    let mut rng = rand::rng();
    let distribution = param.distribution().response(RandomError::InvalidDistributionParameter)?;
    Ok(distribution.sample(&mut rng).to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct RandomString {
    pub len: usize,
}
impl Default for RandomString {
    fn default() -> Self {
        Self { len: 32 }
    }
}
#[tracing::instrument]
pub async fn random_string<P, T, D>(param: P, rs: Query<RandomString>) -> AppResult<String, RandomError>
where
    P: DistributionParameter<D> + Debug,
    P::Error: Into<Box<dyn std::error::Error>>,
    T: Display,
    D: SampleString,
{
    let mut rng = rand::rng();
    let distribution = param.distribution().response(RandomError::InvalidDistributionParameter)?;
    Ok(distribution.sample_string(&mut rng, rs.len))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DistRangeParam<T> {
    #[serde(default)]
    pub low: Option<T>,
    #[serde(default)]
    pub high: Option<T>,
    #[serde(default)]
    pub inclusive: bool,
}
impl<T: SampleUniform + PartialOrd + Bounded> DistributionParameter<Uniform<T>> for Query<DistRangeParam<T>> {
    type Error = rand_distr::uniform::Error;
    fn distribution(&self) -> Result<Uniform<T>, Self::Error> {
        let start = match self.start_bound() {
            Bound::Included(s) => s,
            Bound::Excluded(s) => s, // TODO? &(*s + 1), but how to implement for float ?
            Bound::Unbounded => &T::min_value(),
        };
        match self.end_bound() {
            Bound::Included(end) => Uniform::new_inclusive(start, end),
            Bound::Excluded(end) => Uniform::new(start, end),
            Bound::Unbounded => Uniform::new_inclusive(start, T::max_value()),
        }
    }
}
impl<T> RangeBounds<T> for DistRangeParam<T> {
    fn start_bound(&self) -> Bound<&T> {
        self.low.as_ref().map(Bound::Included).unwrap_or(Bound::Unbounded)
    }
    fn end_bound(&self) -> Bound<&T> {
        if self.inclusive {
            self.high.as_ref().map(Bound::Included).unwrap_or(Bound::Unbounded)
        } else {
            self.high.as_ref().map(Bound::Excluded).unwrap_or(Bound::Unbounded)
        }
    }
}
impl<T: Display> Display for DistRangeParam<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { low, high, inclusive } = self;
        write!(
            f,
            "{}{}{}",
            low.as_ref().map(T::to_string).unwrap_or_default(),
            if *inclusive { "..=" } else { ".." },
            high.as_ref().map(T::to_string).unwrap_or_default(),
        )
    }
}
#[tracing::instrument]
pub async fn random_range<P, T, D>(param: P, range: Query<DistRangeParam<T>>) -> AppResult<String, RandomError>
where
    P: DistributionParameter<D> + Debug,
    P::Error: Into<Box<dyn std::error::Error>>,
    T: Display + Debug,
    D: Distribution<T>,
{
    let mut rng = rand::rng();
    let distribution = param.distribution().response(RandomError::InvalidDistributionParameter)?;
    Ok(distribution.sample(&mut rng).to_string())
}

#[tracing::instrument]
pub async fn randjson() -> Json<Value> {
    let (max_size, max_depth) = (10, 3);
    fn recursive_json(max_size: usize, max_depth: i32) -> Value {
        let mut rng = rand::rng();
        let size = rng.random_range(0..max_size);
        if max_depth == 0 || max_size == 0 {
            match rng.random_range(0..4) {
                0 => Value::Null,
                1 => Value::Number(rng.random::<i64>().into()),
                2 => Value::Bool(rng.random::<bool>()),
                3 => Value::String(Alphanumeric.sample_string(&mut rng, size)),
                _ => unreachable!(),
            }
        } else {
            match rng.random_range(0..10) {
                0 => Value::Null,
                1 => Value::Number(rng.random::<i64>().into()),
                2 => Value::Bool(rng.random::<bool>()),
                3 => Value::String(Alphanumeric.sample_string(&mut rng, size)),
                4..7 => Value::Array((0..size).map(|_| recursive_json(max_size, max_depth - 1)).collect()),
                7..10 => Value::Object(
                    (0..size)
                        .map(|_| (Alphanumeric.sample_string(&mut rng, size), recursive_json(max_size, max_depth - 1)))
                        .collect(),
                ),
                _ => unreachable!(),
            }
        }
    }
    Json(recursive_json(max_size, max_depth))
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum RandomError {
    #[error("invalid distribution parameter")]
    InvalidDistributionParameter,
}
impl AsStatusCode for RandomError {}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::{
        app::{
            tests::{call, call_bytes},
            AppRouter,
        },
        error::ErrorResponse,
    };

    use super::*;

    #[tokio::test]
    async fn test_random_display() {
        let int = random_display::<(), i64, StandardUniform>(()).await.unwrap();
        assert!(int.parse::<i64>().is_ok());
    }

    #[tokio::test]
    async fn test_random() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/random/standard").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(String::from_utf8_lossy(res.body()).parse::<f64>().unwrap() >= 0.0);
        assert!(String::from_utf8_lossy(res.body()).parse::<f64>().unwrap() <= 1.0);
    }

    #[tokio::test]
    async fn test_random_string_length() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/random/string").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.body().len(), 32);

        let req = Request::builder().uri("/random/string?len=999").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.body().len(), 999);
    }

    #[tokio::test]
    async fn test_random_uniform() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/random/uniform").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(String::from_utf8_lossy(res.body()).parse::<usize>().is_ok());

        let req = Request::builder().uri("/random/uniform/float?low=0.0&high=1.0").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(String::from_utf8_lossy(res.body()).parse::<f64>().unwrap() >= 0.0);
        assert!(String::from_utf8_lossy(res.body()).parse::<f64>().unwrap() < 1.0);

        let req = Request::builder().uri("/random/uniform?low=10&high=100").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(String::from_utf8_lossy(res.body()).parse::<usize>().unwrap() >= 10);
        assert!(String::from_utf8_lossy(res.body()).parse::<usize>().unwrap() < 100);

        let req = Request::builder().uri("/random/uniform?high=1").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(String::from_utf8_lossy(res.body()).parse::<usize>().unwrap(), 0);

        let req = Request::builder().uri("/random/uniform?high=0&inclusive=true").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(String::from_utf8_lossy(res.body()).parse::<usize>().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_random_uniform_invalid() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/random/uniform?low=100&high=0").body(Body::empty()).unwrap();
        let res = call(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        assert!(matches!(res.body(), &ErrorResponse { error: RandomError::InvalidDistributionParameter }));
    }

    #[tokio::test]
    async fn test_random_json() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/random/json").body(Body::empty()).unwrap();
        let res = call_bytes(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(serde_json::from_slice::<Value>(res.body()).is_ok());
    }
}
