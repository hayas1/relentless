use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppResult, ResponseWithError},
    state::AppState,
};

pub fn route_health() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", get(health))
        .route("/rich", get(health_rich))
        .route("/heavy", get(health_heavy))
        .route("/disabled", get(disabled))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    #[serde(flatten, with = "health_response")]
    status: StatusCode,
}
impl IntoResponse for Health {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
mod health_response {
    use super::*;
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct HealthResponse {
        status: String,
        code: u16,
    }
    pub fn serialize<S>(value: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let (status, code) = (value.to_string(), value.as_u16());
        HealthResponse { status, code }.serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let HealthResponse { code, .. } = HealthResponse::deserialize(deserializer)?;
        Ok(StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
    }
}

#[tracing::instrument]
pub async fn health() -> String {
    "ok".to_string()
}

#[tracing::instrument]
pub async fn health_rich() -> Health {
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

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{HeaderMap, StatusCode},
    };

    use super::*;
    use crate::tests::{send, send_bytes};

    #[tokio::test]
    async fn test_health_call() {
        let (uri, body, headers) = ("/health", Body::empty(), HeaderMap::new());
        let (status, body) = send_bytes(uri, body, headers).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn test_health_rich_call() {
        let (uri, body, headers) = ("/health/rich", Body::empty(), HeaderMap::new());
        let (status, health) = send::<Health>(uri, body, headers).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(health, Health { status: StatusCode::OK });
    }

    #[tokio::test]
    async fn test_disabled_call() {
        let (uri, body, headers) = ("/health/disabled", Body::empty(), HeaderMap::new());
        let (status, health) = send::<Health>(uri, body, headers).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(health, Health { status: StatusCode::SERVICE_UNAVAILABLE });
    }
}
