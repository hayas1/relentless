use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Buf;
use http::uri::PathAndQuery;
use prost::Message;
use prost_reflect::{DynamicMessage, MessageDescriptor, MethodDescriptor};
use serde::{Deserializer, Serialize, Serializer};
use tonic::{
    body::Body as BoxBody,
    client::GrpcService,
    codec::{Codec, Decoder, Encoder},
    transport::{Body, Channel},
    Status,
};
use tower::Service;

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
    type Error = tonic::Status;
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
