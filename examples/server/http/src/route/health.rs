use axum::{http::StatusCode, routing::get, Json};
use serde::{Deserialize, Serialize};

use crate::state::State;

pub fn route_health() -> axum::Router<State> {
    axum::Router::new()
        .route("/raw", get(health_raw))
        .route("/json", get(health_json))
        .route("/heavy", get(health_heavy))
        .route("/disabled", get(disabled))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
}

#[tracing::instrument]
pub async fn health_raw() -> String {
    "ok".to_string()
}

#[tracing::instrument]
pub async fn health_json() -> Json<Health> {
    Json(Health { status: StatusCode::OK })
}

#[tracing::instrument]
pub async fn health_heavy() -> Json<Health> {
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    Json(Health { status: StatusCode::TOO_MANY_REQUESTS })
}

#[tracing::instrument]
pub async fn disabled() -> Result<(), Json<Health>> {
    Err(Json(Health { status: StatusCode::SERVICE_UNAVAILABLE }))
}
