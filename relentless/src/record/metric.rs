use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::{Duration, Instant, SystemTime},
};

use bytesize::ByteSize;
use pin_project::pin_project;
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
struct MetricAggInner {
    #[semigroup(with = "semigroup::op::Sum")]
    times: u64,
    #[semigroup(with = "semigroup::op::Min")]
    start: Instant,
    #[semigroup(with = "semigroup::op::Max")]
    end: Instant,
    bytes: HdrHistogram<u64>,
    latency: HdrHistogram<u64>,
}
#[derive(Debug, Clone, PartialEq, Semigroup)]
#[semigroup(monoid)]
pub struct MetricAgg(OptionMonoid<MetricAggInner>);
impl From<Metric> for MetricAgg {
    fn from(value: Metric) -> Self {
        let (start, end) = value.duration;
        let bytes = value.bytes.into();
        let latency = ((end - start).as_millis() as u64).into();
        Self(MetricAggInner { times: 1, start, end, bytes, latency }.into())
    }
}
impl MetricAgg {
    pub fn times(&self) -> u64 {
        self.0.as_ref().map(|agg| agg.times).unwrap_or(0)
    }
    pub fn duration(&self) -> Duration {
        self.0.as_ref().map(|agg| agg.end - agg.start).unwrap_or_default()
    }
    pub fn rps(&self) -> f64 {
        self.times() as f64 / self.duration().as_secs_f64()
    }
    pub fn approx_bytes_quantile(&self, quantile: f64) -> ByteSize {
        self.0
            .as_ref()
            .map(|agg| agg.bytes.histogram().value_at_quantile(quantile))
            .map(ByteSize::b)
            .unwrap_or_default()
    }
    pub fn approx_latency_quantile(&self, quantile: f64) -> Duration {
        self.0
            .as_ref()
            .map(|agg| agg.latency.histogram().value_at_quantile(quantile))
            .map(Duration::from_millis)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct MeasureLayer {
    agg: Arc<Mutex<MetricAgg>>,
}
impl MeasureLayer {
    pub fn new() -> Self {
        let agg = Arc::new(Mutex::new(MetricAgg::identity()));
        Self { agg }
    }
    pub fn aggregated(&self) -> MetricAgg {
        self.agg.lock().unwrap().clone()
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
    agg: Arc<Mutex<MetricAgg>>,
}

impl<S, Req> Service<Req> for MeasureService<S>
where
    S: Service<Req>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = MeasureFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        MeasureFuture::new(self.inner.call(req), self.agg.clone())
    }
}

#[pin_project]
pub struct MeasureFuture<F> {
    #[pin]
    fut: F,
    start: Option<(SystemTime, Instant)>,
    end: Option<Box<dyn FnOnce() -> Instant>>,
    agg: Arc<Mutex<MetricAgg>>,
}
impl<F> MeasureFuture<F> {
    pub fn new(fut: F, agg: Arc<Mutex<MetricAgg>>) -> Self {
        Self { fut, start: None, end: Some(Box::new(Instant::now)), agg }
    }
}

impl<F, T, E> Future for MeasureFuture<F>
where
    F: Future<Output = Result<T, E>>,
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if this.start.is_none() {
            *this.start = Some((SystemTime::now(), Instant::now()));
        }

        this.fut.poll(cx).map(|o| {
            let Some((timestamp, start)) = this.start.take() else { unreachable!() };
            let end = this.end.take().expect("poll after ready")();

            let metric = Metric::new(0, timestamp, (start, end)).into_agg();
            let mut agg = this.agg.lock().unwrap();
            agg.semigroup_assign(metric);

            o
        })
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use futures::StreamExt;
    use tower::{ServiceBuilder, ServiceExt};

    use super::*;

    #[tokio::test]
    async fn test_measure_layer() {
        let measure = MeasureLayer::new();
        let svc = tower::service_fn(|_| async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok::<_, Infallible>(())
        });

        let service = ServiceBuilder::new().layer(measure.clone()).service(svc);
        let stream = futures::stream::iter(0..180)
            .map(|_| async { service.clone().oneshot(()).await.unwrap() })
            .buffer_unordered(180);

        let count = stream.count().await;

        // TODO runtime
        let agg = measure.aggregated();
        assert_eq!(agg.times(), count as u64);
        assert!((175.0..180.0).contains(&agg.rps()));
        assert!((Duration::from_millis(1000)..Duration::from_millis(1100)).contains(&agg.approx_latency_quantile(0.99)))
    }
}
