use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use thiserror::Error;
use tower::{timeout::error::Elapsed, Layer, Service};

pub type RequestResult<T> = Result<T, RequestError>;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("request timeout: {0:?}")]
    Timeout(Duration),

    #[error(transparent)]
    FailToMakeRequest(Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    NoReady(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    InnerServiceError(Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    Unknown(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct RequestLayer;

impl<S> Layer<S> for RequestLayer {
    type Service = RequestService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestService { inner }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct RequestService<S> {
    inner: S,
}

impl<S, Req> Service<Req> for RequestService<S>
where
    S: Service<Req> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = S::Response; // TODO contain timestamp, latency, byte size, ...
    type Error = RequestError;
    type Future = Pin<Box<dyn Future<Output = RequestResult<Self::Response>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|e| RequestError::NoReady(e.into()))
    }

    fn call(&mut self, req: Req) -> Self::Future {
        let fut = self.inner.call(req);
        Box::pin(async {
            let now = Instant::now();
            let result = fut.await;
            let latency = now.elapsed();

            result.map_err(|error| {
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
            })
        })
    }
}
