use axum::{extract::Path, response::Result, routing::get, Json, Router};
use rand::{
    distributions::{Alphanumeric, DistString, Distribution, Standard},
    Rng,
};
use rand_distr::{Binomial, StandardNormal};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{kind::BadRequest, random::RandomError, AppErrorDetail},
    state::AppState,
};

pub fn route_random() -> Router<AppState> {
    Router::new()
        .route("/:distribution", get(rand))
        .route("/:distribution/int", get(randint))
        .route("/:distribution/float", get(rand))
        .route("/:distribution/string", get(rands))
        .route("/:distribution/response", get(random_response))
        .route("/json", get(randjson))
    // .fallback() // TODO
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum DistributionType {
    Standard,
    Alphanumeric,
    Normal,
    Binomial,
}
impl DistributionType {
    pub fn sample_i64(&self, rng: &mut impl Rng) -> Result<i64, RandomError> {
        match self {
            DistributionType::Standard => Ok(Standard.sample(rng)),
            DistributionType::Alphanumeric => {
                Err(RandomError::UnsupportedDistribution("i64".to_string(), self.clone()))
            }
            DistributionType::Normal => Err(RandomError::UnsupportedDistribution("i64".to_string(), self.clone())),
            DistributionType::Binomial => Ok(Binomial::new(10, 0.5).unwrap().sample(rng) as i64),
        }
    }
    pub fn sample_f64(&self, rng: &mut impl Rng) -> Result<f64, RandomError> {
        match self {
            DistributionType::Standard => Ok(Standard.sample(rng)),
            DistributionType::Alphanumeric => {
                Err(RandomError::UnsupportedDistribution("f64".to_string(), self.clone()))
            }
            DistributionType::Normal => Ok(StandardNormal.sample(rng)),
            DistributionType::Binomial => Ok(Binomial::new(10, 0.5).unwrap().sample(rng) as f64),
        }
    }
    pub fn sample_string(&self, rng: &mut impl Rng, len: usize) -> Result<String, RandomError> {
        match self {
            DistributionType::Standard => Ok(Standard.sample_string(rng, len)),
            DistributionType::Alphanumeric => Ok(Alphanumeric.sample_string(rng, len)),
            DistributionType::Normal => Err(RandomError::UnsupportedDistribution("string".to_string(), self.clone())),
            DistributionType::Binomial => Err(RandomError::UnsupportedDistribution("string".to_string(), self.clone())),
        }
    }
}

#[tracing::instrument]
pub async fn rand(Path(distribution): Path<DistributionType>) -> Result<String> {
    let mut rng = rand::thread_rng();
    Ok(distribution.sample_f64(&mut rng).map_err(AppErrorDetail::<BadRequest, _>::detail_display)?.to_string())
}

#[tracing::instrument]
pub async fn randint(Path(distribution): Path<DistributionType>) -> Result<String> {
    let mut rng = rand::thread_rng();
    Ok(distribution.sample_i64(&mut rng).map_err(AppErrorDetail::<BadRequest, _>::detail_display)?.to_string())
}

#[tracing::instrument]
pub async fn rands(Path(distribution): Path<DistributionType>) -> Result<String> {
    let mut rng = rand::thread_rng();
    Ok(distribution.sample_string(&mut rng, 32).map_err(AppErrorDetail::<BadRequest, _>::detail_display)?)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomResponse {
    pub int: i64,
    pub float: f64,
    pub string: String,
}

#[tracing::instrument]
pub async fn random_response(Path(distribution): Path<DistributionType>) -> Result<Json<RandomResponse>> {
    let mut rng = rand::thread_rng();
    Ok(Json(RandomResponse {
        int: distribution.sample_i64(&mut rng).map_err(AppErrorDetail::<BadRequest, _>::detail_display)?,
        float: distribution.sample_f64(&mut rng).map_err(AppErrorDetail::<BadRequest, _>::detail_display)?,
        string: distribution.sample_string(&mut rng, 32).map_err(AppErrorDetail::<BadRequest, _>::detail_display)?,
    }))
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
        error::{kind::Kind, ErrorResponseInner},
        route::{
            app_with,
            tests::{call_bytes, call_with_assert},
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_random_handler() {
        let int = randint(Path(DistributionType::Standard)).await.unwrap();

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
    async fn test_random_unsupported() {
        let mut app = app_with(Default::default());

        call_with_assert(
            &mut app,
            Request::builder().uri("/random/alphanumeric/int").body(Body::empty()).unwrap(),
            StatusCode::BAD_REQUEST,
            ErrorResponseInner {
                msg: BadRequest::msg().to_string(),
                detail: RandomError::UnsupportedDistribution("i64".to_string(), DistributionType::Alphanumeric)
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
        assert!(!body.is_empty());
    }
}
