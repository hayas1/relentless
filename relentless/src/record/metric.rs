use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Instant, SystemTime},
};

use semigroup::{op::HdrHistogram, Monoid, OptionMonoid, Semigroup};
use tower::{Layer, Service};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Metric {
    bytes: u64,
    timestamp: SystemTime,
    duration: (Instant, Instant),
}
impl Metric {
    pub fn new(bytes: u64, timestamp: SystemTime, duration: (Instant, Instant)) -> Self {
        Self { bytes, timestamp, duration }
    }
    pub fn into_agg(self) -> MetricAgg {
        self.into()
    }
}

#[derive(Debug, Clone, PartialEq, Semigroup)]
pub struct MetricAgg {
    #[semigroup(with = "semigroup::op::Sum")]
    times: u64,
    #[semigroup(with = "semigroup::op::Min")]
    start: Instant,
    #[semigroup(with = "semigroup::op::Max")]
    end: Instant,
    bytes: HdrHistogram<u64>,
    latency: HdrHistogram<u64>,
}
impl From<Metric> for MetricAgg {
    fn from(value: Metric) -> Self {
        let (start, end) = value.duration;
        let bytes = value.bytes.into();
        let latency = ((end - start).as_millis() as u64).into();
        MetricAgg { times: 1, start, end, bytes, latency }
    }
}

#[derive(Debug, Clone)]
pub struct MeasureLayer {
    agg: Arc<Mutex<OptionMonoid<MetricAgg>>>,
}
impl MeasureLayer {
    pub fn new() -> Self {
        let agg = Arc::new(Mutex::new(OptionMonoid::identity()));
        Self { agg }
    }
}
impl<S> Layer<S> for MeasureLayer {
    type Service = MeasureService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        let agg = self.agg.clone();
        MeasureService { inner, agg }
    }
}

#[derive(Debug, Clone)]
pub struct MeasureService<S> {
    inner: S,
    agg: Arc<Mutex<OptionMonoid<MetricAgg>>>,
}

impl<S, Req> Service<Req> for MeasureService<S>
where
    S: Service<Req> + Clone + Send + 'static,
    S::Response: Send,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let (fut, agg) = (self.inner.call(req), self.agg.clone());
        Box::pin(async move {
            let timestamp = SystemTime::now();

            let start = Instant::now();
            let result = fut.await;
            let end = Instant::now();

            let metric = Metric::new(0, timestamp, (start, end)).into_agg();
            let mut owned = agg.lock().unwrap();
            owned.semigroup_assign(metric.into());

            result
        })
    }
}
