use axum::{extract::Path, response::Result, routing::get, Json, Router};
use jiff::{Span, SpanRelativeTo};
use serde::{Deserialize, Serialize};

use crate::app::AppState;

pub fn route_wait() -> Router<AppState> {
    Router::new().route("/{time}", get(wait))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct WaitResponse {
    pub wait: String,
}

#[tracing::instrument]
pub async fn wait(Path(time): Path<String>) -> Result<Json<WaitResponse>> {
    let span: Span = time.parse().unwrap_or_else(|e| todo!("{e}"));
    let duration = span.to_duration(SpanRelativeTo::days_are_24_hours()).unwrap_or_else(|e| todo!("{e}"));
    tokio::time::sleep(duration.try_into().unwrap_or_else(|e| todo!("{e}"))).await;
    let wait = format!("{span:#}");
    Ok(Json(WaitResponse { wait }))
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
