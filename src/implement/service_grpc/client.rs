use std::{
    future::Future,
    marker::PhantomData,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use http::{uri::PathAndQuery, Uri};
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, MethodDescriptor, ServiceDescriptor};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use tonic::{
    codec::{Codec, Decoder, Encoder, ProstCodec},
    transport::Channel,
    Status,
};
use tower::Service;

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
        format!("/{}/{}", self.service.full_name(), self.method.name()).parse().unwrap_or_else(|e| todo!("{}", e))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct DefaultGrpcClient {}
impl<D> Service<DefaultGrpcRequest<D, serde_json::value::Serializer>> for DefaultGrpcClient
where
    D: for<'a> Deserializer<'a> + Send + Sync + 'static,
{
    type Response = tonic::Response<<serde_json::value::Serializer as Serializer>::Ok>;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: DefaultGrpcRequest<D, serde_json::value::Serializer>) -> Self::Future {
        let path = request.format_method_path();
        Box::pin(async move {
            let channel = Channel::builder(request.destination).connect().await.unwrap_or_else(|e| todo!("{}", e));
            let mut client = tonic::client::Grpc::new(channel);

            client.ready().await.unwrap_or_else(|e| todo!("{}", e));

            let response = client
                .unary(tonic::Request::new(request.message), path, request.codec)
                .await
                .unwrap_or_else(|e| todo!("{}", e));

            Ok(response)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodCodec<D, S> {
    method: MethodDescriptor,
    phantom: PhantomData<(D, S)>,
}
impl<D, S> MethodCodec<D, S> {
    pub fn new(method: MethodDescriptor) -> Self {
        Self { method, phantom: PhantomData }
    }
}

impl<D, S> Codec for MethodCodec<D, S>
where
    D: for<'a> Deserializer<'a> + Send + 'static,
{
    type Encode = D;
    type Decode = <serde_json::value::Serializer as Serializer>::Ok;
    type Encoder = MethodEncoder<D>;
    type Decoder = MethodDecoder<serde_json::value::Serializer>;

    fn encoder(&mut self) -> Self::Encoder {
        MethodEncoder(self.method.input(), PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        MethodDecoder(self.method.output(), PhantomData)
    }
}

#[derive(Debug)]
pub struct MethodEncoder<D>(MessageDescriptor, PhantomData<D>);
impl<D> Encoder for MethodEncoder<D>
where
    D: for<'a> Deserializer<'a>,
{
    type Item = D;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
        let Self(descriptor, _phantom) = self;
        DynamicMessage::deserialize(descriptor.clone(), item)
            .unwrap_or_else(|_| todo!())
            .encode(dst)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MethodDecoder<S>(MessageDescriptor, PhantomData<S>);
impl Decoder for MethodDecoder<serde_json::value::Serializer>
// where
//     S: Serializer + Send + 'static,
//     S::Ok: Send + 'static,
{
    type Item = <serde_json::value::Serializer as Serializer>::Ok;
    type Error = Status;

    fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }
        let Self(descriptor, _phantom) = self;
        let dynamic_message = DynamicMessage::decode(descriptor.clone(), src) // TODO `decode` requires ownership of MethodDescriptor
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(dynamic_message.serialize(serde_json::value::Serializer).unwrap_or_else(|_| todo!())))
    }
}
