use std::{
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use http::Uri;
use relentless::assault::service::origin_router::OriginRouter;
use tonic::{
    body::BoxBody,
    client::{Grpc, GrpcService},
    transport::{Body, Channel},
};
use tower::Service;

use crate::{client::DefaultGrpcRequest, error::GrpcClientError};

#[derive(Debug, Clone)]
pub struct DefaultGrpcClient<S, Req, Res> {
    inner: tonic::client::Grpc<S>,
    phantom: PhantomData<(Req, Res)>,
}

impl<B, Req, Res> DefaultGrpcClient<OriginRouter<tonic::transport::Channel, B>, Req, Res> {
    pub async fn new(all_destinations: &[Uri]) -> Result<Self, GrpcClientError> {
        let mut services = HashMap::new();
        for d in all_destinations {
            let channel = Channel::builder(d.clone()).connect().await.unwrap_or_else(|e| todo!("{}", e));
            services.insert(d.authority().unwrap_or_else(|| todo!()).clone(), channel);
        }
        Ok(Self { inner: tonic::client::Grpc::new(OriginRouter::new(services)), phantom: PhantomData })
    }
}

impl<S, Mq, Ms> Service<DefaultGrpcRequest<Mq, Ms>> for DefaultGrpcClient<S, Mq, Ms>
where
    S: GrpcService<BoxBody> + Send,
    S::ResponseBody: Send,
    <S::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send + 'static,
    Mq: Send,
    Ms: Send,
{
    type Response = tonic::Response<Ms>;
    type Error = GrpcClientError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(())) // TODO
    }

    fn call(&mut self, req: DefaultGrpcRequest<Mq, Ms>) -> Self::Future {
        Box::pin(async move {
            self.inner.ready().await.map_err(|_| GrpcClientError::Todo)?;
            self.inner
                .unary(tonic::Request::new(req.message), req.format_method_path(), req.codec)
                .await
                .map_err(|_| GrpcClientError::Todo)
        })
    }
}
