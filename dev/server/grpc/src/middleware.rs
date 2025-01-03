pub mod logging {
    use std::{
        fmt::{Debug, Display},
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    use tower::{Layer, Service};

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
    pub struct LoggingLayer;
    impl<S> Layer<S> for LoggingLayer {
        type Service = LoggingService<S>;

        fn layer(&self, service: S) -> Self::Service {
            LoggingService { inner: service }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
    pub struct LoggingService<S> {
        inner: S,
    }
    impl<S, Req> Service<Req> for LoggingService<S>
    where
        Req: Debug,
        S: Service<Req> + Send + 'static,
        S::Response: Debug,
        S::Future: Send + 'static,
        S::Error: Display,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, req: Req) -> Self::Future {
            tracing::info!("{:?}", &req);
            let fut = self.inner.call(req);
            Box::pin(async {
                match fut.await {
                    Ok(res) => {
                        tracing::info!("{:?}", &res);
                        Ok(res)
                    }
                    Err(e) => {
                        tracing::error!("{}", e);
                        Err(e)
                    }
                }
            })
        }
    }
}
