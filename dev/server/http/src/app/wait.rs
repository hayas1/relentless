use axum::{extract::Path, routing::get, Json, Router};
use jiff::{SignedDuration, Span, SpanRelativeTo};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    app::AppState,
    error2::{AppResult, IntoAppResult},
};

pub fn route_wait() -> Router<AppState> {
    Router::new().route("/{time}", get(wait))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct WaitResponse {
    pub wait: String,
}

#[tracing::instrument]
pub async fn wait(Path(time): Path<String>) -> AppResult<Json<WaitResponse>> {
    let span: Span = time.parse().response(WaitError::InvalidTime(time))?;
    let duration = span.to_duration(SpanRelativeTo::days_are_24_hours()).response(WaitError::InvalidDuration(span))?;
    let sleep = duration.try_into().response(WaitError::CannotWait(duration))?;
    tokio::time::sleep(sleep).await;
    let wait = format!("{span:#}");
    Ok(Json(WaitResponse { wait }))
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum WaitError {
    #[error("invalid time: {0}")]
    InvalidTime(String),

    #[error("invalid duration: {0}")]
    InvalidDuration(Span),

    #[error("cannot wait: {0}")]
    CannotWait(SignedDuration),
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::app::{tests::call_with_assert, AppRouter};

    use super::*;

    #[tokio::test]
    async fn test_wait() {
        let mut service = AppRouter::default().service();

        let now = Instant::now();
        call_with_assert(
            &mut service,
            Request::builder().uri("/wait/500ms").body(Body::empty()).unwrap(),
            StatusCode::OK,
            WaitResponse { wait: "500ms".to_string() },
        )
        .await;
        assert!(now.elapsed() >= Duration::from_millis(500));
    }
}
