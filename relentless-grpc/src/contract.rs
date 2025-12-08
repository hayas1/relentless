use std::{
    convert::Infallible,
    fs::File,
    future::Future,
    io::Read,
    marker::PhantomData,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use http::uri::PathAndQuery;
use prost::Message;
use prost_reflect::DescriptorPool;
use prost_types::FileDescriptorProto;
use relentless::shot::contract::{Contract, SignContract};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tonic::{client::GrpcService, transport::Body, Status};
use tonic_reflection::pb::v1::{
    server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse, FileDescriptorResponse, ServerReflectionRequest, ServiceResponse,
};
use tower::{Layer, Service};

use crate::{codec::DynamicCodec, request::GrpcRequest, response::GrpcResponse, wip::JsonSerializer};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcDescriptor {
    ProtoFiles {
        #[serde(default)]
        protos: Vec<PathBuf>,
        #[serde(default)]
        includes: Vec<PathBuf>,
    },
    Bin(PathBuf),
    #[default]
    Reflection,
}
impl<G, D, S> SignContract<G, DynamicContract<D, S>> for GrpcDescriptor
where
    G: Clone + GrpcService<tonic::body::Body> + Send + Sync + 'static,
    G::Future: Send,
    G::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <G::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>> + Send,
{
    type Error = relentless::Error;
    async fn sign_contract(&self, service: G, destination: &http::Uri) -> Result<DynamicContract<D, S>, Self::Error> {
        let pool = match self {
            Self::ProtoFiles { protos, includes } => Self::from_protos(protos, includes)?,
            Self::Bin(path) => Self::descriptor_from_file(path)?,
            Self::Reflection => Self::from_reflection(service, destination).await?,
        };
        Ok(DynamicContract { pool, phantom: PhantomData })
    }
}
impl GrpcDescriptor {
    pub fn from_protos(protos: &[PathBuf], includes: &[PathBuf]) -> relentless::Result<DescriptorPool> {
        let builder = &mut prost_build::Config::new();
        let fds = builder.load_fds(protos, includes).map_err(relentless::Error::boxed)?;
        DescriptorPool::from_file_descriptor_set(fds).map_err(relentless::Error::boxed)
    }
    pub fn descriptor_from_file(path: &PathBuf) -> relentless::Result<DescriptorPool> {
        let mut descriptor_bytes = Vec::new();
        File::open(path)
            .map_err(relentless::Error::boxed)?
            .read_to_end(&mut descriptor_bytes)
            .map_err(relentless::Error::boxed)?;
        DescriptorPool::decode(Bytes::from(descriptor_bytes)).map_err(relentless::Error::boxed)
    }
    pub async fn from_reflection<G>(service: G, destination: &http::Uri) -> relentless::Result<DescriptorPool>
    where
        G: Clone + GrpcService<tonic::body::Body> + Send + Sync + 'static,
        G::Future: Send,
        G::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <G::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>> + Send,
    {
        let mut client = ServerReflectionClient::new(service);
        let services = Self::reflection_services(&mut client, destination).await?;
        let pool = Self::reflection_file_descriptors(&mut client, destination, services).await?;
        Ok(pool)
    }
    pub async fn reflection_services<G>(
        client: &mut ServerReflectionClient<G>,
        destination: &http::Uri,
    ) -> relentless::Result<Vec<ServiceResponse>>
    where
        G: Clone + GrpcService<tonic::body::Body> + Send + Sync + 'static,
        G::Future: Send,
        G::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <G::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>> + Send,
    {
        let host = destination.host().unwrap_or_else(|| todo!()).to_string();
        let request_stream = futures::stream::once({
            let host = host.clone();
            async move {
                ServerReflectionRequest {
                    host,
                    message_request: Some(MessageRequest::ListServices(Default::default())),
                }
            }
        });
        let streaming = client.server_reflection_info(request_stream).await.unwrap_or_else(|_| todo!()).into_inner();
        let services = streaming
            .try_fold(Vec::new(), |_, recv| async move {
                match recv.message_response.unwrap_or_else(|| todo!()) {
                    MessageResponse::ListServicesResponse(list) => Ok(list.service),
                    _ => todo!(),
                }
            })
            .await
            .unwrap_or_else(|_| todo!());
        Ok(services)
    }
    pub async fn reflection_file_descriptors<G>(
        client: &mut ServerReflectionClient<G>,
        destination: &http::Uri,
        services: Vec<ServiceResponse>,
    ) -> relentless::Result<DescriptorPool>
    where
        G: Clone + GrpcService<tonic::body::Body> + Send + Sync + 'static,
        G::Future: Send,
        G::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <G::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>> + Send,
    {
        let buffer = services.len();
        let host = destination.host().unwrap_or_else(|| todo!()).to_string();
        let request_stream = futures::stream::iter(services)
            .map(move |service| {
                let host = host.clone();
                async move {
                    ServerReflectionRequest {
                        host,
                        message_request: Some(MessageRequest::FileContainingSymbol(service.name)),
                    }
                }
            })
            .buffer_unordered(buffer);
        let streaming = client.server_reflection_info(request_stream).await.unwrap_or_else(|_| todo!()).into_inner();
        let descriptors = streaming
            .try_fold(DescriptorPool::new(), move |mut pool, recv| {
                let client = client.clone();
                async move {
                    match recv.message_response.unwrap_or_else(|| todo!()) {
                        MessageResponse::FileDescriptorResponse(fd_resp) => {
                            Self::reflection_file_descriptors_recursive(client, destination, &mut pool, fd_resp)
                                .await
                                .unwrap_or_else(|e| todo!("{e}"));
                        }
                        _ => todo!(),
                    };
                    Ok(pool)
                }
            })
            .await
            .unwrap_or_else(|e| todo!("{e}"));
        Ok(descriptors)
    }
    pub fn reflection_file_descriptors_recursive<'a, G>(
        client: ServerReflectionClient<G>,
        destination: &'a http::Uri,
        pool: &'a mut DescriptorPool,
        fd_resp: FileDescriptorResponse,
    ) -> Pin<Box<dyn 'a + Future<Output = relentless::Result<&'a mut DescriptorPool>>>>
    where
        G: Clone + GrpcService<tonic::body::Body> + Send + Sync + 'static,
        G::Future: Send,
        G::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <G::ResponseBody as Body>::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>> + Send,
    {
        let host = destination.host().unwrap_or_else(|| todo!()).to_string();
        Box::pin(async move {
            let deps: Vec<_> = fd_resp
                .file_descriptor_proto
                .into_iter()
                .flat_map(|raw| {
                    let fd_proto = FileDescriptorProto::decode(&*raw).unwrap_or_else(|_| todo!());
                    match pool.add_file_descriptor_proto(fd_proto.clone()) {
                        Ok(()) => Vec::new(),
                        Err(_) => fd_proto.dependency,
                    }
                })
                .collect();
            let buffer = deps.len().max(1);
            let request_stream =
                futures::stream::iter(deps.into_iter().map(move |dep| {
                    let host = host.clone();
                    async move {
                        ServerReflectionRequest { host, message_request: Some(MessageRequest::FileByFilename(dep)) }
                    }
                }))
                .buffer_unordered(buffer);
            let streaming =
                client.clone().server_reflection_info(request_stream).await.unwrap_or_else(|_| todo!()).into_inner();
            let descriptors = streaming
                .try_fold(pool, |pl, recv| {
                    let client = client.clone();
                    async move {
                        match recv.message_response.unwrap_or_else(|| todo!()) {
                            MessageResponse::FileDescriptorResponse(fd_resp) => {
                                Self::reflection_file_descriptors_recursive(client.clone(), destination, pl, fd_resp)
                                    .await
                                    .unwrap_or_else(|_| todo!());
                            }
                            _ => todo!(),
                        }
                        Ok(pl)
                    }
                })
                .await;
            Ok(descriptors.unwrap_or_else(|_| todo!()))
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
