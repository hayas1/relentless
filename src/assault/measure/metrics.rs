use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

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
