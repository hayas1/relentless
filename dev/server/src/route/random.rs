use std::{future::Future, pin::Pin};

use axum::{
    body::Body,
    extract::Request,
    handler::Handler,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rand::{
    distributions::{Alphanumeric, DistString, Distribution, Standard},
    Rng,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::AppState;

pub fn route_random() -> Router<AppState> {
    Router::new()
        .nest("/", route_random_distribute::<f64>())
        .nest("/int", route_random_distribute::<i64>())
        .nest("/float", route_random_distribute::<f64>())
        .nest("/string", route_random_distribute_string())
    // .nest("/response", route_random_distribute::<RandomResponse>())
    // .nest("/json", route_random_distribute::<Value>())
}
pub fn route_random_distribute<T>() -> Router<AppState>
where
    Standard: Distribution<T> + Clone,
    T: Serialize + 'static,
{
    Router::new().route("/", get(standard::<T>)).route("/standard", get(standard::<T>))
}
pub fn route_random_distribute_string() -> Router<AppState> {
    Router::new().route("/", get(alphanumeric)).route("/standard", get(standard_string))
}

#[derive(Debug, Clone)]
pub struct StandardHandler;
impl<T> Handler<T, AppState> for StandardHandler
where
    Standard: Distribution<T>,
    T: Serialize,
{
    type Future = Pin<Box<dyn Future<Output = Response<Body>> + Send>>;
    fn call(self, _req: Request, _state: AppState) -> Self::Future {
        Box::pin(async move {
            let mut rng = rand::thread_rng();
            Json(<Standard as Distribution<T>>::sample(&Standard, &mut rng)).into_response()
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
pub async fn standard<T>() -> Json<T>
where
    Standard: Distribution<T>,
    T: Serialize,
{
    let mut rng = rand::thread_rng();
    Json(<Standard as Distribution<T>>::sample(&Standard, &mut rng))
}

#[tracing::instrument]
pub async fn standard_string() -> String {
    let mut rng = rand::thread_rng();
    Standard.sample_string(&mut rng, 10)
}

#[tracing::instrument]
pub async fn alphanumeric() -> String {
    let mut rng = rand::thread_rng();
    Alphanumeric.sample_string(&mut rng, 10)
}

#[tracing::instrument]
pub async fn random_response() -> Json<RandomResponse> {
    Json(RandomResponse {
        int: rand::random(),
        float: rand::random(),
        string: Alphanumeric.sample_string(&mut rand::thread_rng(), 32),
    })
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
