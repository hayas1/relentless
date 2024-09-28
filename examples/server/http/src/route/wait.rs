use std::{future::Future, pin::Pin, time::Duration};

use axum::{extract::Path, response::Result, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::state::AppState;

pub fn route_wait() -> Router<AppState> {
    Router::new()
        .route("/:duration", get(DurationUnit::Seconds.handler()))
        .route("/:duration/s", get(DurationUnit::Seconds.handler()))
        .route("/:duration/ms", get(DurationUnit::Milliseconds.handler()))
        .route("/:duration/ns", get(DurationUnit::Nanoseconds.handler()))
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
    pub fn handler(
        self,
    ) -> impl FnOnce(Path<u64>) -> Pin<Box<dyn Future<Output = Result<Json<WaitResponse>>> + Send>> + Clone {
        // return type ref: https://github.com/tokio-rs/axum/pull/1082/files#diff-93eb961c85da77636607a224513f085faf7876f5a9f7091c13e05939aa5de33cR61-R62
        move |Path(duration): Path<u64>| {
            let d = match self {
                DurationUnit::Seconds => Duration::from_secs(duration),
                DurationUnit::Milliseconds => Duration::from_millis(duration),
                DurationUnit::Nanoseconds => Duration::from_nanos(duration),
            };
            let unit = self.clone();
            Box::pin(async move {
                sleep(d).await;
                Ok(Json(WaitResponse { duration, unit }))
            })
        }
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
