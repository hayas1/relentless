use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Buf;
use http::uri::PathAndQuery;
use prost::Message;
use prost_reflect::{DynamicMessage, MessageDescriptor, MethodDescriptor, ServiceDescriptor};
use serde::{Deserializer, Serialize, Serializer};
use tonic::{
    codec::{Codec, Decoder, Encoder},
    transport::Channel,
    Status,
};
use tower::Service;

use relentless::error::IntoResult;

#[derive(Debug, Clone, PartialEq)]
pub struct DefaultGrpcRequest<D, S> {
    pub destination: http::Uri,
    pub service: ServiceDescriptor,
    pub method: MethodDescriptor,
    pub codec: MethodCodec<D, S>,
    pub message: D,
}
impl<D, S> DefaultGrpcRequest<D, S> {
    pub fn format_method_path(&self) -> PathAndQuery {
        // https://github.com/hyperium/tonic/blob/master/tonic-build/src/lib.rs#L212-L218
        format!("/{}/{}", self.service.full_name(), self.method.name())
            .parse()
            .unwrap_or_else(|e| unreachable!("{}", e))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct DefaultGrpcClient<D> {
    phantom: PhantomData<D>,
}
impl<D> DefaultGrpcClient<D> {
    pub fn new() -> Self {
        Self { phantom: PhantomData }
    }
}
// impl<D> Service<DefaultGrpcRequest<D, serde_json::value::Serializer>> for DefaultGrpcClient<D>
// where
//     D: for<'a> Deserializer<'a> + Send + Sync + 'static,
//     for<'a> <D as Deserializer<'a>>::Error: std::error::Error + Send + Sync + 'static,
// {
//     type Response = tonic::Response<<serde_json::value::Serializer as Serializer>::Ok>;
//     type Error = relentless::Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn call(&mut self, request: DefaultGrpcRequest<D, serde_json::value::Serializer>) -> Self::Future {
//         let path = request.format_method_path();
//         Box::pin(async move {
//             let mut client = tonic::client::Grpc::new(Channel::builder(request.destination).connect().await.box_err()?);
//             client.ready().await.box_err()?;
//             client.unary(tonic::Request::new(request.message), path, request.codec).await.box_err()
//         })
//     }
// }
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
