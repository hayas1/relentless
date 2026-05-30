use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use opentelemetry_http::HeaderInjector;
use tower::{Layer, Service};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Clone, Debug)]
pub struct OtelInjectLayer;

impl<S> Layer<S> for OtelInjectLayer {
    type Service = OtelInjectService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OtelInjectService { inner }
    }
}

#[derive(Clone, Debug)]
pub struct OtelInjectService<S> {
    inner: S,
}

impl<S, B> Service<http::Request<B>> for OtelInjectService<S>
where
    S: Service<http::Request<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<B>) -> Self::Future {
        let cx = tracing::Span::current().context();

        opentelemetry::global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&cx, &mut HeaderInjector(req.headers_mut()))
        });

        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}
