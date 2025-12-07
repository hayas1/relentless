use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use tonic::transport::Channel;
use tower::{Layer, Service};

#[derive(Debug, Clone)]
pub struct MakeChannel<L>(pub L);
impl<L> Service<http::Uri> for MakeChannel<L>
where
    L: Layer<tonic::transport::Channel> + Clone + Send + 'static,
{
    type Response = L::Service;
    type Error = tonic::transport::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, destination: http::Uri) -> Self::Future {
        let layer = self.0.clone();
        Box::pin(async move {
            let channel = Channel::builder(destination).connect().await?;
            Ok(layer.layer(channel))
        })
    }
}
