use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    error::{AppResult, ResponseWithError},
    state::State,
};

pub fn route_health() -> axum::Router<State> {
    axum::Router::new()
        .route("/", get(health))
        .route("/rich", get(health_json))
        .route("/heavy", get(health_heavy))
        .route("/disabled", get(disabled))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
}
impl IntoResponse for Health {
    fn into_response(self) -> Response {
        // TODO must be used json! macro?
        let content = json!({ "status": self.status.to_string(), "code": self.status.as_u16() });
        (self.status, Json(content)).into_response()
    }
}

#[tracing::instrument]
pub async fn health() -> String {
    "ok".to_string()
}

#[tracing::instrument]
pub async fn health_json() -> Health {
    Health { status: StatusCode::OK }
}

#[tracing::instrument]
pub async fn health_heavy() -> Health {
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    Health { status: StatusCode::TOO_MANY_REQUESTS }
}

#[tracing::instrument]
pub async fn disabled() -> AppResult<(), Health> {
    Err(ResponseWithError::new(StatusCode::SERVICE_UNAVAILABLE, Health { status: StatusCode::SERVICE_UNAVAILABLE }))?
}
