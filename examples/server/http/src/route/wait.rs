use std::time::Duration;

use axum::{extract::Path, response::Result, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::state::AppState;

pub fn route_wait() -> Router<AppState> {
    Router::new()
        .route("/:duration", get(|Path(d): Path<u64>| async move { DurationUnit::Seconds.handle(d).await }))
        .route("/:duration/s", get(|Path(d): Path<u64>| async move { DurationUnit::Seconds.handle(d).await }))
        .route("/:duration/ms", get(|Path(d): Path<u64>| async move { DurationUnit::Milliseconds.handle(d).await }))
        .route("/:duration/ns", get(|Path(d): Path<u64>| async move { DurationUnit::Nanoseconds.handle(d).await }))
    // .fallback() // TODO
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitResponse {
    pub duration: u64,
    pub unit: DurationUnit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurationUnit {
    Seconds,
    Milliseconds,
    Nanoseconds,
}
impl DurationUnit {
    pub async fn handle(self, duration: u64) -> Result<Json<WaitResponse>> {
        match self {
            DurationUnit::Seconds => sleep(Duration::from_secs(duration)).await,
            DurationUnit::Milliseconds => sleep(Duration::from_millis(duration)).await,
            DurationUnit::Nanoseconds => sleep(Duration::from_nanos(duration)).await,
        };
        Ok(Json(WaitResponse { duration, unit: self }))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };

    use crate::route::{app_with, tests::call_with_assert};

    use super::*;

    #[tokio::test]
    async fn test() {
        let mut app = app_with(Default::default());

        let now = Instant::now();
        call_with_assert(
            &mut app,
            Request::builder().uri("/wait/500/ms").body(Body::empty()).unwrap(),
            StatusCode::OK,
            WaitResponse { duration: 500, unit: DurationUnit::Milliseconds },
        )
        .await;
        assert!(now.elapsed() >= Duration::from_millis(500));
    }
}
