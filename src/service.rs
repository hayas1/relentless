use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::{body::Body, client::conn::http1};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpStream, ToSocketAddrs};
use tower::Service;

use crate::{
    config::BodyStructure,
    error::{RelentlessError, RelentlessResult},
};

#[derive(Debug)]
pub struct DefaultHttpClient<ReqB, ResB> {
    sender: hyper::client::conn::http1::SendRequest<ReqB>,
    phantom: std::marker::PhantomData<ResB>,
}
impl<ReqB: Body + Send + 'static, ResB> DefaultHttpClient<ReqB, ResB>
where
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
{
    pub async fn new<A>(host: A) -> RelentlessResult<Self>
    where
        A: ToSocketAddrs,
    {
        let stream = TcpStream::connect(host).await?;
        let io = TokioIo::new(stream);
        let (sender, conn) = http1::handshake(io).await?;
        tokio::spawn(conn);
        let phantom = std::marker::PhantomData;
        Ok(Self { sender, phantom })
    }
}

impl<ReqB: Body + 'static, ResB: Body + 'static> Service<http::Request<ReqB>> for DefaultHttpClient<ReqB, ResB> {
    type Response = http::Response<BytesBody>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.sender.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<ReqB>) -> Self::Future {
        let fut = self.sender.send_request(req);
        Box::pin(async {
            match fut.await {
                Ok(r) => {
                    let (parts, incoming) = r.into_parts();
                    Ok(http::Response::from_parts(parts, incoming.into_bytes_body()))
                }
                Err(e) => Err(e),
            }
        })
    }
}

pub struct BytesBody(BoxBody<Bytes, RelentlessError>);
impl Body for BytesBody {
    type Data = Bytes;
    type Error = RelentlessError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }
    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
    fn size_hint(&self) -> http_body::SizeHint {
        self.0.size_hint()
    }
}
impl FromBodyStructure for BytesBody {
    fn from_body_structure(val: BodyStructure) -> Self {
        match val {
            BodyStructure::Empty => BytesBody(http_body_util::Empty::new().map_err(RelentlessError::from).boxed()),
        }
    }
}

pub trait FromBodyStructure {
    fn from_body_structure(val: BodyStructure) -> Self;
}
impl<T> FromBodyStructure for T
where
    T: Body + Default, // TODO other than Default
{
    fn from_body_structure(body: BodyStructure) -> Self {
        match body {
            BodyStructure::Empty => Default::default(),
        }
    }
}

pub trait IntoBytesBody {
    fn into_bytes_body(self) -> BytesBody;
}
impl<T> IntoBytesBody for T
where
    T: Body<Data = Bytes> + Send + Sync + 'static,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn into_bytes_body(self) -> BytesBody {
        BytesBody(self.map_err(|e| RelentlessError::from(e.into())).boxed())
    }
}
