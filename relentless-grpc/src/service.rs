use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use tonic::{
    service::{interceptor::InterceptedService, Interceptor},
    transport::Channel,
};
use tower::Service;

#[derive(Debug, Clone)]
pub struct MakeChannel<I>(pub I);
impl<I> Service<http::Uri> for MakeChannel<I>
where
    I: Interceptor + Clone + Send + 'static,
{
    type Response = InterceptedService<Channel, I>;
    type Error = tonic::transport::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, destination: http::Uri) -> Self::Future {
        let interceptor = self.0.clone();
        Box::pin(async move {
            let channel = Channel::builder(destination).connect().await?;
            Ok(InterceptedService::new(channel, interceptor))
        })
    }
}
