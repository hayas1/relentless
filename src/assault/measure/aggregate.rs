use std::{
    marker::PhantomData,
    time::{Duration, SystemTime, SystemTimeError},
};

use average::{Estimate, Max, Mean, Min, Quantile};

use crate::assault::destinations::Destinations;

use super::metrics::MeasuredResponse;

pub trait Aggregator {
    type Add;
    type Aggregate;
    fn add(&mut self, add: &Self::Add);
    fn aggregate(&self) -> Self::Aggregate;
}

pub struct EvaluateAggregate<Res> {
    passed: PassAggregate,
    destinations: Destinations<ResponseAggregate<Res>>,
    phantom: PhantomData<Res>,
}
impl<Res> Aggregator for EvaluateAggregate<Res> {
    type Add = (bool, Destinations<MeasuredResponse<Res>>);
    type Aggregate =
        (<PassAggregate as Aggregator>::Aggregate, Destinations<<ResponseAggregate<Res> as Aggregator>::Aggregate>);

    fn add(&mut self, (pass, dst): &Self::Add) {
        self.passed.add(pass);
        self.destinations.iter_mut().for_each(|(d, r)| r.add(&dst[d])); // TODO error handling
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.passed.aggregate(), self.destinations.iter().map(|(d, r)| (d, r.aggregate())).collect())
    }
}
impl<Res> EvaluateAggregate<Res> {
    pub fn new<T, I: IntoIterator<Item = f64>>(dst: &Destinations<T>, now: SystemTime, quantile: I) -> Self {
        let percentile: Vec<_> = quantile.into_iter().collect();
        let destinations = dst.keys().map(|d| (d, ResponseAggregate::new(now, percentile.iter().copied()))).collect();
        Self { passed: PassAggregate::new(), destinations, phantom: PhantomData }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseAggregate<Res> {
    count: CountAggregate,
    duration: DurationAggregate,
    bytes: BytesAggregate,
    latency: LatencyAggregate,
    phantom: PhantomData<Res>,
}
pub type Rps = f64;
impl<Res> Aggregator for ResponseAggregate<Res> {
    type Add = MeasuredResponse<Res>;
    type Aggregate = (
        <CountAggregate as Aggregator>::Aggregate,
        <DurationAggregate as Aggregator>::Aggregate,
        Result<Rps, SystemTimeError>,
        <BytesAggregate as Aggregator>::Aggregate,
        <LatencyAggregate as Aggregator>::Aggregate,
    );
    fn add(&mut self, res: &Self::Add) {
        self.count.add(&());
        self.duration.add(&res.timestamp());
        self.bytes.add(&());
        self.latency.add(&res.latency());
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
    pub fn new<I: IntoIterator<Item = f64>>(now: SystemTime, quantile: I) -> Self {
        Self {
            count: CountAggregate::new(),
            duration: DurationAggregate::new(now),
            bytes: BytesAggregate {},
            latency: LatencyAggregate::new(quantile),
            phantom: PhantomData,
        }
    }

    pub fn rps(&self) -> Result<Rps, SystemTimeError> {
        Ok(self.count.aggregate() as f64 / self.duration.aggregate()?.as_secs_f64())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CountAggregate {
    count: Count,
}
pub type Count = u64;
impl Aggregator for CountAggregate {
    type Add = ();
    type Aggregate = Count;
    fn add(&mut self, (): &Self::Add) {
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
pub struct PassAggregate {
    pass: CountAggregate,
    count: CountAggregate,
}
pub type Pass = u64;
pub type PassRate = f64;
impl Aggregator for PassAggregate {
    type Add = bool;
    type Aggregate = (Pass, Count, PassRate);
    fn add(&mut self, pass: &Self::Add) {
        if *pass {
            self.pass.add(&());
        }
        self.count.add(&());
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.passed(), self.count(), self.ratio())
    }
}
impl PassAggregate {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn count(&self) -> Count {
        self.count.aggregate()
    }
    pub fn passed(&self) -> Pass {
        self.pass.aggregate()
    }
    pub fn ratio(&self) -> PassRate {
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
    fn add(&mut self, timestamp: &Self::Add) {
        self.first = self.first.min(*timestamp);
        self.last = self.last.max(*timestamp);
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
    fn add(&mut self, _: &Self::Add) {}
    fn aggregate(&self) -> Self::Aggregate {}
}

#[derive(Debug, Clone)]
pub struct LatencyAggregate {
    min: Min,
    mean: Mean,
    quantile: Vec<Quantile>,
    max: Max,
}
pub type MinLatency = Duration;
pub type MeanLatency = Duration;
pub type QuantileLatencies = Vec<Duration>;
pub type MaxLatency = Duration;
impl Aggregator for LatencyAggregate {
    type Add = Duration;
    type Aggregate = (MinLatency, MeanLatency, QuantileLatencies, MaxLatency);
    fn add(&mut self, latency: &Self::Add) {
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

    pub fn min(&self) -> MinLatency {
        Duration::from_secs_f64(self.min.min())
    }
    pub fn mean(&self) -> MeanLatency {
        Duration::from_secs_f64(self.mean.mean())
    }
    pub fn quantile(&self) -> QuantileLatencies {
        self.quantile.iter().map(|q| Duration::from_secs_f64(q.quantile())).collect()
    }
    pub fn max(&self) -> MaxLatency {
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
            agg.add(&());
        }
        assert_eq!(agg.aggregate(), 1000);
    }

    #[test]
    fn passed_aggregate() {
        let mut agg = PassAggregate::new();
        for i in 0..1000 {
            agg.add(&(i % 2 == 0));
        }
        assert_eq!(agg.aggregate(), (500, 1000, 0.5));
    }

    #[test]
    fn duration_aggregate() {
        let mut agg = DurationAggregate::new(SystemTime::UNIX_EPOCH);
        for i in 0..1000 {
            agg.add(&(SystemTime::UNIX_EPOCH + Duration::from_millis(i)));
        }
        assert_eq!(agg.aggregate().unwrap(), Duration::from_millis(999));
    }

    #[test]
    fn latency_aggregate() {
        let mut agg = LatencyAggregate::new([0.5, 0.9, 0.99]);
        for i in 1..1000 {
            agg.add(&Duration::from_millis(i));
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
