use std::time::{Duration, SystemTime, SystemTimeError};

use hdrhistogram::Histogram;

use crate::assault::destinations::Destinations;

use super::metrics::Metrics;

pub trait Aggregate: Default {
    type Add;
    type Query;
    type Aggregate;
    fn add(&mut self, add: &Self::Add);
    fn merge(&mut self, other: &Self);
    fn aggregate(&self, query: &Self::Query) -> Self::Aggregate;
}

#[derive(Debug, Clone, Default)]
pub struct EvaluateAggregator {
    pass: PassAggregator,
    destinations: Destinations<ResponseAggregator>,
}
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct EvaluateAggregate {
    pub pass: PassAggregate,
    pub response: ResponseAggregate,
}
impl Aggregate for EvaluateAggregator {
    type Add = (bool, Destinations<Option<<ResponseAggregator as Aggregate>::Add>>);
    type Query = Vec<f64>; // TODO [f64] ?
    type Aggregate = EvaluateAggregate;

    fn add(&mut self, (pass, dst): &Self::Add) {
        self.pass.add(pass);
        dst.iter().for_each(|(d, metrics)| {
            if let Some(m) = metrics {
                self.destinations.entry(d.to_string()).or_default().add(m);
            } else {
                // TODO can we skip the None metrics?
            }
        });
    }
    fn merge(&mut self, other: &Self) {
        self.pass.merge(&other.pass);
        other.destinations.iter().for_each(|(d, r)| {
            self.destinations.entry(d.to_string()).or_default().merge(r);
        })
    }
    fn aggregate(&self, query: &Self::Query) -> Self::Aggregate {
        EvaluateAggregate { pass: self.pass.aggregate(&()), response: self.aggregate_responses(query) }
    }
}
impl EvaluateAggregator {
    pub fn new<T>(dst: &Destinations<T>, now: Option<SystemTime>) -> Self {
        let destinations = dst.keys().map(|d| (d, ResponseAggregator::new(now))).collect();
        Self { pass: PassAggregator::new(), destinations }
    }

    pub fn aggregate_responses(&self, query: &[f64]) -> ResponseAggregate {
        self.destinations
            .values()
            .fold(ResponseAggregator::default(), |mut agg, r| {
                agg.merge(r);
                agg
            })
            .aggregate(&query.to_vec())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResponseAggregator {
    count: CountAggregator,
    duration: DurationAggregator,
    bytes: BytesAggregator,
    latency: LatencyAggregator,
}
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct ResponseAggregate {
    pub req: u64,
    pub duration: Option<Duration>,
    pub rps: Option<f64>,
    pub bytes: BytesAggregate,
    pub latency: LatencyAggregate,
}
impl Aggregate for ResponseAggregator {
    type Add = Metrics;
    type Query = Vec<f64>; // TODO [f64] ?
    type Aggregate = ResponseAggregate;
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
    fn aggregate(&self, query: &Self::Query) -> Self::Aggregate {
        ResponseAggregate {
            req: self.count.aggregate(&()),
            duration: self.duration.aggregate(&()).ok(),
            rps: self.rps().ok(),
            bytes: self.bytes.aggregate(&()),
            latency: self.latency.aggregate(query),
        }
    }
}
impl ResponseAggregator {
    pub fn new(now: Option<SystemTime>) -> Self {
        Self {
            count: CountAggregator::new(),
            duration: DurationAggregator::new(now),
            bytes: BytesAggregator {},
            latency: LatencyAggregator::new(),
        }
    }

    pub fn rps(&self) -> Result<f64, SystemTimeError> {
        Ok(self.count.aggregate(&()) as f64 / self.duration.aggregate(&())?.as_secs_f64())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CountAggregator {
    count: u64,
}
impl Aggregate for CountAggregator {
    type Add = ();
    type Query = ();
    type Aggregate = u64;
    fn add(&mut self, (): &Self::Add) {
        self.count += 1;
    }
    fn merge(&mut self, other: &Self) {
        self.count += other.count;
    }
    fn aggregate(&self, (): &Self::Query) -> Self::Aggregate {
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
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct PassAggregate {
    pub pass: u64,
    pub count: u64,
    pub pass_rate: f64,
}
impl Aggregate for PassAggregator {
    type Add = bool;
    type Query = ();
    type Aggregate = PassAggregate;
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
    fn aggregate(&self, (): &Self::Query) -> Self::Aggregate {
        PassAggregate { pass: self.pass(), count: self.count(), pass_rate: self.pass_rate() }
    }
}
impl PassAggregator {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn count(&self) -> u64 {
        self.count.aggregate(&())
    }
    pub fn pass(&self) -> u64 {
        self.pass.aggregate(&())
    }
    pub fn pass_rate(&self) -> f64 {
        self.pass() as f64 / self.count() as f64
    }
}

#[derive(Debug, Clone, Default)]
pub struct DurationAggregator {
    start_end: Option<(SystemTime, SystemTime)>,
}
impl Aggregate for DurationAggregator {
    type Add = SystemTime;
    type Query = ();
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
    fn aggregate(&self, (): &Self::Query) -> Self::Aggregate {
        Ok(match &self.start_end {
            None => Duration::from_secs(0),
            Some((start, end)) => end.duration_since(*start)?,
        })
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct BytesAggregate {}
impl Aggregate for BytesAggregator {
    type Add = ();
    type Query = ();
    type Aggregate = BytesAggregate;
    fn add(&mut self, _: &Self::Add) {}
    fn merge(&mut self, _: &Self) {}
    fn aggregate(&self, (): &Self::Query) -> Self::Aggregate {
        Default::default()
    }
}

#[derive(Debug, Clone)]
pub struct LatencyAggregator {
    hist: Histogram<u64>,
}
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct LatencyAggregate {
    pub min: Duration,
    pub mean: Duration,
    pub quantile: Vec<Duration>,
    pub max: Duration,
}
impl Aggregate for LatencyAggregator {
    type Add = Duration;
    type Query = Vec<f64>; // TODO [f64] ?
    type Aggregate = LatencyAggregate;
    fn add(&mut self, latency: &Self::Add) {
        self.hist += latency.as_micros() as u64;
    }
    fn merge(&mut self, other: &Self) {
        // TODO Default quantile will be empty, so give priority to other
        // TODO `Aggregate` trait should do not have `aggregate` method, but have `sub_aggregator` method ?
        self.hist += &other.hist;
    }
    fn aggregate(&self, query: &Self::Query) -> Self::Aggregate {
        LatencyAggregate { min: self.min(), mean: self.mean(), quantile: self.quantile(query), max: self.max() }
    }
}
impl Default for LatencyAggregator {
    fn default() -> Self {
        let hist = Histogram::new(3).unwrap_or_else(|e| unreachable!("{}", e));
        Self { hist }
    }
}
impl LatencyAggregator {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn min(&self) -> Duration {
        Duration::from_micros(self.hist.min())
    }
    pub fn mean(&self) -> Duration {
        Duration::from_micros(self.hist.mean() as u64)
    }
    pub fn quantile(&self, quantile: &[f64]) -> Vec<Duration> {
        quantile.iter().map(|&q| self.value_at_quantile(q)).collect()
    }
    pub fn value_at_quantile(&self, quantile: f64) -> Duration {
        Duration::from_micros(self.hist.value_at_quantile(quantile))
    }
    pub fn max(&self) -> Duration {
        Duration::from_micros(self.hist.max())
    }
}

#[cfg(test)]
mod tests {
    use crate::assault::measure::metrics::MeasuredResponse;

    use super::*;

    #[test]
    fn count_aggregate() {
        let mut agg = CountAggregator::new();
        for _ in 0..1000 {
            agg.add(&());
        }
        assert_eq!(agg.aggregate(&()), 1000);
    }

    #[test]
    fn passed_aggregate() {
        let mut agg = PassAggregator::new();
        for i in 0..1000 {
            agg.add(&(i % 2 == 0));
        }
        assert_eq!(agg.aggregate(&()), PassAggregate { pass: 500, count: 1000, pass_rate: 0.5 });
    }

    #[test]
    fn duration_aggregate() {
        let mut agg = DurationAggregator::new(Some(SystemTime::UNIX_EPOCH));
        for i in 0..1000 {
            agg.add(&(SystemTime::UNIX_EPOCH + Duration::from_millis(i)));
        }
        assert_eq!(agg.aggregate(&()).unwrap(), Duration::from_millis(999));
    }

    #[test]
    fn latency_aggregate() {
        let mut agg = LatencyAggregator::new();
        for i in 1..1000 {
            agg.add(&Duration::from_millis(i));
        }

        let tolerance = Duration::from_millis(1);
        let LatencyAggregate { min, mean, quantile, max } = agg.aggregate(&vec![0.5, 0.9, 0.99]);

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
        let mut agg1 = LatencyAggregator::new();
        for i in 1..500 {
            agg1.add(&Duration::from_millis(i));
        }
        let mut agg2 = LatencyAggregator::new();
        for i in 500..1000 {
            agg2.add(&Duration::from_millis(i));
        }

        let tolerance = Duration::from_millis(1);
        let LatencyAggregate { min: min1, mean: mean1, quantile: quantile1, max: max1 } =
            agg1.aggregate(&vec![0.5, 0.9, 0.99]);
        let LatencyAggregate { min: min2, mean: mean2, quantile: quantile2, max: max2 } =
            agg2.aggregate(&vec![0.5, 0.9, 0.99]);

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
        let LatencyAggregate { min, mean, quantile, max } = agg1.aggregate(&vec![0.5, 0.9, 0.99]);
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
    fn evaluate_aggregate() {
        let mut agg = EvaluateAggregator::new(&Destinations::<()>::new(), Some(SystemTime::UNIX_EPOCH));
        for i in 0..1000 {
            let d = vec![
                (
                    "a",
                    Some(
                        MeasuredResponse::new(
                            (),
                            SystemTime::UNIX_EPOCH + Duration::from_millis(i),
                            Duration::from_millis(i),
                        )
                        .metrics()
                        .clone(),
                    ),
                ),
                (
                    "b",
                    Some(
                        MeasuredResponse::new(
                            (),
                            SystemTime::UNIX_EPOCH + Duration::from_millis(i),
                            Duration::from_millis(i),
                        )
                        .metrics()
                        .clone(),
                    ),
                ),
            ]
            .into_iter()
            .collect();
            agg.add(&(i % 2 == 0, d));
        }

        let tolerance = Duration::from_millis(1);
        let EvaluateAggregate { pass, response } = agg.aggregate(&vec![0.5, 0.9, 0.99]);
        assert_eq!(pass, PassAggregate { pass: 500, count: 1000, pass_rate: 0.5 });
        let ResponseAggregate { req, duration, rps, bytes, latency } = response;
        assert_eq!(req, 2000);
        assert!(duration.unwrap().abs_diff(Duration::from_millis(999)) < tolerance);
        assert_eq!(rps, Some(2002.002002002002));
        assert_eq!(bytes, BytesAggregate {});
        let LatencyAggregate { min, mean, quantile, max } = latency;
        assert!(min.abs_diff(Duration::from_millis(0)) < tolerance);
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
