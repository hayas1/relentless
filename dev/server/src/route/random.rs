use axum::{extract::Path, response::Result, routing::get, Json, Router};
use rand::{
    distributions::{Alphanumeric, DistString, Distribution, Standard},
    Rng,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::{kind::BadRequest, random::RandomError, AppError},
    state::AppState,
};

pub fn route_random() -> Router<AppState> {
    Router::new()
        .route("/:distribution", get(rand))
        .route("/:distribution/int", get(randint))
        .route("/:distribution/float", get(rand))
        .route("/:distribution/string", get(rands))
        .route("/:distribution/response", get(random_response))
        .route("/:distribution/json", get(randjson))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum DistributionType {
    Standard,
    Alphanumeric,
}
impl DistributionType {
    pub fn sample_i64(&self, rng: &mut impl Rng) -> Result<i64, RandomError> {
        match self {
            DistributionType::Standard => Ok(Standard.sample(rng)),
            DistributionType::Alphanumeric => {
                Err(RandomError::UnsupportedDistribution("i64".to_string(), self.clone()))
            }
        }
    }
    pub fn sample_f64(&self, rng: &mut impl Rng) -> Result<f64, RandomError> {
        match self {
            DistributionType::Standard => Ok(Standard.sample(rng)),
            DistributionType::Alphanumeric => {
                Err(RandomError::UnsupportedDistribution("f64".to_string(), self.clone()))
            }
        }
    }
    pub fn sample_string(&self, rng: &mut impl Rng, len: usize) -> Result<String, RandomError> {
        match self {
            DistributionType::Standard => Ok(Standard.sample_string(rng, len)),
            DistributionType::Alphanumeric => Ok(Alphanumeric.sample_string(rng, len)),
        }
    }
}

#[tracing::instrument]
pub async fn rand(Path(distribution): Path<DistributionType>) -> Result<String> {
    let mut rng = rand::thread_rng();
    Ok(distribution.sample_f64(&mut rng).map_err(AppError::<BadRequest>::wrap)?.to_string())
}

#[tracing::instrument]
pub async fn randint(Path(distribution): Path<DistributionType>) -> Result<String> {
    let mut rng = rand::thread_rng();
    Ok(distribution.sample_i64(&mut rng).map_err(AppError::<BadRequest>::wrap)?.to_string())
}

#[tracing::instrument]
pub async fn rands(Path(distribution): Path<DistributionType>) -> Result<String> {
    let mut rng = rand::thread_rng();
    Ok(distribution.sample_string(&mut rng, 32).map_err(AppError::<BadRequest>::wrap)?)
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
        int: distribution.sample_i64(&mut rng).map_err(AppError::<BadRequest>::wrap)?,
        float: distribution.sample_f64(&mut rng).map_err(AppError::<BadRequest>::wrap)?,
        string: distribution.sample_string(&mut rng, 32).map_err(AppError::<BadRequest>::wrap)?,
    }))
}

#[tracing::instrument]
pub async fn randjson(Path(distribution): Path<DistributionType>) -> Result<Json<Value>> {
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
