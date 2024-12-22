use std::time::{Duration, SystemTime, SystemTimeError};

use hdrhistogram::Histogram;

use crate::assault::destinations::Destinations;

use super::metrics::Metrics;

// TODO remove Reportable trait ?
pub trait Aggregate: Default {
    type Add;
    type Aggregate;
    fn add(&mut self, add: &Self::Add);
    fn merge(&mut self, other: &Self);
    fn aggregate(&self) -> Self::Aggregate;
}

#[derive(Debug, Clone, Default)]
pub struct EvaluateAggregator {
    passed: PassAggregator,
    destinations: Destinations<ResponseAggregator>,
}
impl Aggregate for EvaluateAggregator {
    type Add = (bool, Destinations<Option<<ResponseAggregator as Aggregate>::Add>>);
    type Aggregate = (<PassAggregator as Aggregate>::Aggregate, <ResponseAggregator as Aggregate>::Aggregate);

    fn add(&mut self, (pass, dst): &Self::Add) {
        self.passed.add(pass);
        dst.iter().for_each(|(d, metrics)| {
            if let Some(m) = metrics {
                self.destinations.entry(d.to_string()).or_default().add(m);
            } else {
                // TODO can we skip the None metrics?
            }
        });
    }
    fn merge(&mut self, other: &Self) {
        self.passed.merge(&other.passed);
        other.destinations.iter().for_each(|(d, r)| {
            self.destinations.entry(d.to_string()).or_default().merge(r);
        })
    }
    fn aggregate(&self) -> Self::Aggregate {
        (
            self.passed.aggregate(),
            self.destinations
                .values()
                .fold(ResponseAggregator::default(), |mut agg, r| {
                    agg.merge(r);
                    agg
                })
                .aggregate(),
        )
    }
}
impl EvaluateAggregator {
    pub fn new<T, I: IntoIterator<Item = f64>>(dst: &Destinations<T>, now: Option<SystemTime>, quantile: I) -> Self {
        let percentile: Vec<_> = quantile.into_iter().collect();
        let destinations = dst.keys().map(|d| (d, ResponseAggregator::new(now, percentile.iter().copied()))).collect();
        Self { passed: PassAggregator::new(), destinations }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseAggregator {
    count: CountAggregator,
    duration: DurationAggregator,
    bytes: BytesAggregator,
    latency: LatencyAggregator,
}
pub type Rps = f64;
impl Aggregate for ResponseAggregator {
    type Add = Metrics;
    type Aggregate = (
        <CountAggregator as Aggregate>::Aggregate,
        <DurationAggregator as Aggregate>::Aggregate,
        Result<Rps, SystemTimeError>,
        <BytesAggregator as Aggregate>::Aggregate,
        <LatencyAggregator as Aggregate>::Aggregate,
    );
    fn add(&mut self, res: &Self::Add) {
        self.count.add(&());
        self.duration.add(&res.timestamp());
        self.bytes.add(&());
        self.latency.add(&res.latency());
    }
    fn merge(&mut self, other: &Self) {
        self.count.merge(&other.count);
        self.duration.merge(&other.duration);
        self.bytes.merge(&other.bytes);
        self.latency.merge(&other.latency);
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
impl ResponseAggregator {
    pub fn new<I: IntoIterator<Item = f64>>(now: Option<SystemTime>, quantile: I) -> Self {
        Self {
            count: CountAggregator::new(),
            duration: DurationAggregator::new(now),
            bytes: BytesAggregator {},
            latency: LatencyAggregator::new(quantile),
        }
    }

    pub fn rps(&self) -> Result<Rps, SystemTimeError> {
        Ok(self.count.aggregate() as f64 / self.duration.aggregate()?.as_secs_f64())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CountAggregator {
    count: Count,
}
pub type Count = u64;
impl Aggregate for CountAggregator {
    type Add = ();
    type Aggregate = Count;
    fn add(&mut self, (): &Self::Add) {
        self.count += 1;
    }
    fn merge(&mut self, other: &Self) {
        self.count += other.count;
    }
    fn aggregate(&self) -> Self::Aggregate {
        self.count
    }
}
impl CountAggregator {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct PassAggregator {
    pass: CountAggregator,
    count: CountAggregator,
}
pub type Pass = u64;
pub type PassRate = f64;
impl Aggregate for PassAggregator {
    type Add = bool;
    type Aggregate = (Pass, Count, PassRate);
    fn add(&mut self, pass: &Self::Add) {
        if *pass {
            self.pass.add(&());
        }
        self.count.add(&());
    }
    fn merge(&mut self, other: &Self) {
        self.pass.merge(&other.pass);
        self.count.merge(&other.count);
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.passed(), self.count(), self.ratio())
    }
}
impl PassAggregator {
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

#[derive(Debug, Clone, Default)]
pub struct DurationAggregator {
    start_end: Option<(SystemTime, SystemTime)>,
}
impl Aggregate for DurationAggregator {
    type Add = SystemTime;
    type Aggregate = Result<Duration, SystemTimeError>;
    fn add(&mut self, timestamp: &Self::Add) {
        match self.start_end {
            None => self.start_end = Some((*timestamp, *timestamp)),
            Some((start, end)) => self.start_end = Some((start.min(*timestamp), end.max(*timestamp))),
        }
    }
    fn merge(&mut self, other: &Self) {
        match (&self.start_end, &other.start_end) {
            (None, _) => self.start_end = other.start_end,
            (_, None) => (),
            (Some((start1, end1)), Some((start2, end2))) => {
                self.start_end = Some((*start1.min(start2), *end1.max(end2)))
            }
        }
    }
    fn aggregate(&self) -> Self::Aggregate {
        match &self.start_end {
            None => Ok(Duration::from_secs(0)),
            Some((start, end)) => Ok(end.duration_since(*start)?),
        }
    }
}
impl DurationAggregator {
    pub fn new(now: Option<SystemTime>) -> Self {
        Self { start_end: now.map(|t| (t, t)) }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BytesAggregator {
    // TODO implement
}
impl Aggregate for BytesAggregator {
    type Add = ();
    type Aggregate = ();
    fn add(&mut self, _: &Self::Add) {}
    fn merge(&mut self, _: &Self) {}
    fn aggregate(&self) -> Self::Aggregate {}
}

#[derive(Debug, Clone)]
pub struct LatencyAggregator {
    quantile: Vec<f64>,
    hist: Histogram<u64>,
}
pub type MinLatency = Duration;
pub type MeanLatency = Duration;
pub type QuantileLatencies = Vec<Duration>;
pub type MaxLatency = Duration;
impl Aggregate for LatencyAggregator {
    type Add = Duration;
    type Aggregate = (MinLatency, MeanLatency, QuantileLatencies, MaxLatency);
    fn add(&mut self, latency: &Self::Add) {
        self.hist += latency.as_nanos() as u64; // TODO overflow ?
    }
    fn merge(&mut self, other: &Self) {
        self.quantile = other.quantile.clone(); // Default quantile will be empty, so give priority to other
        self.hist += &other.hist;
    }
    fn aggregate(&self) -> Self::Aggregate {
        (self.min(), self.mean(), self.quantile(), self.max())
    }
}
impl Default for LatencyAggregator {
    fn default() -> Self {
        Self::new([])
    }
}
impl LatencyAggregator {
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
        let mut agg = CountAggregator::new();
        for _ in 0..1000 {
            agg.add(&());
        }
        assert_eq!(agg.aggregate(), 1000);
    }

    #[test]
    fn passed_aggregate() {
        let mut agg = PassAggregator::new();
        for i in 0..1000 {
            agg.add(&(i % 2 == 0));
        }
        assert_eq!(agg.aggregate(), (500, 1000, 0.5));
    }

    #[test]
    fn duration_aggregate() {
        let mut agg = DurationAggregator::new(Some(SystemTime::UNIX_EPOCH));
        for i in 0..1000 {
            agg.add(&(SystemTime::UNIX_EPOCH + Duration::from_millis(i)));
        }
        assert_eq!(agg.aggregate().unwrap(), Duration::from_millis(999));
    }

    #[test]
    fn latency_aggregate() {
        let mut agg = LatencyAggregator::new([0.5, 0.9, 0.99]);
        for i in 1..1000 {
            agg.add(&Duration::from_millis(i));
        }

        let tolerance = Duration::from_millis(1);
        let (min, mean, quantile, max) = agg.aggregate();

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

    #[test]
    fn merge_latency_aggregate() {
        let mut agg1 = LatencyAggregator::new([0.5, 0.9, 0.99]);
        for i in 1..500 {
            agg1.add(&Duration::from_millis(i));
        }
        let mut agg2 = LatencyAggregator::new([0.5, 0.9, 0.99]);
        for i in 500..1000 {
            agg2.add(&Duration::from_millis(i));
        }

        let tolerance = Duration::from_millis(1);
        let (min1, mean1, quantile1, max1) = agg1.aggregate();
        let (min2, mean2, quantile2, max2) = agg2.aggregate();

        assert!(min1.abs_diff(Duration::from_millis(1)) < tolerance);
        assert!(mean1.abs_diff(Duration::from_millis(250)) < tolerance);
        for (q, p) in quantile1.iter().zip(vec![
            Duration::from_millis(250),
            Duration::from_millis(450),
            Duration::from_millis(495),
        ]) {
            assert!(q.abs_diff(p) < tolerance);
        }
        assert!(max1.abs_diff(Duration::from_millis(500)) < tolerance);

        assert!(min2.abs_diff(Duration::from_millis(500)) < tolerance);
        assert!(mean2.abs_diff(Duration::from_millis(750)) < tolerance);
        for (q, p) in quantile2.iter().zip(vec![
            Duration::from_millis(750),
            Duration::from_millis(950),
            Duration::from_millis(995),
        ]) {
            assert!(q.abs_diff(p) < tolerance);
        }
        assert!(max2.abs_diff(Duration::from_millis(1000)) < tolerance);

        agg1.merge(&agg2);
        let (min, mean, quantile, max) = agg1.aggregate();
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
