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
pub struct DefaultGrpcRequest<E, D> {
    pub uri: http::Uri,
    pub service: ServiceDescriptor,
    pub method: MethodDescriptor,
    pub codec: MethodCodec<E, D>,
    pub message: E,
}
impl<E, D> DefaultGrpcRequest<E, D> {
    pub fn format_method_path(&self) -> PathAndQuery {
        // https://github.com/hyperium/tonic/blob/master/tonic-build/src/lib.rs#L212-L218
        format!("/{}/{}", self.service.full_name(), self.method.name()).parse().unwrap_or_else(|e| todo!("{}", e))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct DefaultGrpcClient {}
impl<E, D> Service<DefaultGrpcRequest<E, D>> for DefaultGrpcClient
where
    E: for<'a> Deserializer<'a> + Send + Sync + 'static,
    D: Serializer<Ok = serde_json::Value> + Send + 'static,
{
    type Response = tonic::Response<D::Ok>;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: DefaultGrpcRequest<E, D>) -> Self::Future {
        let path = request.format_method_path();
        Box::pin(async move {
            let channel = Channel::builder(request.uri).connect().await.unwrap_or_else(|e| todo!("{}", e));
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
pub struct MethodCodec<E, D> {
    method: MethodDescriptor,
    phantom: PhantomData<(E, D)>,
}
impl<E, D> MethodCodec<E, D> {
    pub fn new(method: MethodDescriptor) -> Self {
        Self { method, phantom: PhantomData }
    }
}

impl<E, D> Codec for MethodCodec<E, D>
where
    E: for<'a> Deserializer<'a> + Send + 'static,
    D: Serializer<Ok = serde_json::Value> + Send + 'static,
{
    type Encode = E;
    type Decode = D::Ok;
    type Encoder = MethodEncoder<E>;
    type Decoder = MethodDecoder<D>;

    fn encoder(&mut self) -> Self::Encoder {
        MethodEncoder(self.method.input(), PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        MethodDecoder(self.method.output(), PhantomData)
    }
}

#[derive(Debug)]
pub struct MethodEncoder<E>(MessageDescriptor, PhantomData<E>);
impl<E> Encoder for MethodEncoder<E>
where
    E: for<'a> Deserializer<'a>,
{
    type Item = E;
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
pub struct MethodDecoder<D>(MessageDescriptor, PhantomData<D>);
impl<D> Decoder for MethodDecoder<D>
where
    D: Serializer<Ok = serde_json::Value>,
{
    type Item = D::Ok;
    type Error = Status;

    fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }
        let Self(descriptor, _phantom) = self;
        let dynamic_message = DynamicMessage::decode(descriptor.clone(), src) // TODO `decode` requires ownership of MethodDescriptor
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        // TODO decoder should have Serializer instance ?
        Ok(Some(dynamic_message.serialize(serde_json::value::Serializer).unwrap_or_else(|_| todo!())))
    }
}
