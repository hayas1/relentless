use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use http::Uri;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use tonic::{
    codec::{Codec, Decoder, Encoder, ProstCodec},
    transport::Channel,
    Status,
};
use tower::Service;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct DefaultGrpcClient {}
impl Service<http::Request<Value>> for DefaultGrpcClient {
    type Response = tonic::Response<Value>;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Value>) -> Self::Future {
        let (parts, request_body) = req.into_parts();
        let path = parts.uri.path_and_query().unwrap_or_else(|| todo!()).clone();
        Box::pin(async move {
            let channel =
                Channel::from_static("http://127.0.0.1:50051").connect().await.unwrap_or_else(|e| todo!("{}", e));
            let mut client = tonic::client::Grpc::new(channel);

            let request = tonic::Request::new(request_body);
            client.ready().await.unwrap_or_else(|e| todo!("{}", e));

            let response = client
                .unary(request, path, JsonCodec(PhantomData::<(Value, Value)>))
                .await
                .unwrap_or_else(|e| todo!("{}", e));

            Ok(response)
        })
    }
}

#[derive(Debug, Default)]
pub struct JsonCodec<E, D>(PhantomData<(E, D)>);

impl<E, D> Codec for JsonCodec<E, D>
where
    E: Serialize + Send + 'static,
    D: DeserializeOwned + Send + 'static,
{
    type Encode = E;
    type Decode = D;
    type Encoder = JsonEncoder<E>;
    type Decoder = JsonDecoder<D>;

    fn encoder(&mut self) -> Self::Encoder {
        JsonEncoder(PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        JsonDecoder(PhantomData)
    }
}

#[derive(Debug, Default)]
pub struct JsonEncoder<E>(PhantomData<E>);

impl<E: Serialize> Encoder for JsonEncoder<E> {
    type Item = E;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut tonic::codec::EncodeBuf<'_>) -> Result<(), Self::Error> {
        serde_json::to_writer(dst.writer(), &item).map_err(|e| Status::internal(e.to_string()))
    }
}

#[derive(Debug, Default)]
pub struct JsonDecoder<D>(PhantomData<D>);

impl<D: DeserializeOwned> Decoder for JsonDecoder<D> {
    type Item = D;
    type Error = Status;

    fn decode(&mut self, src: &mut tonic::codec::DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        if !src.has_remaining() {
            return Ok(None);
        }

        let item = serde_json::from_reader(src.reader()).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Some(item))
    }
}
