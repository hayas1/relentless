use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use prost::Message;
use prost_reflect::{DescriptorPool, MethodDescriptor, ServiceDescriptor};
use prost_types::FileDescriptorProto;
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;
use tonic_reflection::pb::v1::{
    server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse, ServerReflectionRequest,
};

use relentless::{
    assault::factory::RequestFactory,
    error::IntoResult,
    interface::{
        helper::{coalesce::Coalesce, is_default::IsDefault},
        template::Template,
    },
};
use tower::Service;

use crate::helper::JsonSerializer;

use super::{
    client::{GrpcMethodRequest, MethodCodec},
    error::GrpcRequestError,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GrpcRequest {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    descriptor: DescriptorFrom,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    message: GrpcMessage,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum DescriptorFrom {
    Protos {
        #[serde(default, skip_serializing_if = "IsDefault::is_default")]
        protos: Vec<PathBuf>,
        #[serde(default, skip_serializing_if = "IsDefault::is_default")]
        import_path: Vec<PathBuf>,
    },
    Bin(PathBuf),
    #[default]
    Reflection,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcMessage {
    #[default]
    Empty,
    Plaintext(String),
    Json(serde_json::Value),
}
impl Coalesce for GrpcRequest {
    fn coalesce(self, other: &Self) -> Self {
        Self { descriptor: self.descriptor.coalesce(&other.descriptor), message: self.message.coalesce(&other.message) }
    }
}
impl Coalesce for DescriptorFrom {
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}
impl Coalesce for GrpcMessage {
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}

impl<S> RequestFactory<GrpcMethodRequest<serde_json::Value, JsonSerializer>, S> for GrpcRequest
where
    S: Service<GrpcMethodRequest<serde_json::Value, JsonSerializer>>,
{
    type Error = relentless::Error;
    async fn produce(
        &self,
        service: S,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<GrpcMethodRequest<serde_json::Value, JsonSerializer>, Self::Error> {
        let (svc, mth) = target.split_once('/').ok_or_else(|| GrpcRequestError::FailToParse(target.to_string()))?; // TODO only one '/' ?
        let pool = self.descriptor_pool(service, destination, (svc, mth)).await?;
        let destination = destination.clone();
        let (service, method) = Self::service_method(&pool, (svc, mth))?;
        let message = template.render_json_recursive(&self.message.produce())?;
        let codec = MethodCodec::new(method.clone(), JsonSerializer::default()); // TODO remove clone

        Ok(GrpcMethodRequest { destination, service, method, codec, message })
    }
}
impl GrpcRequest {
    pub fn service_method(
        pool: &DescriptorPool,
        (service, method): (&str, &str),
    ) -> relentless::Result<(ServiceDescriptor, MethodDescriptor)> {
        let svc = pool.get_service_by_name(service).ok_or_else(|| GrpcRequestError::NoService(service.to_string()))?;
        let mth =
            svc.methods().find(|m| m.name() == method).ok_or_else(|| GrpcRequestError::NoMethod(method.to_string()))?;
        Ok((svc, mth))
    }
    pub async fn descriptor_pool<S>(
        &self,
        service: S,
        destination: &http::Uri,
        (svc, _mth): (&str, &str),
    ) -> relentless::Result<DescriptorPool> {
        // TODO cache
        match &self.descriptor {
            DescriptorFrom::Protos { protos, import_path } => Self::descriptor_from_protos(protos, import_path).await,
            DescriptorFrom::Bin(path) => Self::descriptor_from_file(path).await,
            DescriptorFrom::Reflection => Self::descriptor_from_reflection(service, destination, svc).await,
        }
    }

    pub async fn descriptor_from_protos<A: AsRef<Path>>(
        protos: &[A],
        import_path: &[A],
    ) -> relentless::Result<DescriptorPool> {
        let builder = &mut prost_build::Config::new();
        let fds = builder.load_fds(protos, import_path).box_err()?;
        DescriptorPool::from_file_descriptor_set(fds).box_err()
    }

    pub async fn descriptor_from_file(path: &PathBuf) -> relentless::Result<DescriptorPool> {
        let mut descriptor_bytes = Vec::new();
        File::open(path).box_err()?.read_to_end(&mut descriptor_bytes).box_err()?;
        DescriptorPool::decode(Bytes::from(descriptor_bytes)).box_err()
    }

    pub async fn descriptor_from_reflection<S>(
        _service: S,
        destination: &http::Uri,
        svc: &str,
    ) -> relentless::Result<DescriptorPool> {
        // TODO!!! do not use Channel directly, use Service
        let mut client = ServerReflectionClient::new(Channel::builder(destination.clone()).connect().await.box_err()?);
        let (host, service) = (
            destination.host().ok_or_else(|| GrpcRequestError::NoHost(destination.clone()))?.to_string(),
            svc.to_string(),
        );
        let request_stream = futures::stream::once({
            let host = host.clone();
            async move {
                ServerReflectionRequest { host, message_request: Some(MessageRequest::FileContainingSymbol(service)) }
            }
        });
        let streaming = client.server_reflection_info(request_stream).await.box_err()?.into_inner();
        let descriptors = streaming
            .map(|recv| async { recv.box_err() })
            .buffer_unordered(1)
            .try_fold(DescriptorPool::new(), move |mut pool, recv| {
                let host = host.to_string();
                async move {
                    let MessageResponse::FileDescriptorResponse(descriptor) =
                        recv.message_response.ok_or_else(|| GrpcRequestError::EmptyResponse)?
                    else {
                        return Err(GrpcRequestError::UnexpectedReflectionResponse.into());
                    };
                    futures::stream::iter(descriptor.file_descriptor_proto.into_iter())
                        .map(|d| async { Ok(d) })
                        .buffer_unordered(16)
                        .try_fold(&mut pool, move |p, d| {
                            let host = host.clone();
                            async move {
                                let fd = FileDescriptorProto::decode(&*d).box_err()?;
                                Self::fetch_all_descriptors(destination, &host, p, fd).await.map(|()| p)
                            }
                        })
                        .await?;
                    Ok(pool)
                }
            })
            .await?;
        Ok(descriptors)
    }

    pub async fn fetch_all_descriptors(
        destination: &http::Uri,
        host: &str,
        pool: &mut DescriptorPool,
        fd: FileDescriptorProto,
    ) -> relentless::Result<()> {
        let mut stack = vec![fd]; // TODO use stream as stack ?
        let mut client = ServerReflectionClient::new(Channel::builder(destination.clone()).connect().await.box_err()?);
        while let Some(proto) = stack.pop() {
            if pool.add_file_descriptor_proto(proto.clone()).is_err() {
                stack.push(proto.clone());
                let host = host.to_string();
                let dep_streaming = client
                    .server_reflection_info(futures::stream::iter(proto.dependency.into_iter().map(move |dep| {
                        let host = host.clone();
                        ServerReflectionRequest { host, message_request: Some(MessageRequest::FileByFilename(dep)) }
                    })))
                    .await
                    .box_err()?
                    .into_inner();
                dep_streaming
                    .map(|recv| async { recv.box_err() })
                    .buffer_unordered(16)
                    .try_fold(&mut stack, |dfs, recv| async move {
                        let MessageResponse::FileDescriptorResponse(descriptor) =
                            recv.message_response.ok_or_else(|| GrpcRequestError::EmptyResponse)?
                        else {
                            return Err(GrpcRequestError::UnexpectedReflectionResponse.into());
                        };
                        let dep_protos: relentless::Result<Vec<_>> = descriptor
                            .file_descriptor_proto
                            .into_iter()
                            .map(|d| FileDescriptorProto::decode(&*d).box_err())
                            .collect();
                        dfs.extend(dep_protos?); // TODO dedup in advance?
                        Ok(dfs)
                    })
                    .await?;
            }
        }
        Ok(())
    }
}

impl GrpcMessage {
    // TODO type of grpc message
    pub fn produce(&self) -> serde_json::Value {
        match self {
            Self::Empty => serde_json::Value::Object(serde_json::Map::new()),
            Self::Plaintext(_) => unimplemented!(),
            Self::Json(v) => v.clone(),
        }
    }
}
