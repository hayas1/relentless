use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::{Bytes, BytesMut};
use tokio::net::TcpStream;
use tower::Service;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct DefaultGrpcClient {}
impl Service<http::Request<Bytes>> for DefaultGrpcClient {
    type Response = http::Response<Bytes>;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<Bytes>) -> Self::Future {
        let (parts, request_body) = req.into_parts();
        Box::pin(async {
            // let uri = "http://127.0.0.1:50051".parse::<http::Uri>()?;
            let tcp = TcpStream::connect("127.0.0.1:50051").await.unwrap_or_else(|_| todo!());
            let (client, connection) = h2::client::handshake(tcp).await.unwrap_or_else(|_| todo!());
            tokio::spawn(async move {
                connection.await.unwrap_or_else(|_| todo!());
            });

            let r = http::Request::<()>::from_parts(parts, ());
            let (response, mut send) =
                client.ready().await.unwrap_or_else(|_| todo!()).send_request(r, false).unwrap_or_else(|_| todo!());

            send.send_data(request_body, false).unwrap_or_else(|e| todo!("{}", e));
            let (head, mut recieve) = response.await.unwrap_or_else(|_| todo!()).into_parts();
            let mut body = BytesMut::new();
            let mut flow_control = recieve.flow_control().clone();
            while let Some(chunk) = recieve.data().await {
                dbg!(&chunk);
                let chunk = chunk.unwrap_or_else(|e| todo!("{}", e));
                body.extend(&chunk);
                println!("RX: {:?}", chunk);

                let _ = flow_control.release_capacity(chunk.len());
            }
            send.send_data(Bytes::new(), true).unwrap_or_else(|e| todo!("{}", e));

            println!("{}", String::from_utf8_lossy(&body));
            Ok(http::Response::from_parts(head, body.freeze()))
        })
    }
}
