use axum::{response::Result, routing::get, Json, Router};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub fn route_random() -> Router<AppState> {
    Router::new()
        .route("/", get(random))
        .route("/int", get(randint))
        .route("/float", get(random))
        .route("/string", get(rands))
        .route("/response", get(random_response))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomResponse {
    pub int: i64,
    pub float: f64,
    pub string: String,
}

#[tracing::instrument]
pub async fn random() -> Result<Json<f64>> {
    Ok(Json(rand::random::<f64>()))
}

#[tracing::instrument]
pub async fn randint() -> Result<Json<i64>> {
    Ok(Json(rand::random()))
}

#[tracing::instrument]
pub async fn rands() -> Result<String> {
    let mut rng = rand::thread_rng();
    let sample = Alphanumeric.sample_string(&mut rng, 32);
    Ok(sample)
}

#[tracing::instrument]
pub async fn random_response() -> Result<Json<RandomResponse>> {
    Ok(Json(RandomResponse { int: randint().await?.0, float: random().await?.0, string: rands().await? }))
}
