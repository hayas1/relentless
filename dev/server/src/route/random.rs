use axum::{routing::get, Json, Router};
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
        .route("/json", get(randjson))
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

#[tracing::instrument]
pub async fn randjson() -> Json<Value> {
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
    Json(recursive_json(max_size, max_depth))
}
