use std::{fmt::Display, future::Future, pin::Pin};

use axum::{response::Result, routing::get, Json, Router};
use rand::{
    distributions::{Alphanumeric, DistString, Distribution, Standard},
    Rng,
};
use rand_distr::{Binomial, StandardNormal};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::AppState;

pub fn route_random() -> Router<AppState> {
    Router::new()
        .route("/", get(random_handler::<f64, _>(Standard)))
        .route("/string", get(random_string_handler(Alphanumeric)))
        .route("/response", get(random_response_handler(Standard, Standard, Alphanumeric)))
        .route("/json", get(randjson))
        .route("/standard", get(random_handler::<f64, _>(Standard)))
        .route("/standard/int", get(random_handler::<i64, _>(Standard)))
        .route("/standard/float", get(random_handler::<f64, _>(Standard)))
        .route("/standard/string", get(random_string_handler(Standard)))
        .route("/standard/response", get(random_response_handler(Standard, Standard, Standard)))
        .route("/normal", get(random_handler::<f64, _>(StandardNormal)))
        .route("/normal/float", get(random_handler::<f64, _>(StandardNormal)))
        .route("/binomial", get(random_handler::<u64, _>(Binomial::new(10, 0.5).unwrap())))
        .route("/binomial/int", get(random_handler::<u64, _>(Binomial::new(10, 0.5).unwrap())))
    // .fallback() // TODO
}

pub fn random_handler<T, D>(
    distribution: D,
) -> impl FnOnce() -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Clone
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

pub fn random_string_handler<D>(
    distribution: D,
) -> impl FnOnce() -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Clone
where
    D: DistString + Clone + Send + 'static,
{
    move || {
        Box::pin(async move {
            let mut rng = rand::thread_rng();
            Ok(distribution.sample_string(&mut rng, 32))
        })
    }
}

pub fn random_response_handler<DI, DF, DS>(
    int_distribution: DI,
    float_distribution: DF,
    distribution_string: DS,
) -> impl FnOnce() -> Pin<Box<dyn Future<Output = Result<Json<RandomResponse>>> + Send>> + Clone
where
    DI: Distribution<i64> + Clone + Send + 'static,
    DF: Distribution<f64> + Clone + Send + 'static,
    DS: DistString + Clone + Send + 'static,
{
    move || {
        Box::pin(async move {
            let mut rng = rand::thread_rng();
            Ok(Json(RandomResponse {
                int: int_distribution.sample(&mut rng),
                float: float_distribution.sample(&mut rng),
                string: distribution_string.sample_string(&mut rng, 32),
            }))
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomResponse {
    pub int: i64,
    pub float: f64,
    pub string: String,
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

// #[cfg(test)]
// mod tests {
//     use axum::{
//         body::Body,
//         http::{Request, StatusCode},
//     };

//     use crate::{
//         error::{kind::Kind, ErrorResponseInner},
//         route::{
//             app_with,
//             tests::{call_bytes, call_with_assert},
//         },
//     };

//     use super::*;

//     #[tokio::test]
//     async fn test_random_handler() {
//         let int = randint(Path(DistributionType::Standard)).await.unwrap();

//         assert!(int.parse::<i64>().is_ok());
//     }

//     #[tokio::test]
//     async fn test_random() {
//         let mut app = app_with(Default::default());

//         let (status, body) =
//             call_bytes(&mut app, Request::builder().uri("/random/standard").body(Body::empty()).unwrap()).await;
//         assert_eq!(status, StatusCode::OK);
//         assert!(String::from_utf8_lossy(&body[..]).parse::<f64>().unwrap() >= 0.0);
//         assert!(String::from_utf8_lossy(&body[..]).parse::<f64>().unwrap() <= 1.0);
//     }

//     #[tokio::test]
//     async fn test_random_unsupported() {
//         let mut app = app_with(Default::default());

//         call_with_assert(
//             &mut app,
//             Request::builder().uri("/random/alphanumeric/int").body(Body::empty()).unwrap(),
//             StatusCode::BAD_REQUEST,
//             ErrorResponseInner {
//                 msg: BadRequest::msg().to_string(),
//                 detail: RandomError::UnsupportedDistribution("i64".to_string(), DistributionType::Alphanumeric)
//                     .to_string(),
//             },
//         )
//         .await;
//     }

//     #[tokio::test]
//     async fn test_random_json() {
//         let mut app = app_with(Default::default());

//         let (status, body) =
//             call_bytes(&mut app, Request::builder().uri("/random/json").body(Body::empty()).unwrap()).await;
//         assert_eq!(status, StatusCode::OK);
//         assert!(!body.is_empty());
//     }
// }
