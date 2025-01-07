use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use bytes::{Bytes, BytesMut};
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor, MethodDescriptor, ServiceDescriptor};
use prost_types::Value;
use serde::{Deserialize, Serialize};

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
        let uri = destination.clone();
        let pool = self.descriptor_pool()?;
        let (service, method) = Self::service_method(&pool, target)?;
        let message = self.message.as_ref().unwrap_or_else(|| todo!()).produce();
        let codec = MethodCodec::new(method.clone()); // TODO remove clone

        Ok(DefaultGrpcRequest { uri, service, method, codec, message })
    }
}
impl GrpcRequest {
    pub fn service_method(pool: &DescriptorPool, target: &str) -> crate::Result<(ServiceDescriptor, MethodDescriptor)> {
        let (svc, mtd) =
            target.split_once('/').ok_or_else(|| GrpcRequestError::FailToParse(target.to_string())).box_err()?; // TODO only one '/' ?
        let service =
            pool.get_service_by_name(svc).ok_or_else(|| GrpcRequestError::NoService(target.to_string())).box_err()?;
        let method = service
            .methods()
            .find(|m| m.name() == mtd)
            .ok_or_else(|| GrpcRequestError::NoMethod(target.to_string()))
            .box_err()?;
        Ok((service, method))
    }
    pub fn descriptor_pool(&self) -> crate::Result<DescriptorPool> {
        let mut descriptor_bytes = Vec::new();
        File::open(self.descriptor.as_ref().unwrap_or_else(|| todo!("// TODO use reflection")))
            .box_err()?
            .read_to_end(&mut descriptor_bytes)
            .box_err()?;
        DescriptorPool::decode(Bytes::from(descriptor_bytes)).box_err()
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
