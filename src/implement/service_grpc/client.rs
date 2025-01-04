use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use http::{uri::PathAndQuery, Uri};
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use tonic::{
    codec::{Codec, Decoder, Encoder, ProstCodec},
    transport::Channel,
    Status,
};
use tower::Service;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct DefaultGrpcClient {}
impl Service<tonic::Request<DynamicMessage>> for DefaultGrpcClient {
    type Response = tonic::Response<DynamicMessage>;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: tonic::Request<DynamicMessage>) -> Self::Future {
        let path = PathAndQuery::from_static("/greeter.Greeter/SayHello");
        Box::pin(async move {
            let channel =
                Channel::from_static("http://127.0.0.1:50051").connect().await.unwrap_or_else(|e| todo!("{}", e));
            let mut client = tonic::client::Grpc::new(channel);

            client.ready().await.unwrap_or_else(|e| todo!("{}", e));

            let response = client
                .unary(req, path, DynamicCodec)
                // .unary(req, path, tonic::codec::ProstCodec::default())
                .await
                .unwrap_or_else(|e| todo!("{}", e));

            Ok(response)
        })
    }
}

#[derive(Debug, Default)]
pub struct DynamicCodec;

impl Codec for DynamicCodec {
    type Encode = DynamicMessage;
    type Decode = DynamicMessage;
    type Encoder = DynamicEncoder;
    type Decoder = DynamicDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        DynamicEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        DynamicDecoder
    }
}

#[derive(Debug, Default)]
pub struct DynamicEncoder;
impl Encoder for DynamicEncoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(dst).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DynamicDecoder;
impl Decoder for DynamicDecoder {
    type Item = DynamicMessage;
    type Error = Status;

    fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }
        dbg!("// TODO schema info");
        let pool = DescriptorPool::decode(
            include_bytes!(
                "../../../target/debug/build/relentless-dev-server-grpc-966e593a5a4fc2ae/out/file_descriptor.bin"
            )
            .as_ref(),
        )
        .unwrap();
        let message_descriptor = pool.get_message_by_name("greeter.HelloResponse").unwrap_or_else(|| todo!());
        let dynamic_message = DynamicMessage::decode(message_descriptor, src)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(dynamic_message))
    }
}
