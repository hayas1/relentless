use std::{
    convert::Infallible,
    fs::File,
    future::Future,
    io::Read,
    marker::PhantomData,
    path::PathBuf,
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use http::uri::PathAndQuery;
use prost::Message;
use prost_reflect::DescriptorPool;
use prost_types::FileDescriptorProto;
use relentless::shot::{
    contract::{Contract, SignContract},
    job::BasePath,
};
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
    #[tracing::instrument(skip(service), err)]
    async fn sign_contract(
        &self,
        service: G,
        destination: &http::Uri,
        base_path: &Option<BasePath>,
    ) -> Result<DynamicContract<D, S>, Self::Error> {
        let pool = match self {
            Self::ProtoFiles { protos, includes } => Self::from_protos(base_path, protos, includes)?,
            Self::Bin(path) => Self::descriptor_from_file(path)?,
            Self::Reflection => Self::from_reflection(service, destination).await?,
        };
        Ok(DynamicContract { pool, phantom: PhantomData })
    }
}
impl GrpcDescriptor {
    #[tracing::instrument(err)]
    pub fn from_protos(
        base_path: &Option<BasePath>,
        protos: &[PathBuf],
        includes: &[PathBuf],
    ) -> relentless::Result<DescriptorPool> {
        let builder = &mut prost_build::Config::new();
        let abs_protos: Vec<_> =
            protos.iter().map(|p| base_path.as_ref().map(|b| b.resolve(p)).unwrap_or_else(|| p.clone())).collect();
        let abs_includes: Vec<_> =
            includes.iter().map(|p| base_path.as_ref().map(|b| b.resolve(p)).unwrap_or_else(|| p.clone())).collect();
        let fds = builder.load_fds(&abs_protos, &abs_includes).map_err(relentless::Error::boxed)?;
        DescriptorPool::from_file_descriptor_set(fds).map_err(relentless::Error::boxed)
    }
    #[tracing::instrument(err)]
    pub fn descriptor_from_file(path: &PathBuf) -> relentless::Result<DescriptorPool> {
        let mut descriptor_bytes = Vec::new();
        File::open(path)
            .map_err(relentless::Error::boxed)?
            .read_to_end(&mut descriptor_bytes)
            .map_err(relentless::Error::boxed)?;
        DescriptorPool::decode(Bytes::from(descriptor_bytes)).map_err(relentless::Error::boxed)
    }
    #[tracing::instrument(skip(service), err)]
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
        let buffer = services.len().max(1);
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
    type Request = (MethodPath, tonic::Request<D>);
    type TransportReq = http::Request<tonic::body::Body>;
    type TransportRes = http::Response<G::ResponseBody>;
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
impl<G, D, S> Service<(MethodPath, tonic::Request<D>)> for DynamicService<G, D, S>
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

    fn call(&mut self, req: (MethodPath, tonic::Request<D>)) -> Self::Future {
        let (method_path, request) = req;
        let codec = DynamicCodec::with_pool(self.pool.clone(), &method_path, JsonSerializer::default()).unwrap();
        let mut grpc = tonic::client::Grpc::new(self.service.clone());
        Box::pin(async move {
            let path = method_path.format().map_err(|e| Status::unknown(e.to_string()))?;
            grpc.ready().await.map_err(|e| Status::unknown(format!("Service was not ready: {}", e.into())))?; // ref https://github.com/hyperium/tonic/blob/v0.14.2/tonic-build/src/client.rs#L240-L242
            grpc.unary(request, path, codec).await
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct MethodPath {
    service: String,
    method: String,
}
impl FromStr for MethodPath {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match &s.split('/').collect::<Vec<_>>()[..] {
            &[service, method] | &["", service, method] => Ok((service, method).into()),
            _ => todo!(),
        }
    }
}
impl<S: Into<String>, M: Into<String>> From<(S, M)> for MethodPath {
    fn from((service, method): (S, M)) -> Self {
        Self { service: service.into(), method: method.into() }
    }
}
impl From<MethodPath> for (String, String) {
    fn from(MethodPath { service, method }: MethodPath) -> Self {
        (service, method)
    }
}
impl MethodPath {
    pub fn parts(&self) -> (&str, &str) {
        let Self { service, method } = self;
        (service, method)
    }
    pub fn format(&self) -> Result<PathAndQuery, <PathAndQuery as FromStr>::Err> {
        // ref https://github.com/hyperium/tonic/blob/v0.14.2/tonic-build/src/lib.rs#L158-L164
        let Self { service, method } = self;
        format!("/{service}/{method}").parse()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use relentless_grpc_dev_server::app::Application;

    use super::*;

    #[tokio::test]
    async fn test_from_reflection() {
        let destination = "127.0.0.1:50051".parse().unwrap();
        let service = Application::reflection_service();
        let pool = GrpcDescriptor::from_reflection(service, &destination).await.unwrap();
        let files: HashSet<_> = pool.files().map(|f| f.name().to_string()).collect();
        assert_eq!(
            files,
            vec![
                "health.proto",
                "reflection_v1.proto",
                "greeter.proto",
                "random.proto",
                "google/protobuf/duration.proto",
                "google/protobuf/timestamp.proto",
                "google/protobuf/wrappers.proto",
                "google/protobuf/empty.proto",
                "google/protobuf/any.proto",
                "google/protobuf/struct.proto",
            ]
            .into_iter()
            .map(|p| p.to_string())
            .collect()
        );
    }

    #[test]
    fn test_method_path() {
        let method_path1 = MethodPath::from_str("greeter.Greeter/SayHello").unwrap();
        assert_eq!(method_path1.parts(), ("greeter.Greeter", "SayHello"));
        assert_eq!(method_path1.format().unwrap(), PathAndQuery::from_static("/greeter.Greeter/SayHello"));

        let method_path2 = MethodPath::from_str("/greeter.Greeter/SayHello").unwrap();
        assert_eq!(method_path2.parts(), ("greeter.Greeter", "SayHello"));
        assert_eq!(method_path2.format().unwrap(), PathAndQuery::from_static("/greeter.Greeter/SayHello"));

        // let error = MethodPath::from_str("//greeter.Greeter/SayHello").unwrap_err();
        // assert_eq!(
        //     error.to_string(),
        //     todo!()
        // );
    }
}
