use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use thiserror::Error;
use tower::{timeout::error::Elapsed, Service};

use crate::error::Wrap;

pub type RequestResult<T> = Result<T, RequestError>;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("timeout in {0:?}")]
    Timeout(Duration),

    // TODO use S::Error ?
    #[error(transparent)]
    NoReady(Wrap),
    #[error(transparent)]
    RequestError(Wrap),
}

pub struct RequestService<S, Req> {
    inner: S,
    phantom: PhantomData<Req>,
}

impl<S, Req> Service<Req> for RequestService<S, Req>
where
    S: Service<Req> + Clone + Send + 'static,
    S::Future: Send + 'static,
    // S::Error: std::error::Error + Send + Sync + 'static,
    Wrap: From<S::Error>,
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

            result.map_err(|e| {
                let wrapped = Wrap::from(e);
                if wrapped.is::<Elapsed>() {
                    RequestError::Timeout(latency)
                } else {
                    RequestError::RequestError(wrapped)
                }
            })
        })
    }
}
