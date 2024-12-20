use std::{
    fmt::{Debug, Display},
    ops::{Bound, RangeBounds},
};

use axum::{extract::Query, response::Result, routing::get, Json, Router};
use num::Bounded;
use rand::{
    distributions::{DistString, Distribution},
    Rng,
};
use rand_distr::{uniform::SampleUniform, Alphanumeric, Binomial, Standard, StandardNormal, Uniform};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{error::random::RandomError, state::AppState};

use super::PinResponseFuture;

pub fn route_random() -> Router<AppState> {
    Router::new()
        .route("/", get(random_handler::<f64, _>(Standard)))
        .route("/string", get(RandomString::handler(Alphanumeric)))
        .route("/response", get(RandomResponse::handler(Standard, Standard, Alphanumeric)))
        .route("/json", get(randjson))
        .route("/standard", get(random_handler::<f64, _>(Standard)))
        .route("/standard/int", get(random_handler::<i64, _>(Standard)))
        .route("/standard/float", get(random_handler::<f64, _>(Standard)))
        .route("/standard/string", get(RandomString::handler(Standard)))
        .route("/standard/response", get(RandomResponse::handler(Standard, Standard, Standard)))
        .route("/alphanumeric", get(RandomString::handler(Alphanumeric)))
        .route("/normal", get(random_handler::<f64, _>(StandardNormal)))
        .route("/normal/float", get(random_handler::<f64, _>(StandardNormal)))
        .route("/binomial", get(random_handler(Binomial::new(10, 0.5).unwrap_or_else(|_| unreachable!()))))
        .route("/binomial/int", get(random_handler(Binomial::new(10, 0.5).unwrap_or_else(|_| unreachable!()))))
        .route("/uniform", get(Uniform::<usize>::handler()))
        .route("/uniform/int", get(Uniform::<i64>::handler()))
        .route("/uniform/float", get(Uniform::<f64>::handler()))
    // .fallback() // TODO
}

pub fn random_handler<T, D>(distribution: D) -> impl FnOnce() -> PinResponseFuture<Result<String>> + Clone
where
    T: Display + Clone + Send + 'static,
    D: Distribution<T> + Clone + Send + 'static,
{
    move || {
        Box::pin(async move {
            let mut rng = rand::thread_rng();
            Ok(distribution.sample(&mut rng).to_string())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RandomString {
    #[serde(default)]
    pub len: Option<usize>,
}
impl RandomString {
    pub fn handler<D>(distribution: D) -> impl FnOnce(Query<RandomString>) -> PinResponseFuture<Result<String>> + Clone
    where
        D: DistString + Clone + Send + 'static,
    {
        move |Query(rs): Query<RandomString>| {
            Box::pin(async move {
                let mut rng = rand::thread_rng();
                Ok(distribution.sample_string(&mut rng, rs.length()))
            })
        }
    }

    pub fn length(&self) -> usize {
        self.len.unwrap_or(32)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomResponse {
    pub int: i64,
    pub float: f64,
    pub string: String,
}
impl RandomResponse {
    pub fn handler<DI, DF, DS>(
        int_distribution: DI,
        float_distribution: DF,
        distribution_string: DS,
    ) -> impl FnOnce(Query<RandomString>) -> PinResponseFuture<Result<Json<Self>>> + Clone
    where
        DI: Distribution<i64> + Clone + Send + 'static,
        DF: Distribution<f64> + Clone + Send + 'static,
        DS: DistString + Clone + Send + 'static,
    {
        move |Query(rs): Query<RandomString>| {
            Box::pin(async move {
                let mut rng = rand::thread_rng();
                Ok(Json(RandomResponse {
                    int: int_distribution.sample(&mut rng),
                    float: float_distribution.sample(&mut rng),
                    string: distribution_string.sample_string(&mut rng, rs.length()),
                }))
            })
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DistRangeParam<T> {
    #[serde(default)]
    pub low: Option<T>,
    #[serde(default)]
    pub high: Option<T>,
    #[serde(default)]
    pub inclusive: bool,
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
pub trait DistRange<T>: Distribution<T> {
    fn new<R>(range: &R) -> Option<Self>
    where
        R: RangeBounds<T>,
        Self: Sized;

    fn handler() -> impl FnOnce(Query<DistRangeParam<T>>) -> PinResponseFuture<Result<String>> + Clone
    where
        Self: DistRange<T> + Sized,
        T: Display + Debug + Clone + Send + Sync + 'static,
    {
        move |Query(r): Query<DistRangeParam<T>>| {
            Box::pin(async move {
                let mut rng = rand::thread_rng();
                let dist = Self::new(&r).ok_or_else(|| RandomError::EmptyRange(r))?;
                Ok(dist.sample(&mut rng).to_string())
            })
        }
    }
}
impl<T> DistRange<T> for Uniform<T>
where
    T: SampleUniform + PartialOrd + Bounded,
{
    fn new<R: RangeBounds<T>>(range: &R) -> Option<Self> {
        let start = match range.start_bound() {
            Bound::Included(s) => s,
            Bound::Excluded(s) => s, // TODO? &(*s + 1),
            Bound::Unbounded => &T::min_value(),
        };
        match range.end_bound() {
            Bound::Included(end) => (start <= end).then(|| Uniform::new_inclusive(start, end)),
            Bound::Excluded(end) => (start < end).then(|| Uniform::new(start, end)),
            Bound::Unbounded => (start <= &T::max_value()).then(|| Uniform::new_inclusive(start, T::max_value())), // TODO float max cause panic and empty reply
        }
    }
}

#[tracing::instrument]
pub async fn randjson() -> Result<Json<Value>> {
    let (max_size, max_depth) = (10, 3);
    fn recursive_json(max_size: usize, max_depth: i32) -> Value {
        let mut rng = rand::thread_rng();
        let size = rng.gen_range(0..max_size);
        if max_depth == 0 || max_size == 0 {
            match rng.gen_range(0..4) {
                0 => Value::Null,
                1 => Value::Number(rng.gen::<i64>().into()),
                2 => Value::Bool(rng.gen::<bool>()),
                3 => Value::String(Alphanumeric.sample_string(&mut rng, size)),
                _ => unreachable!(),
            }
        } else {
            match rng.gen_range(0..6) {
                0 => Value::Null,
                1 => Value::Number(rng.gen::<i64>().into()),
                2 => Value::Bool(rng.gen::<bool>()),
                3 => Value::String(Alphanumeric.sample_string(&mut rng, size)),
                4 => Value::Array((0..size).map(|_| recursive_json(max_size, max_depth - 1)).collect()),
                5 => Value::Object(
                    (0..size)
                        .map(|_| (Alphanumeric.sample_string(&mut rng, size), recursive_json(max_size, max_depth - 1)))
                        .collect(),
                ),
                _ => unreachable!(),
            }
        }
    }
    Ok(Json(recursive_json(max_size, max_depth)))
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
        route::{
            app_with,
            tests::{call_bytes, call_with_assert},
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_random_handler() {
        let int = random_handler::<i64, _>(Standard)().await.unwrap();

        assert!(int.parse::<i64>().is_ok());
    }

    #[tokio::test]
    async fn test_random() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/random/standard").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert!(String::from_utf8_lossy(&body[..]).parse::<f64>().unwrap() >= 0.0);
        assert!(String::from_utf8_lossy(&body[..]).parse::<f64>().unwrap() <= 1.0);
    }

    #[tokio::test]
    async fn test_random_string_length() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/random/string").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.len(), 32);

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/random/string?len=999").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.len(), 999);
    }

    #[tokio::test]
    async fn test_random_uniform() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/random/uniform").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert!(String::from_utf8_lossy(&body[..]).parse::<usize>().is_ok());

        let (status, body) = call_bytes(
            &mut app,
            Request::builder().uri("/random/uniform/float?low=0.0&high=1.0").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(String::from_utf8_lossy(&body[..]).parse::<f64>().unwrap() >= 0.0);
        assert!(String::from_utf8_lossy(&body[..]).parse::<f64>().unwrap() < 1.0);

        let (status, body) = call_bytes(
            &mut app,
            Request::builder().uri("/random/uniform?low=10&high=100").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(String::from_utf8_lossy(&body[..]).parse::<usize>().unwrap() >= 10);
        assert!(String::from_utf8_lossy(&body[..]).parse::<usize>().unwrap() < 100);

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/random/uniform?high=1").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(String::from_utf8_lossy(&body[..]).parse::<usize>().unwrap(), 0);

        let (status, body) = call_bytes(
            &mut app,
            Request::builder().uri("/random/uniform?high=0&inclusive=true").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(String::from_utf8_lossy(&body[..]).parse::<usize>().unwrap(), 0);

        call_with_assert(
            &mut app,
            Request::builder().uri("/random/uniform?low=100&high=0").body(Body::empty()).unwrap(),
            APP_DEFAULT_ERROR_CODE,
            ErrorResponseInner {
                msg: BadRequest::msg().to_string(),
                detail: RandomError::EmptyRange(DistRangeParam { low: Some(100), high: Some(0), inclusive: false })
                    .to_string(),
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_random_json() {
        let mut app = app_with(Default::default());

        let (status, body) =
            call_bytes(&mut app, Request::builder().uri("/random/json").body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
        assert!(serde_json::from_slice::<Value>(&body[..]).is_ok());
    }
}
