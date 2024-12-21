use std::{
    marker::PhantomData,
    time::{Duration, SystemTime, SystemTimeError},
};

use average::{Estimate, Max, Mean, Min, Quantile};

use super::metrics::MeasuredResponse;

pub trait Aggregator {
    type Add;
    type Aggregate;
    fn add(&mut self, add: Self::Add);
    fn aggregate(&self) -> Self::Aggregate;
}

#[derive(Debug, Clone)]
pub struct ResponseAggregate<Res> {
    count: CountAggregate,
    duration: DurationAggregate,
    bytes: BytesAggregate,
    latency: LatencyAggregate,
    phantom: PhantomData<Res>,
}
impl<Res> Aggregator for ResponseAggregate<Res> {
    type Add = MeasuredResponse<Res>;
    type Aggregate = (
        <CountAggregate as Aggregator>::Aggregate,
        <DurationAggregate as Aggregator>::Aggregate,
        Result<f64, SystemTimeError>,
        <BytesAggregate as Aggregator>::Aggregate,
        <LatencyAggregate as Aggregator>::Aggregate,
    );
    fn add(&mut self, res: Self::Add) {
        self.count.add(());
        self.duration.add(res.timestamp());
        self.bytes.add(());
        self.latency.add(res.latency());
    }
    fn aggregate(&self) -> Self::Aggregate {
        (
            self.count.aggregate(),
            self.duration.aggregate(),
            self.rps(),
            self.bytes.aggregate(),
            self.latency.aggregate(),
        )
    }
}
impl<Res> ResponseAggregate<Res> {
    pub fn new<I: Iterator<Item = f64>>(now: SystemTime, quantile: I) -> Self {
        Self {
            count: CountAggregate::new(),
            duration: DurationAggregate::new(now),
            bytes: BytesAggregate {},
            latency: LatencyAggregate::new(quantile),
            phantom: PhantomData,
        }
    }

    pub fn rps(&self) -> Result<f64, SystemTimeError> {
        Ok(self.count.aggregate() as f64 / self.duration.aggregate()?.as_secs_f64())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CountAggregate {
    count: u64,
}
impl Aggregator for CountAggregate {
    type Add = ();
    type Aggregate = u64;
    fn add(&mut self, (): Self::Add) {
        self.count += 1;
    }
    fn aggregate(&self) -> Self::Aggregate {
        self.count
    }
}
impl CountAggregate {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct PassedAggregate {
    count: CountAggregate,
    passed: CountAggregate,
}
impl Aggregator for PassedAggregate {
    type Add = bool;
    type Aggregate = (u64, u64, f64);
    fn add(&mut self, pass: Self::Add) {
        self.count.add(());
        if pass {
            self.passed.add(());
        }
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.count(), self.passed(), self.ratio())
    }
}
impl PassedAggregate {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn count(&self) -> u64 {
        self.count.aggregate()
    }
    pub fn passed(&self) -> u64 {
        self.passed.aggregate()
    }
    pub fn ratio(&self) -> f64 {
        self.passed() as f64 / self.count() as f64
    }
}

#[derive(Debug, Clone)]
pub struct DurationAggregate {
    first: SystemTime,
    last: SystemTime,
}
impl Aggregator for DurationAggregate {
    type Add = SystemTime;
    type Aggregate = Result<Duration, SystemTimeError>;
    fn add(&mut self, timestamp: Self::Add) {
        self.first = self.first.min(timestamp);
        self.last = self.last.max(timestamp);
    }
    fn aggregate(&self) -> Self::Aggregate {
        self.last.duration_since(self.first)
    }
}
impl DurationAggregate {
    pub fn new(now: SystemTime) -> Self {
        Self { first: now, last: now }
    }
}

#[derive(Debug, Clone)]
pub struct BytesAggregate {
    // TODO implement
}
impl Aggregator for BytesAggregate {
    type Add = ();
    type Aggregate = ();
    fn add(&mut self, _: Self::Add) {}
    fn aggregate(&self) -> Self::Aggregate {}
}

#[derive(Debug, Clone)]
pub struct LatencyAggregate {
    min: Min,
    mean: Mean,
    quantile: Vec<Quantile>,
    max: Max,
}
impl Aggregator for LatencyAggregate {
    type Add = Duration;
    type Aggregate = (Duration, Duration, Vec<Duration>, Duration);
    fn add(&mut self, latency: Self::Add) {
        let nanos = latency.as_secs_f64();
        self.min.add(nanos);
        self.mean.add(nanos);
        self.quantile.iter_mut().for_each(|q| q.add(nanos));
        self.max.add(nanos);
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.min(), self.mean(), self.quantile(), self.max())
    }
}
impl LatencyAggregate {
    pub fn new<I: IntoIterator<Item = f64>>(quantile: I) -> Self {
        Self {
            min: Min::new(),
            mean: Mean::new(),
            quantile: quantile.into_iter().map(Quantile::new).collect(),
            max: Max::new(),
        }
    }

    pub fn min(&self) -> Duration {
        Duration::from_secs_f64(self.min.min())
    }
    pub fn mean(&self) -> Duration {
        Duration::from_secs_f64(self.mean.mean())
    }
    pub fn quantile(&self) -> Vec<Duration> {
        self.quantile.iter().map(|q| Duration::from_secs_f64(q.quantile())).collect()
    }
    pub fn max(&self) -> Duration {
        Duration::from_secs_f64(self.max.max())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_aggregate() {
        let mut agg = CountAggregate::new();
        for _ in 0..1000 {
            agg.add(());
        }
        assert_eq!(agg.aggregate(), 1000);
    }

    #[test]
    fn passed_aggregate() {
        let mut agg = PassedAggregate::new();
        for i in 0..1000 {
            agg.add(i % 2 == 0);
        }
        assert_eq!(agg.aggregate(), (1000, 500, 0.5));
    }

    #[test]
    fn duration_aggregate() {
        let mut agg = DurationAggregate::new(SystemTime::UNIX_EPOCH);
        for i in 0..1000 {
            agg.add(SystemTime::UNIX_EPOCH + Duration::from_millis(i));
        }
        assert_eq!(agg.aggregate().unwrap(), Duration::from_millis(999));
    }

    #[test]
    fn latency_aggregate() {
        let mut agg = LatencyAggregate::new([0.5, 0.9, 0.99]);
        for i in 1..1000 {
            agg.add(Duration::from_millis(i));
        }
        assert_eq!(
            agg.aggregate(),
            (
                Duration::from_millis(1),
                Duration::from_millis(500),
                vec![Duration::from_millis(500), Duration::from_millis(899), Duration::from_millis(989)],
                Duration::from_millis(999)
            )
        );
    }
}
