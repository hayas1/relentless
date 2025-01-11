use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use prost_reflect::{DescriptorPool, MethodDescriptor, ServiceDescriptor};
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;
use tonic_reflection::pb::v1::{
    server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse, ServerReflectionRequest,
};

use crate::{
    assault::factory::RequestFactory,
    error::IntoResult,
    interface::{
        helper::{coalesce::Coalesce, is_default::IsDefault},
        template::Template,
    },
};

use super::{
    client::{DefaultGrpcRequest, MethodCodec},
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
    #[cfg(feature = "json")]
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

impl RequestFactory<DefaultGrpcRequest<serde_json::Value, serde_json::value::Serializer>> for GrpcRequest {
    type Error = crate::Error;
    async fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<DefaultGrpcRequest<serde_json::Value, serde_json::value::Serializer>, Self::Error> {
        let (svc, mtd) = target.split_once('/').ok_or_else(|| GrpcRequestError::FailToParse(target.to_string()))?; // TODO only one '/' ?
        let pool = self.descriptor_pool(destination, (svc, mtd)).await?;
        let destination = destination.clone();
        let (service, method) = Self::service_method(&pool, (svc, mtd))?;
        let message = self.message.produce();
        let codec = MethodCodec::new(method.clone()); // TODO remove clone

        Ok(DefaultGrpcRequest { destination, service, method, codec, message })
    }
}
impl GrpcRequest {
    pub fn service_method(
        pool: &DescriptorPool,
        (service, method): (&str, &str),
    ) -> crate::Result<(ServiceDescriptor, MethodDescriptor)> {
        let svc = pool.get_service_by_name(service).ok_or_else(|| GrpcRequestError::NoService(service.to_string()))?;
        let mtd =
            svc.methods().find(|m| m.name() == method).ok_or_else(|| GrpcRequestError::NoMethod(method.to_string()))?;
        Ok((svc, mtd))
    }
    pub async fn descriptor_pool(
        &self,
        destination: &http::Uri,
        (service, _method): (&str, &str),
    ) -> crate::Result<DescriptorPool> {
        match &self.descriptor {
            DescriptorFrom::Protos { protos, import_path } => Self::descriptor_from_protos(protos, import_path).await,
            DescriptorFrom::Bin(path) => Self::descriptor_from_file(path).await,
            DescriptorFrom::Reflection => Self::descriptor_from_reflection(destination, service).await,
        }
    }

    pub async fn descriptor_from_protos<A: AsRef<Path>>(
        protos: &[A],
        import_path: &[A],
    ) -> crate::Result<DescriptorPool> {
        let builder = &mut prost_build::Config::new();
        let fds = builder.load_fds(protos, import_path).box_err()?;
        DescriptorPool::from_file_descriptor_set(fds).box_err()
    }

    pub async fn descriptor_from_file(path: &PathBuf) -> crate::Result<DescriptorPool> {
        let mut descriptor_bytes = Vec::new();
        File::open(path).box_err()?.read_to_end(&mut descriptor_bytes).box_err()?;
        DescriptorPool::decode(Bytes::from(descriptor_bytes)).box_err()
    }

    pub async fn descriptor_from_reflection(destination: &http::Uri, svc: &str) -> crate::Result<DescriptorPool> {
        // TODO well known type, cache, etc...
        let conn = Channel::builder(destination.clone()).connect().await.box_err()?;
        let mut client = ServerReflectionClient::new(conn);
        let host = destination.host().unwrap_or_else(|| todo!()).to_string();
        let service = svc.to_string();
        let request_stream = futures::stream::once(async move {
            ServerReflectionRequest { host, message_request: Some(MessageRequest::FileContainingSymbol(service)) }
        });
        let streaming = client.server_reflection_info(request_stream).await.box_err()?.into_inner();
        let descriptors = streaming
            .map(|recv| recv.box_err())
            .try_fold(DescriptorPool::new(), |pool, recv| async move {
                match recv.message_response.unwrap_or_else(|| todo!()) {
                    MessageResponse::FileDescriptorResponse(descriptor) => descriptor
                        .file_descriptor_proto
                        .into_iter()
                        .try_fold(pool, |mut p, d| p.decode_file_descriptor_proto(&*d).box_err().map(|()| p)),
                    _ => Err(GrpcRequestError::UnexpectedReflectionResponse)?,
                }
            })
            .await
            .box_err()?;
        Ok(descriptors)
    }
}

impl GrpcMessage {
    // TODO!!!
    pub fn produce(&self) -> serde_json::Value {
        match self {
            Self::Empty => serde_json::Value::Object(serde_json::Map::new()),
            Self::Plaintext(_) => todo!(),
            #[cfg(feature = "json")]
            // Self::Json(v) => DynamicMessage::deserialize(descriptor, v).unwrap_or_else(|e| todo!("{}", e)),
            Self::Json(v) => v.clone(),
        }
    }
}
