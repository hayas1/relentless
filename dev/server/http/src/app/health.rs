use axum::{
    http::StatusCode,
    response::{IntoResponse, Response, Result},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::app::AppState;

pub fn route_health() -> Router<AppState> {
    Router::new()
        .route("/", get(health))
        .route("/rich", get(health_rich))
        .route("/heavy", get(health_heavy))
        .route("/disabled", get(disabled))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize)]
pub struct Health {
    #[serde(flatten, with = "health_response")]
    pub status: StatusCode,
}
impl IntoResponse for Health {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}
mod health_response {
    use super::*;
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
pub async fn disabled() -> Health {
    Health { status: StatusCode::SERVICE_UNAVAILABLE }
}

#[cfg(test)]
mod tests {

    use axum::{body::Body, http::Request};

    use crate::app::{
        tests::{call2, call_bytes2},
        AppRouter,
    };

    use super::*;

    #[tokio::test]
    async fn test_health() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
        let res = call_bytes2(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(&**res.body(), b"ok");
    }

    #[tokio::test]
    async fn test_healthz() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/healthz").body(Body::empty()).unwrap();
        let res = call_bytes2(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(&**res.body(), b"ok");
    }

    #[tokio::test]
    async fn test_health_rich() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/health/rich").body(Body::empty()).unwrap();
        let res = call2(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(&Health { status: StatusCode::OK }, res.body());
    }

    #[tokio::test]
    async fn test_health_heavy() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/health/heavy").body(Body::empty()).unwrap();
        let res = call2(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(&Health { status: StatusCode::TOO_MANY_REQUESTS }, res.body());
    }

    #[tokio::test]
    async fn test_health_disabled() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/health/disabled").body(Body::empty()).unwrap();
        let res = call2(&mut service, req).await.unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(&Health { status: StatusCode::SERVICE_UNAVAILABLE }, res.body());
    }
}
