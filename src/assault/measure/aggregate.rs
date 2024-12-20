use std::time::{Duration, SystemTime};

use average::{Estimate, Max, Mean, Min, Quantile};

#[derive(Debug, Clone)]
pub struct CountAggregate {
    count: u64,
    passed: u64,
    first: SystemTime,
    last: SystemTime,
}

impl CountAggregate {
    pub fn new(now: SystemTime) -> Self {
        Self { count: 0, passed: 0, first: now, last: now }
    }

    pub fn add(&mut self, passed: bool, timestamp: SystemTime) {
        self.count += 1;
        self.passed += passed as u64;
        self.first = self.first.min(timestamp);
        self.last = self.last.max(timestamp);
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
pub struct LatencyAggregate {
    min: Min,
    mean: Mean,
    quantile: Vec<Quantile>,
    max: Max,
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

    pub fn add(&mut self, latency: Duration) {
        let nanos = latency.as_secs_f64();
        self.min.add(nanos);
        self.mean.add(nanos);
        self.quantile.iter_mut().for_each(|q| q.add(nanos));
        self.max.add(nanos);
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
    pub fn aggregate(&self) -> (Duration, Duration, Vec<Duration>, Duration) {
        (self.min(), self.mean(), self.quantile(), self.max())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_aggregate() {
        let mut agg = CountAggregate::new(SystemTime::UNIX_EPOCH);
        for i in 0..1000 {
            agg.add(i % 2 == 0, SystemTime::UNIX_EPOCH + Duration::from_millis(i));
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
