use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeasuredResponse<Res> {
    response: Res,
    metrics: Metrics,
}
impl<Res> MeasuredResponse<Res> {
    pub fn new(response: Res, timestamp: SystemTime, latency: Duration) -> Self {
        let bytes = 0; // TODO implement
        let metrics = Metrics { timestamp, latency, bytes };
        Self { response, metrics }
    }

    pub fn response(&self) -> &Res {
        &self.response
    }
    pub fn into_response(self) -> Res {
        self.response
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }
    pub fn into_metrics(self) -> Metrics {
        self.metrics
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Metrics {
    timestamp: SystemTime,
    latency: Duration,
    bytes: usize,
}
impl Metrics {
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
