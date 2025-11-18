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

use crate::codec::MethodCodec;

#[derive(Debug, Clone)]
pub struct GrpcClient<G>(tonic::client::Grpc<G>);

#[derive(Debug, Clone)]
pub struct GrpcChannel;
impl Service<http::Uri> for GrpcChannel {
    type Response = GrpcClient<tonic::transport::Channel>;
    type Error = tonic::transport::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, destination: http::Uri) -> Self::Future {
        Box::pin(async move {
            let channel = Channel::builder(destination).connect().await?;
            Ok(GrpcClient(tonic::client::Grpc::new(channel)))
        })
    }
}

// impl GrpcClient<tonic::transport::Channel> {
//     pub async fn new(destination: http::Uri) -> relentless::Result<Self> {
//         let channel = Channel::builder(destination).connect().await.unwrap_or_else(|e| todo!("{}", e));
//         Ok(Self(tonic::client::Grpc::new(channel)))
//     }
// }

// impl<S> GrpcClient<S>
// where
//     S: Clone,
// {
//     pub async fn from_services(services: &HashMap<Uri, S>) -> relentless::Result<Self> {
//         let clients = services.iter().map(|(d, s)| (d.clone(), tonic::client::Grpc::new(s.clone()))).collect();
//         Ok(Self { inner: clients })
//     }
// }

impl<G, D, S> Service<(tonic::Request<D>, PathAndQuery, MethodCodec<D, S>)> for GrpcClient<G>
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

    fn call(&mut self, req: (tonic::Request<D>, PathAndQuery, MethodCodec<D, S>)) -> Self::Future {
        let mut inner = self.0.clone();
        Box::pin(async move {
            let (request, path, codec) = req;
            inner.ready().await.map_err(|e| tonic::Status::unknown(format!("Service was not ready: {}", e.into())))?;
            inner.unary(request, path, codec).await
        })
    }
}
