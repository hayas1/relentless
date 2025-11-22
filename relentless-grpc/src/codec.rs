use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, Bytes};
use http::uri::PathAndQuery;
use prost_reflect::{prost::Message, DescriptorPool, DynamicMessage, MessageDescriptor, MethodDescriptor};
use relentless::shot::contract::Contract;
use serde::{Deserializer, Serialize, Serializer};
use tonic::{
    client::GrpcService,
    codec::{Codec, Decoder, Encoder},
    Status,
};
use tower::{Layer, Service};

use crate::{request::GrpcRequest, response::GrpcResponse, wip::JsonSerializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorLayer<D, S> {
    pool: DescriptorPool,
    phantom: PhantomData<(D, S)>,
}
impl<G, D, S> Layer<G> for DescriptorLayer<D, S> {
    type Service = DescriptorService<G, D, S>;

    fn layer(&self, service: G) -> Self::Service {
        DescriptorService { pool: self.pool.clone(), service, phantom: PhantomData }
    }
}
impl<G: Send, D: Send, S: Send> Contract<G> for DescriptorLayer<D, S>
where
    G: GrpcService<tonic::body::Body> + Clone + Send + 'static,
    G::ResponseBody: Send,
    <G::ResponseBody as tonic::transport::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    G::Future: Send + 'static,
    D: for<'x> Deserializer<'x> + Send + Sync + 'static,
    for<'x> <D as Deserializer<'x>>::Error: std::error::Error + Send + Sync + 'static,
{
    type ReqSource = GrpcRequest;
    type Request = (PathAndQuery, tonic::Request<D>);
    type TransportReq = http::Request<tonic::body::Body>;
    type ResSink = GrpcResponse;
    type Response = tonic::Response<S>;
    type TransportRes = http::Response<tonic::body::Body>;
    type ServiceError = tonic::Status;
    type Error = Status;

    async fn new(service: G, request: &GrpcRequest) -> Result<Self, Self::Error> {
        let mut descriptor_bytes = Vec::new();
        // File::open(path)?.read_to_end(&mut descriptor_bytes)?;
        Ok(Self { pool: DescriptorPool::decode(Bytes::from(descriptor_bytes)).unwrap(), phantom: PhantomData })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorService<G, D, S> {
    pool: DescriptorPool,
    service: G,
    phantom: PhantomData<(D, S)>,
}
impl<G, D, S> Service<(PathAndQuery, tonic::Request<D>)> for DescriptorService<G, D, S>
where
    G: GrpcService<tonic::body::Body> + Clone + Send + 'static,
    G::ResponseBody: Send,
    <G::ResponseBody as tonic::transport::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    G::Future: Send + 'static,
    D: for<'x> Deserializer<'x> + Send + Sync + 'static,
    for<'x> <D as Deserializer<'x>>::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = tonic::Response<<JsonSerializer as Serializer>::Ok>;
    type Error = Status;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: (PathAndQuery, tonic::Request<D>)) -> Self::Future {
        let mut grpc = tonic::client::Grpc::new(self.service.clone());
        let (target, request) = req;
        let method = get_method(self.pool.clone(), &target).unwrap();
        let codec = DynamicCodec::new(method, JsonSerializer::default());
        Box::pin(async move {
            grpc.ready().await.map_err(|e| Status::unknown(format!("Service was not ready: {}", e.into())))?; // ref https://github.com/hyperium/tonic/blob/v0.14.2/tonic-build/src/client.rs#L240-L242
            grpc.unary(request, target, codec).await
        })
    }
}
fn get_method(pool: DescriptorPool, target: &PathAndQuery) -> Option<MethodDescriptor> {
    let (svc, mtd) = target.as_str().split_once('/')?;
    let service = pool.get_service_by_name(svc)?;
    let method = service.methods().find(|m| m.name() == mtd)?;
    Some(method)
}

#[derive(Debug, PartialEq, Eq)]
pub struct DynamicCodec<D, S> {
    method: MethodDescriptor,
    serializer: S,
    phantom: PhantomData<(D, S)>,
}
impl<D, S: Clone> Clone for DynamicCodec<D, S> {
    fn clone(&self) -> Self {
        Self { method: self.method.clone(), serializer: self.serializer.clone(), phantom: PhantomData }
    }
}
impl<D, S> DynamicCodec<D, S> {
    pub fn new(method: MethodDescriptor, serializer: S) -> Self {
        Self { method, serializer, phantom: PhantomData }
    }
}

impl<D, S> Codec for DynamicCodec<D, S>
where
    D: for<'x> Deserializer<'x> + Send + 'static,
    for<'x> <D as Deserializer<'x>>::Error: std::error::Error + Send + Sync + 'static,
    S: Serializer + Clone + Send + 'static,
    S::Ok: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Encode = D;
    type Decode = S::Ok;
    type Encoder = DynamicEncoder<D>;
    type Decoder = DynamicDecoder<S>;

    fn encoder(&mut self) -> Self::Encoder {
        DynamicEncoder(self.method.input(), PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        DynamicDecoder(self.method.output(), self.serializer.clone())
    }
}

#[derive(Debug)]
pub struct DynamicEncoder<D>(MessageDescriptor, PhantomData<D>);
impl<D> Encoder for DynamicEncoder<D>
where
    D: for<'x> Deserializer<'x>,
    for<'x> <D as Deserializer<'x>>::Error: std::error::Error + Send + Sync + 'static,
{
    type Item = D;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
        let Self(descriptor, _phantom) = self;
        DynamicMessage::deserialize(descriptor.clone(), item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            .encode(dst)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DynamicDecoder<S>(MessageDescriptor, S);
impl<S> Decoder for DynamicDecoder<S>
where
    S: Serializer + Clone + Send + 'static,
    S::Ok: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Item = S::Ok;
    type Error = Status;

    fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }
        let Self(descriptor, serializer) = self;
        let dynamic_message = DynamicMessage::decode(descriptor.clone(), src) // TODO `decode` requires ownership of MethodDescriptor
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(
            dynamic_message
                .serialize(serializer.clone())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        ))
    }
}
