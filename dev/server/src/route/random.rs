use axum::{response::Result, routing::get, Json, Router};
use rand::{
    distributions::{Alphanumeric, DistString},
    Rng,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
pub async fn random() -> Json<f64> {
    Json(rand::random::<f64>())
}

#[tracing::instrument]
pub async fn randint() -> Json<i64> {
    Json(rand::random())
}

#[tracing::instrument]
pub async fn rands() -> String {
    let mut rng = rand::thread_rng();
    Alphanumeric.sample_string(&mut rng, 32)
}

#[tracing::instrument]
pub async fn random_response() -> Json<RandomResponse> {
    Json(RandomResponse { int: randint().await.0, float: random().await.0, string: rands().await })
}
