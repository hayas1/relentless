use std::time::{Duration, SystemTime, SystemTimeError};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type RequestResult<Res> = Result<MeasuredResponse<Res>, RequestError>;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeasuredResponse<Res> {
    response: Res,
    timestamp: SystemTime,
    latency: Duration,
}
impl<Res> MeasuredResponse<Res> {
    pub fn new(response: Res, timestamp: SystemTime, latency: Duration) -> Self {
        Self { response, timestamp, latency }
    }

    pub fn response(&self) -> &Res {
        &self.response
    }
    pub fn into_response(self) -> Res {
        self.response
    }

    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    pub fn end_timestamp(&self) -> SystemTime {
        self.timestamp + self.latency
    }
    pub fn latency(&self) -> Duration {
        self.latency
    }
}

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
