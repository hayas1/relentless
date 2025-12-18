use axum::{extract::Path, routing::get, Json, Router};
use jiff::Span;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    app::AppState,
    error2::{AppResult, AsStatusCode, IntoAppResult},
};

pub fn route_wait() -> Router<AppState> {
    Router::new().route("/{time}", get(wait))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct WaitResponse {
    pub wait: String,
}

#[tracing::instrument]
pub async fn wait(Path(time): Path<String>) -> AppResult<Json<WaitResponse>, WaitError> {
    let span: Span = time.parse().response(WaitError::InvalidTime(time))?;
    let duration = span.try_into().response(WaitError::CannotWait(span))?;
    tokio::time::sleep(duration).await;
    let wait = format!("{span:#}");
    Ok(Json(WaitResponse { wait }))
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum WaitError {
    #[error("invalid time: {0}")]
    InvalidTime(String),

    #[error("cannot wait: {0:#}")]
    CannotWait(Span),
}
impl AsStatusCode for WaitError {}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::{
        app::{tests::call2, AppRouter},
        error2::ErrorResponse,
    };

    use super::*;

    #[tokio::test]
    async fn test_wait() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/wait/500ms").body(Body::empty()).unwrap();
        let now = Instant::now();
        let res = call2(&mut service, req).await.unwrap();
        assert!(now.elapsed() >= Duration::from_millis(500));
        assert_eq!(StatusCode::OK, res.status());
        assert_eq!(&WaitResponse { wait: "500ms".to_string() }, res.body());
    }

    #[tokio::test]
    async fn test_wait_error() {
        let mut service = AppRouter::default().service();

        let req = Request::builder().uri("/wait/-500ms").body(Body::empty()).unwrap();
        let res = call2(&mut service, req).await.unwrap();
        assert_eq!(StatusCode::BAD_REQUEST, res.status());
        assert!(
            matches!(res.body(), &ErrorResponse { error: WaitError::CannotWait(s) } if s == Span::try_from(Duration::from_millis(500)).unwrap().fieldwise())
        );
    }
}
