use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MeasuredResponse<Res> {
    response: Res,
    metrics: Metrics,
}
impl<Res> MeasuredResponse<Res> {
    pub fn new(response: Res, timestamp: SystemTime, duration: (Instant, Instant)) -> Self {
        let bytes = 0; // TODO implement
        let metrics = Metrics { bytes, timestamp, duration };
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Metrics {
    bytes: usize,
    timestamp: SystemTime,
    duration: (Instant, Instant),
}
impl Metrics {
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    pub fn latency(&self) -> Duration {
        let (start, end) = self.duration;
        end.duration_since(start)
    }
    pub fn end_timestamp(&self) -> SystemTime {
        self.timestamp + self.latency()
    }
    pub fn start_instant(&self) -> Instant {
        let (start, _) = self.duration;
        start
    }
    pub fn end_instant(&self) -> Instant {
        let (_, end) = self.duration;
        end
    }
}
