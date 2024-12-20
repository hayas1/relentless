use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::SystemTime,
};

use tower::{timeout::error::Elapsed, Layer, Service};

use crate::assault::{
    measure::metrics::MeasuredResponse,
    metrics::{RequestError, RequestResult},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct MeasureLayer;

impl<S> Layer<S> for MeasureLayer {
    type Service = MeasureService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MeasureService { inner }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct MeasureService<S> {
    inner: S,
}

impl<S, Req> Service<Req> for MeasureService<S>
where
    S: Service<Req> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = MeasuredResponse<S::Response>; // TODO contain byte size, (http status?), ...
    type Error = RequestError;
    type Future = Pin<Box<dyn Future<Output = RequestResult<S::Response>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|e| RequestError::NoReady(e.into()))
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let fut = self.inner.call(req);
        Box::pin(async {
            let timestamp = SystemTime::now();
            let result = fut.await;
            let latency = timestamp.elapsed().map_err(RequestError::FailToMeasureLatency)?; // TODO this error should be allowed?

            let response = result.map_err(|error| {
                let boxed: Box<dyn std::error::Error + Send + Sync> = error.into();
                if let Some(err) = boxed.downcast_ref() {
                    match err {
                        RequestError::InnerServiceError(e) => {
                            if e.is::<Elapsed>() {
                                RequestError::Timeout(latency)
                            } else {
                                RequestError::InnerServiceError(boxed)
                            }
                        }
                        _ => RequestError::Unknown(boxed),
                    }
                } else {
                    RequestError::Unknown(boxed)
                }
            })?;
            Ok(MeasuredResponse::new(response, timestamp, latency))
        })
    }
}
