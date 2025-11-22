use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use http::uri::PathAndQuery;
use serde::{Deserializer, Serializer};
use tonic::{
    body::Body as BoxBody,
    client::GrpcService,
    transport::{Body, Channel},
    Status,
};
use tower::Service;

use crate::codec::DynamicCodec;

#[derive(Debug, Clone)]
pub struct GrpcClient<G>(G);

#[derive(Debug, Clone)]
pub struct GrpcChannel;
impl Service<http::Uri> for GrpcChannel {
    type Response = tonic::transport::Channel;
    type Error = tonic::transport::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, destination: http::Uri) -> Self::Future {
        Box::pin(async move {
            let channel = Channel::builder(destination).connect().await?;
            Ok(channel)
        })
    }
}

impl<G, D, S> Service<(tonic::Request<D>, PathAndQuery, DynamicCodec<D, S>)> for GrpcClient<G>
where
    G: GrpcService<BoxBody> + Clone + Send + 'static,
    G::ResponseBody: Send,
    <G::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    G::Future: Send + 'static,
    D: for<'a> Deserializer<'a> + Send + Sync + 'static,
    for<'a> <D as Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
    S: Serializer + Clone + Send + Sync + 'static,
    S::Ok: Send + Sync + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = tonic::Response<S::Ok>;
    type Error = Status;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: (tonic::Request<D>, PathAndQuery, DynamicCodec<D, S>)) -> Self::Future {
        let mut client = tonic::client::Grpc::new(self.0.clone());
        Box::pin(async move {
            let (request, path, codec) = req;
            client.ready().await.map_err(|e| Status::unknown(format!("Service was not ready: {}", e.into())))?; // ref https://github.com/hyperium/tonic/blob/v0.14.2/tonic-build/src/client.rs#L240-L242
            client.unary(request, path, codec).await
        })
    }
}
