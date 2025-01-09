use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use http::uri::PathAndQuery;
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, MethodDescriptor, ServiceDescriptor};
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;
use tonic_reflection::pb::v1::{
    server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse, ServerReflectionRequest, FILE_DESCRIPTOR_SET,
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
    descriptor: Option<PathBuf>, // TODO compile `.proto` files with `protoc`
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    message: Option<GrpcMessage>,
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
        Self {
            descriptor: self.descriptor.or(other.descriptor.clone()),
            message: self.message.or(other.message.clone()),
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
        let message = self.message.as_ref().unwrap_or_else(|| todo!()).produce();
        let codec = MethodCodec::new(method.clone()); // TODO remove clone

        Ok(DefaultGrpcRequest { destination, service, method, codec, message })
    }
}
impl GrpcRequest {
    pub fn service_method(
        pool: &DescriptorPool,
        (svc, mtd): (&str, &str),
    ) -> crate::Result<(ServiceDescriptor, MethodDescriptor)> {
        let service = pool.get_service_by_name(svc).ok_or_else(|| GrpcRequestError::NoService(svc.to_string()))?;
        let method =
            service.methods().find(|m| m.name() == mtd).ok_or_else(|| GrpcRequestError::NoMethod(mtd.to_string()))?;
        Ok((service, method))
    }
    pub async fn descriptor_pool(
        &self,
        destination: &http::Uri,
        (svc, mtd): (&str, &str),
    ) -> crate::Result<DescriptorPool> {
        if let Some(descriptor) = self.descriptor.as_ref() {
            Self::descriptor_from_file(descriptor).await
        } else {
            Self::descriptor_from_reflection(destination, svc).await
        }
    }

    pub async fn descriptor_from_file(path: &PathBuf) -> crate::Result<DescriptorPool> {
        let mut descriptor_bytes = Vec::new();
        File::open(path).box_err()?.read_to_end(&mut descriptor_bytes).box_err()?;
        DescriptorPool::decode(Bytes::from(descriptor_bytes)).box_err()
    }

    pub async fn descriptor_from_reflection(destination: &http::Uri, svc: &str) -> crate::Result<DescriptorPool> {
        let conn = Channel::builder(destination.clone()).connect().await.box_err()?;
        let mut client = ServerReflectionClient::new(conn);
        let host = destination.host().unwrap_or_else(|| todo!()).to_string();
        let service = svc.to_string();
        let request_stream = futures::stream::once(async move {
            ServerReflectionRequest { host, message_request: Some(MessageRequest::FileContainingSymbol(service)) }
        });
        let mut streaming = client.server_reflection_info(request_stream).await.box_err()?.into_inner();
        while let Some(recv) = streaming.next().await {
            match recv {
                Ok(response) => {
                    let msg = response.message_response.unwrap_or_else(|| todo!());
                    match msg {
                        MessageResponse::FileDescriptorResponse(descriptor) => {
                            let mut pool = DescriptorPool::new();
                            for d in descriptor.file_descriptor_proto {
                                pool.decode_file_descriptor_proto(&*d).box_err()?;
                            }
                            return Ok(pool);
                        }
                        _ => unreachable!(),
                    }
                }
                Err(e) => todo!("{}", e),
            }
        }
        unreachable!()
    }
}

impl GrpcMessage {
    // TODO!!!
    pub fn produce(&self) -> serde_json::Value {
        match self {
            Self::Empty => todo!(),
            Self::Plaintext(_) => todo!(),
            #[cfg(feature = "json")]
            // Self::Json(v) => DynamicMessage::deserialize(descriptor, v).unwrap_or_else(|e| todo!("{}", e)),
            Self::Json(v) => v.clone(),
        }
    }
}
