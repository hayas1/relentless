use std::{fmt::Display, marker::PhantomData, time::Duration};

use axum::{extract::Path, response::Result, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::state::AppState;

pub fn route_wait() -> Router<AppState> {
    Router::new()
        .route("/:duration", get(wait::<unit::Seconds>))
        .route("/:duration/s", get(wait::<unit::Seconds>))
        .route("/:duration/ms", get(wait::<unit::Milliseconds>))
        .route("/:duration/ns", get(wait::<unit::Nanoseconds>))
    // .fallback() // TODO
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitResponse<U> {
    pub duration: u64,
    pub unit: PhantomData<U>,
}

pub mod unit {
    use super::*;

    pub trait DurationUnit: Display {
        fn duration(duration: u64) -> Duration;
    }

    pub enum Seconds {}
    impl DurationUnit for Seconds {
        fn duration(duration: u64) -> Duration {
            Duration::from_secs(duration)
        }
    }
    impl Display for Seconds {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "s")
        }
    }

    pub enum Milliseconds {}
    impl DurationUnit for Milliseconds {
        fn duration(duration: u64) -> Duration {
            Duration::from_millis(duration)
        }
    }
    impl Display for Milliseconds {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "ms")
        }
    }

    pub enum Nanoseconds {}
    impl DurationUnit for Nanoseconds {
        fn duration(duration: u64) -> Duration {
            Duration::from_nanos(duration)
        }
    }
    impl Display for Nanoseconds {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "ns")
        }
    }
}

#[tracing::instrument]
pub async fn wait<U: unit::DurationUnit>(Path(duration): Path<u64>) -> Result<Json<WaitResponse<U>>> {
    sleep(U::duration(duration)).await;
    Ok(Json(WaitResponse { duration, unit: PhantomData }))
}
