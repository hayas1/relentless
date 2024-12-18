use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, SystemTime, SystemTimeError},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tower::{timeout::error::Elapsed, Layer, Service};

pub type RequestResult<Res> = Result<MetaResponse<Res>, RequestError>;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetaResponse<Res> {
    response: Res,
    timestamp: SystemTime,
    latency: Duration,
}
impl<Res> MetaResponse<Res> {
    pub fn new(response: Res, timestamp: SystemTime, latency: Duration) -> Self {
        Self { response, timestamp, latency }
    }

    pub fn response(&self) -> &Res {
        &self.response
    }
    pub fn into_response(self) -> Res {
        self.response
    }

    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    pub fn end_timestamp(&self) -> SystemTime {
        self.timestamp + self.latency
    }
    pub fn latency(&self) -> Duration {
        self.latency
    }
}

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
    FailToMeasureLatency(SystemTimeError),
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
    type Response = MetaResponse<S::Response>; // TODO contain byte size, (http status?), ...
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
            Ok(MetaResponse::new(response, timestamp, latency))
        })
    }
}
