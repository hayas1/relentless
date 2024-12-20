use std::{
    marker::PhantomData,
    time::{Duration, SystemTime},
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
    bytes: BytesAggregate,
    latency: LatencyAggregate,
    phantom: PhantomData<Res>,
}
impl<Res> Aggregator for ResponseAggregate<Res> {
    type Add = (bool, MeasuredResponse<Res>);
    type Aggregate = (
        <CountAggregate as Aggregator>::Aggregate,
        <BytesAggregate as Aggregator>::Aggregate,
        <LatencyAggregate as Aggregator>::Aggregate,
    );
    fn add(&mut self, (pass, res): Self::Add) {
        self.count.add((pass, res.timestamp()));
        self.bytes.add(());
        self.latency.add(res.latency());
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.count.aggregate(), self.bytes.aggregate(), self.latency.aggregate())
    }
}

#[derive(Debug, Clone)]
pub struct CountAggregate {
    count: u64,
    passed: u64,
    first: SystemTime,
    last: SystemTime,
}
impl Aggregator for CountAggregate {
    type Add = (bool, SystemTime);
    type Aggregate = (u64, u64, f64, f64);
    fn add(&mut self, (passed, timestamp): Self::Add) {
        self.count += 1;
        self.passed += passed as u64;
        self.first = self.first.min(timestamp);
        self.last = self.last.max(timestamp);
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.count, self.passed, self.success_rate(), self.rps())
    }
}
impl CountAggregate {
    pub fn new(now: SystemTime) -> Self {
        Self { count: 0, passed: 0, first: now, last: now }
    }

    pub fn count(&self) -> u64 {
        self.count
    }
    pub fn passed(&self) -> u64 {
        self.passed
    }
    pub fn success_rate(&self) -> f64 {
        self.passed as f64 / self.count as f64
    }
    pub fn rps(&self) -> f64 {
        let elapsed = self.last.duration_since(self.first).unwrap_or_default();
        self.count as f64 / elapsed.as_secs_f64()
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
        let mut agg = CountAggregate::new(SystemTime::UNIX_EPOCH);
        for i in 0..1000 {
            agg.add((i % 2 == 0, SystemTime::UNIX_EPOCH + Duration::from_millis(i)));
        }
        assert_eq!(agg.count(), 1000);
        assert_eq!(agg.passed(), 500);
        assert_eq!(agg.success_rate(), 0.5);
        assert_eq!(agg.rps(), 1001.001001001001);
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
