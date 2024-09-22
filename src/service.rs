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
pub struct HyperClient<ReqB, ResB> {
    sender: hyper::client::conn::http1::SendRequest<ReqB>,
    phantom: std::marker::PhantomData<ResB>,
}
impl<ReqB: Body + Send + 'static, ResB> HyperClient<ReqB, ResB>
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
impl<ReqB: Body + Send + 'static, ResB> Clone for HyperClient<ReqB, ResB>
where
    ReqB::Data: Send + 'static,
    ReqB::Error: std::error::Error + Sync + Send + 'static,
{
    fn clone(&self) -> Self {
        // TODO
        todo!();
        let f = Self::new("http://localhost:3000");
        Runtime::new().unwrap().block_on(f).unwrap()
    }
}

impl<ReqB: Body + 'static, ResB: From<Bytes>> Service<http::Request<ReqB>> for HyperClient<ReqB, ResB> {
    type Response = http::Response<ResB>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<ReqB>) -> Self::Future {
        let fut = self.sender.send_request(req);
        Box::pin(async {
            match fut.await {
                Ok(r) => {
                    let (parts, incoming) = r.into_parts();
                    let body = BodyExt::collect(incoming).await.map(|buf| buf.to_bytes())?;
                    Ok(http::Response::from_parts(parts, body.into()))
                }
                Err(e) => Err(e),
            }
        })
    }
}

// TODO stream ?
// impl<ReqB: Body + 'static, ResB: From<Incoming>> Service<http::Request<ReqB>> for HyperClient<ReqB, ResB> {
//     type Response = http::Response<ResB>;
//     type Error = hyper::Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
//
//     fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }
//
//     fn call(&mut self, req: http::Request<ReqB>) -> Self::Future {
//         let fut = self.sender.send_request(req);
//         Box::pin(async {
//             fut.await.map(|r| {
//                 let (parts, incoming) = r.into_parts();
//                 http::Response::from_parts(parts, incoming.into())
//             })
//         })
//     }
// }
