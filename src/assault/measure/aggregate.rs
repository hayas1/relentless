use std::time::{Duration, SystemTime, SystemTimeError};

use hdrhistogram::Histogram;

use crate::assault::destinations::Destinations;

use super::metrics::Metrics;

pub trait Aggregator {
    type Add;
    type Aggregate;
    fn add(&mut self, add: &Self::Add);
    fn aggregate(&self) -> Self::Aggregate;
}

pub struct EvaluateAggregate {
    passed: PassAggregate,
    destinations: Destinations<ResponseAggregate>,
}
impl Aggregator for EvaluateAggregate {
    type Add = (bool, Destinations<<ResponseAggregate as Aggregator>::Add>);
    type Aggregate =
        (<PassAggregate as Aggregator>::Aggregate, Destinations<<ResponseAggregate as Aggregator>::Aggregate>);

    fn add(&mut self, (pass, dst): &Self::Add) {
        self.passed.add(pass);
        self.destinations.iter_mut().for_each(|(d, r)| r.add(&dst[d])); // TODO error handling
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.passed.aggregate(), self.destinations.iter().map(|(d, r)| (d, r.aggregate())).collect())
    }
}
impl EvaluateAggregate {
    pub fn new<T, I: IntoIterator<Item = f64>>(dst: &Destinations<T>, now: SystemTime, quantile: I) -> Self {
        let percentile: Vec<_> = quantile.into_iter().collect();
        let destinations = dst.keys().map(|d| (d, ResponseAggregate::new(now, percentile.iter().copied()))).collect();
        Self { passed: PassAggregate::new(), destinations }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseAggregate {
    count: CountAggregate,
    duration: DurationAggregate,
    bytes: BytesAggregate,
    latency: LatencyAggregate,
}
pub type Rps = f64;
impl Aggregator for ResponseAggregate {
    type Add = Metrics;
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
impl ResponseAggregate {
    pub fn new<I: IntoIterator<Item = f64>>(now: SystemTime, quantile: I) -> Self {
        Self {
            count: CountAggregate::new(),
            duration: DurationAggregate::new(now),
            bytes: BytesAggregate {},
            latency: LatencyAggregate::new(quantile),
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
    quantile: Vec<f64>,
    hist: Histogram<u64>,
}
pub type MinLatency = Duration;
pub type MeanLatency = Duration;
pub type QuantileLatencies = Vec<Duration>;
pub type MaxLatency = Duration;
impl Aggregator for LatencyAggregate {
    type Add = Duration;
    type Aggregate = (MinLatency, MeanLatency, QuantileLatencies, MaxLatency);
    fn add(&mut self, latency: &Self::Add) {
        self.hist += latency.as_nanos() as u64; // TODO overflow ?
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.min(), self.mean(), self.quantile(), self.max())
    }
}
impl LatencyAggregate {
    pub fn new<I: IntoIterator<Item = f64>>(quantile: I) -> Self {
        let quantile = quantile.into_iter().collect();
        let hist = Histogram::new(3).unwrap_or_else(|_| todo!());
        Self { quantile, hist }
    }

    pub fn min(&self) -> MinLatency {
        Duration::from_nanos(self.hist.min())
    }
    pub fn mean(&self) -> MeanLatency {
        Duration::from_nanos(self.hist.mean() as u64)
    }
    pub fn quantile(&self) -> QuantileLatencies {
        self.quantile.iter().map(|q| self.value_at_quantile(*q)).collect()
    }
    pub fn value_at_quantile(&self, quantile: f64) -> Duration {
        Duration::from_nanos(self.hist.value_at_quantile(quantile))
    }
    pub fn max(&self) -> MaxLatency {
        Duration::from_nanos(self.hist.max())
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

        let (min, mean, quantile, max) = agg.aggregate();
        let tolerance = Duration::from_millis(1);

        assert!(min.abs_diff(Duration::from_millis(1)) < tolerance);
        assert!(mean.abs_diff(Duration::from_millis(500)) < tolerance);
        for (q, p) in quantile.iter().zip(vec![
            Duration::from_millis(500),
            Duration::from_millis(900),
            Duration::from_millis(990),
        ]) {
            assert!(q.abs_diff(p) < tolerance);
        }
        assert!(max.abs_diff(Duration::from_millis(1000)) < tolerance);
    }
}
