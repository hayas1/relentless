use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use http::{uri::PathAndQuery, Uri};
use prost_reflect::{MethodDescriptor, ServiceDescriptor};
use relentless::assault::service::origin_router::OriginRouter;
use serde::Deserializer;
use tonic::{
    body::BoxBody,
    client::{Grpc, GrpcService},
    codec::Codec,
    transport::{Body, Channel},
};
use tower::Service;

use crate::error::GrpcClientError;

#[derive(Debug, Clone)]
pub struct DefaultGrpcClient<S, Mq> {
    inner: HashMap<Uri, tonic::client::Grpc<S>>,
    phantom: PhantomData<Mq>,
}

// TODO tonic::client::Grpc has 1 origin, so OriginRouter will not work properly
impl<Mq> DefaultGrpcClient<tonic::transport::Channel, Mq> {
    pub async fn new(all_destinations: &[Uri]) -> Result<Self, GrpcClientError> {
        let mut clients = HashMap::new();
        for d in all_destinations {
            let channel = Channel::builder(d.clone()).connect().await.unwrap_or_else(|e| todo!("{}", e));
            clients.insert(d.clone(), tonic::client::Grpc::new(channel));
        }
        Ok(Self { inner: clients, phantom: PhantomData })
    }
}

impl<S, Mq, C> Service<DefaultGrpcRequest2<C, Mq>> for DefaultGrpcClient<S, Mq>
where
    S: GrpcService<BoxBody> + Clone + Send + 'static,
    S::ResponseBody: Send,
    <S::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
    Mq: Send + Sync + 'static,
    C: Codec<Encode = Mq> + Send + 'static,
    C::Decode: Send + Sync + 'static,
{
    type Response = tonic::Response<C::Decode>;
    type Error = GrpcClientError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // TODO
    }

    fn call(&mut self, req: DefaultGrpcRequest2<C, Mq>) -> Self::Future {
        let mut inner = self.inner[&req.destination].clone();
        Box::pin(async move {
            let path = req.format_method_path();
            let DefaultGrpcRequest2 { codec, message, .. } = req;
            inner.ready().await.map_err(|_| GrpcClientError::Todo)?;
            inner.unary(tonic::Request::new(message), path, codec).await.map_err(|_| GrpcClientError::Todo)
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefaultGrpcRequest2<C, M> {
    pub destination: Uri,
    pub service: ServiceDescriptor,
    pub method: MethodDescriptor,
    pub codec: C,
    pub message: M,
}
impl<C, M> DefaultGrpcRequest2<C, M> {
    pub fn format_method_path(&self) -> PathAndQuery {
        // https://github.com/hyperium/tonic/blob/master/tonic-build/src/lib.rs#L212-L218
        format!("/{}/{}", self.service.full_name(), self.method.name())
            .parse()
            .unwrap_or_else(|e| unreachable!("{}", e))
    }
}
