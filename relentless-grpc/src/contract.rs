use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use http::uri::PathAndQuery;
use prost_reflect::DescriptorPool;
use relentless::shot::contract::{Contract, SignContract};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tonic::{client::GrpcService, Status};
use tower::{Layer, Service};

use crate::{codec::DynamicCodec, request::GrpcRequest, response::GrpcResponse, wip::JsonSerializer};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum GrpcDescriptor {
    Protos {
        #[serde(default)]
        protos: Vec<PathBuf>,
        #[serde(default)]
        import_path: Vec<PathBuf>,
    },
    Bin(PathBuf),
    #[default]
    Reflection,
}
impl<G, D, S> SignContract<G, DynamicContract<D, S>> for GrpcDescriptor
where
    G: GrpcService<tonic::body::Body> + Clone + Send + 'static,
{
    type Error = Infallible;
    async fn sign_contract(&self, service: G) -> Result<DynamicContract<D, S>, Self::Error> {
        let mut descriptor_bytes = Vec::new();
        // File::open(path)?.read_to_end(&mut descriptor_bytes)?;
        Ok(DynamicContract {
            pool: DescriptorPool::decode(Bytes::from(descriptor_bytes)).unwrap(),
            phantom: PhantomData,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicContract<D, S> {
    pool: DescriptorPool,
    phantom: PhantomData<(D, S)>,
}
impl<G, D, S> Layer<G> for DynamicContract<D, S> {
    type Service = DynamicService<G, D, S>;

    fn layer(&self, service: G) -> Self::Service {
        DynamicService { pool: self.pool.clone(), service, phantom: PhantomData }
    }
}
impl<G: Send, D: Send, S: Send> Contract<G> for DynamicContract<D, S>
where
    G: GrpcService<tonic::body::Body> + Clone + Send + 'static,
    G::ResponseBody: Send,
    <G::ResponseBody as tonic::transport::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    G::Future: Send + 'static,
    D: for<'x> Deserializer<'x> + Send + Sync + 'static,
    for<'x> <D as Deserializer<'x>>::Error: std::error::Error + Send + Sync + 'static,
{
    type Sign = GrpcDescriptor;
    type ReqSource = GrpcRequest;
    type Request = (PathAndQuery, tonic::Request<D>);
    type TransportReq = http::Request<tonic::body::Body>;
    type TransportRes = http::Response<tonic::body::Body>;
    type Response = tonic::Response<<JsonSerializer as Serializer>::Ok>;
    type ResSink = GrpcResponse;

    type SignError = Infallible;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicService<G, D, S> {
    pool: DescriptorPool,
    service: G,
    phantom: PhantomData<(D, S)>,
}
impl<G, D, S> Service<(PathAndQuery, tonic::Request<D>)> for DynamicService<G, D, S>
where
    G: GrpcService<tonic::body::Body> + Clone + Send + 'static,
    G::ResponseBody: Send,
    <G::ResponseBody as tonic::transport::Body>::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    G::Future: Send + 'static,
    D: for<'x> Deserializer<'x> + Send + Sync + 'static,
    for<'x> <D as Deserializer<'x>>::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = tonic::Response<<JsonSerializer as Serializer>::Ok>;
    type Error = Status;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: (PathAndQuery, tonic::Request<D>)) -> Self::Future {
        let mut grpc = tonic::client::Grpc::new(self.service.clone());
        let (target, request) = req;
        let codec = DynamicCodec::with_pool(self.pool.clone(), &target, JsonSerializer::default()).unwrap();
        Box::pin(async move {
            grpc.ready().await.map_err(|e| Status::unknown(format!("Service was not ready: {}", e.into())))?; // ref https://github.com/hyperium/tonic/blob/v0.14.2/tonic-build/src/client.rs#L240-L242
            grpc.unary(request, target, codec).await
        })
    }
}
