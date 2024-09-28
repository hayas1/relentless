use std::{fmt::Display, future::Future, marker::PhantomData, pin::Pin, sync::Arc, time::Duration};

use axum::{extract::Path, handler::Handler, response::Result, routing::get, Json, Router};
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
    // TODO
    // pub fn duration(self, duration: u64) -> Box<dyn Handler<(), axum::body::Body> + Clone> {
    //     // -> impl Fn(Path<u64>) -> Pin<Box<dyn Future<Output = Result<Json<WaitResponse>>>>> {
    //     Box::new(|Path(unit): Path<u64>| {
    //         async {
    //             // let unit = match &self {
    //             //     DurationUnit::Seconds => Duration::from_secs(duration),
    //             //     DurationUnit::Milliseconds => Duration::from_millis(duration),
    //             //     DurationUnit::Nanoseconds => Duration::from_nanos(duration),
    //             // };
    //             // sleep(unit).await;
    //             // Ok(Json(WaitResponse { duration, unit: self }))
    //             todo!()
    //         }
    //     })
    // }

    pub async fn handle(self, duration: u64) -> Result<Json<WaitResponse>> {
        match self {
            DurationUnit::Seconds => sleep(Duration::from_secs(duration)).await,
            DurationUnit::Milliseconds => sleep(Duration::from_millis(duration)).await,
            DurationUnit::Nanoseconds => sleep(Duration::from_nanos(duration)).await,
        };
        Ok(Json(WaitResponse { duration, unit: self }))
    }
}
