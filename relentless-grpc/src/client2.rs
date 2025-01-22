use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use http::Uri;
use serde::{Deserializer, Serializer};
use tonic::{
    body::BoxBody,
    client::{Grpc, GrpcService},
    codec::Codec,
    transport::{Body, Channel},
};
use tower::Service;

use crate::{client::DefaultGrpcRequest, error::GrpcClientError};

#[derive(Debug)]
pub struct DefaultGrpcClient<S, De, Se> {
    inner: HashMap<Uri, tonic::client::Grpc<S>>,
    phantom: PhantomData<(De, Se)>,
}
impl<S: Clone, De, Se> Clone for DefaultGrpcClient<S, De, Se> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), phantom: PhantomData }
    }
}

impl<De, Se> DefaultGrpcClient<tonic::transport::Channel, De, Se> {
    pub async fn new(all_destinations: &[Uri]) -> Result<Self, GrpcClientError> {
        let mut clients = HashMap::new();
        for d in all_destinations {
            let channel = Channel::builder(d.clone()).connect().await.unwrap_or_else(|e| todo!("{}", e));
            clients.insert(d.clone(), tonic::client::Grpc::new(channel));
        }
        Ok(Self { inner: clients, phantom: PhantomData })
    }
}
impl<S, De, Se> DefaultGrpcClient<S, De, Se>
where
    S: Clone,
{
    pub async fn from_services(services: &HashMap<Uri, S>) -> Result<Self, GrpcClientError> {
        let clients = services.iter().map(|(d, s)| (d.clone(), tonic::client::Grpc::new(s.clone()))).collect();
        Ok(Self { inner: clients, phantom: PhantomData })
    }
}

impl<S, De, Se> Service<DefaultGrpcRequest<De, Se>> for DefaultGrpcClient<S, De, Se>
where
    S: GrpcService<BoxBody> + Clone + Send + 'static,
    S::ResponseBody: Send,
    <S::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
    De: for<'a> Deserializer<'a> + Send + Sync + 'static,
    for<'a> <De as Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
    Se: Serializer + Clone + Send + Sync + 'static,
    Se::Ok: Send + Sync + 'static,
    Se::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = tonic::Response<Se::Ok>;
    type Error = GrpcClientError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // TODO
    }

    fn call(&mut self, req: DefaultGrpcRequest<De, Se>) -> Self::Future {
        let mut inner = self.inner[&req.destination].clone();
        Box::pin(async move {
            let path = req.format_method_path();
            let DefaultGrpcRequest { codec, message, .. } = req;
            inner.ready().await.map_err(|_| GrpcClientError::Todo)?;
            inner.unary(tonic::Request::new(message), path, codec).await.map_err(|_| GrpcClientError::Todo)
        })
    }
}
