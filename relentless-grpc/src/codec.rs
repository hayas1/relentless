use std::marker::PhantomData;

use bytes::Buf;
use prost_reflect::{prost::Message, DynamicMessage, MessageDescriptor, MethodDescriptor};
use serde::{Deserializer, Serialize, Serializer};
use tonic::{
    codec::{Codec, Decoder, Encoder},
    Status,
};

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
