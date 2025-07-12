use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Buf;
use http::{uri::PathAndQuery, Uri};
use prost::Message;
use prost_reflect::{DynamicMessage, MessageDescriptor, MethodDescriptor, ServiceDescriptor};
use serde::{Deserializer, Serialize, Serializer};
use tonic::{
    body::Body as BoxBody,
    client::GrpcService,
    codec::{Codec, Decoder, Encoder},
    transport::{Body, Channel},
    Status,
};
use tower::Service;

use crate::error::GrpcClientError;

#[derive(Debug, Clone, PartialEq)]
pub struct GrpcMethodRequest<D, S> {
    pub destination: http::Uri,
    pub service: ServiceDescriptor,
    pub method: MethodDescriptor,
    pub codec: MethodCodec<D, S>,
    pub message: D,
}
impl<D, S> GrpcMethodRequest<D, S> {
    pub fn format_method_path(&self) -> PathAndQuery {
        // https://github.com/hyperium/tonic/blob/master/tonic-build/src/lib.rs#L212-L218
        format!("/{}/{}", self.service.full_name(), self.method.name())
            .parse()
            .unwrap_or_else(|e| unreachable!("{}", e))
    }
}

#[derive(Debug, Clone)]
pub struct GrpcClient<S> {
    inner: HashMap<Uri, tonic::client::Grpc<S>>,
}

impl GrpcClient<tonic::transport::Channel> {
    pub async fn new(all_destinations: &[Uri]) -> Result<Self, GrpcClientError> {
        let mut clients = HashMap::new();
        for d in all_destinations {
            let channel = Channel::builder(d.clone()).connect().await.unwrap_or_else(|e| todo!("{}", e));
            clients.insert(d.clone(), tonic::client::Grpc::new(channel));
        }
        Ok(Self { inner: clients })
    }
}
impl<S> GrpcClient<S>
where
    S: Clone,
{
    pub async fn from_services(services: &HashMap<Uri, S>) -> Result<Self, GrpcClientError> {
        let clients = services.iter().map(|(d, s)| (d.clone(), tonic::client::Grpc::new(s.clone()))).collect();
        Ok(Self { inner: clients })
    }
}

impl<S, De, Se> Service<GrpcMethodRequest<De, Se>> for GrpcClient<S>
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

    fn call(&mut self, req: GrpcMethodRequest<De, Se>) -> Self::Future {
        let mut inner = self.inner[&req.destination].clone();
        Box::pin(async move {
            let path = req.format_method_path();
            let GrpcMethodRequest { codec, message, .. } = req;
            inner.ready().await.map_err(|_| GrpcClientError::Todo)?;
            inner.unary(tonic::Request::new(message), path, codec).await.map_err(|_| GrpcClientError::Todo)
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MethodCodec<D, S> {
    method: MethodDescriptor,
    serializer: S,
    phantom: PhantomData<(D, S)>,
}
impl<D, S: Clone> Clone for MethodCodec<D, S> {
    fn clone(&self) -> Self {
        Self { method: self.method.clone(), serializer: self.serializer.clone(), phantom: PhantomData }
    }
}
impl<D, S> MethodCodec<D, S> {
    pub fn new(method: MethodDescriptor, serializer: S) -> Self {
        Self { method, serializer, phantom: PhantomData }
    }
}

impl<D, S> Codec for MethodCodec<D, S>
where
    D: for<'a> Deserializer<'a> + Send + 'static,
    for<'a> <D as Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
    S: Serializer + Clone + Send + 'static,
    S::Ok: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Encode = D;
    type Decode = S::Ok;
    type Encoder = MethodEncoder<D>;
    type Decoder = MethodDecoder<S>;

    fn encoder(&mut self) -> Self::Encoder {
        MethodEncoder(self.method.input(), PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        MethodDecoder(self.method.output(), self.serializer.clone())
    }
}

#[derive(Debug)]
pub struct MethodEncoder<D>(MessageDescriptor, PhantomData<D>);
impl<D> Encoder for MethodEncoder<D>
where
    D: for<'a> Deserializer<'a>,
    for<'a> <D as Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
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
pub struct MethodDecoder<S>(MessageDescriptor, S);
impl<S> Decoder for MethodDecoder<S>
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
