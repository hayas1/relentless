use std::time::{Duration, SystemTimeError};

use thiserror::Error;

use super::measure::metrics::MeasuredResponse;

pub type RequestResult<Res> = Result<MeasuredResponse<Res>, RequestError>;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("request timeout: {0:?}")]
    Timeout(Duration),

    #[error(transparent)]
    FailToMakeRequest(Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    NoReady(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    InnerServiceError(Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    FailToMeasureLatency(SystemTimeError),
    #[error(transparent)]
    Unknown(Box<dyn std::error::Error + Send + Sync>),
}
