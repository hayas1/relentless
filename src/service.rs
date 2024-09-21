use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{
    body::{Body, Incoming},
    client::conn::http1,
};
use hyper_util::rt::TokioIo;
use tokio::{
    net::{TcpStream, ToSocketAddrs},
    runtime::Runtime,
};
use tower::Service;

use crate::error::RelentlessResult;

#[derive(Debug)]
pub struct HyperClient<B> {
    sender: hyper::client::conn::http1::SendRequest<B>,
}
impl<B: Body + Send + 'static> HyperClient<B>
where
    B::Data: Send + 'static,
    B::Error: std::error::Error + Sync + Send + 'static,
{
    pub async fn new<A>(origin: A) -> RelentlessResult<Self>
    where
        A: ToSocketAddrs,
    {
        let stream = TcpStream::connect(origin).await?;
        let io = TokioIo::new(stream);
        let (sender, conn) = http1::handshake(io).await?;
        tokio::spawn(conn);
        Ok(Self { sender })
    }
}
impl<B: Body + 'static> HyperClient<B> {
    pub async fn send_request(&mut self, req: http::Request<B>) -> Result<http::Response<Bytes>, hyper::Error> {
        let response = self.sender.send_request(req).await?;
        let (parts, incoming) = response.into_parts();
        let body = BodyExt::collect(incoming).await.map(|buf| buf.to_bytes());
        let response = http::Response::from_parts(parts, body?);
        Ok(response)
    }
}
impl<B: Body + Send + 'static> Clone for HyperClient<B>
where
    B::Data: Send + 'static,
    B::Error: std::error::Error + Sync + Send + 'static,
{
    fn clone(&self) -> Self {
        // TODO
        let f = Self::new("http://localhost:3000");
        Runtime::new().unwrap().block_on(f).unwrap()
    }
}

impl<B: Body + 'static> Service<http::Request<B>> for HyperClient<B> {
    type Response = http::Response<Incoming>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        Box::pin(self.sender.send_request(req))
    }
}
