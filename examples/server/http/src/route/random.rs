use axum::{response::Result, routing::get, Json, Router};
use rand::distributions::{Alphanumeric, DistString};

use crate::state::AppState;

pub fn route_random() -> Router<AppState> {
    Router::new()
        .route("/", get(random))
        .route("/int", get(randint))
        .route("/float", get(random))
        .route("/string", get(rands))
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
