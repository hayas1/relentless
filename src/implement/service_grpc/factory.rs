use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use bytes::{Bytes, BytesMut};
use prost_reflect::{DescriptorPool, DynamicMessage, MessageDescriptor};
use serde::{Deserialize, Serialize};

use crate::{
    assault::factory::RequestFactory,
    error::IntoResult,
    interface::{
        helper::{coalesce::Coalesce, is_default::IsDefault},
        template::Template,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GrpcRequest {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    descriptor: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    request_schema: Option<String>,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    response_schema: Option<String>,
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
            request_schema: self.request_schema.or(other.request_schema.clone()),
            response_schema: self.response_schema.or(other.response_schema.clone()),
            message: self.message.or(other.message.clone()),
        }
    }
}

impl RequestFactory<tonic::Request<DynamicMessage>> for GrpcRequest {
    type Error = crate::Error;
    fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<tonic::Request<DynamicMessage>, Self::Error> {
        let Self { descriptor, request_schema, message, .. } = self;
        let mut descriptor_bytes = Vec::new();
        File::open(descriptor.as_ref().unwrap_or_else(|| todo!()))
            .unwrap_or_else(|e| todo!("{}", e))
            .read_to_end(&mut descriptor_bytes)
            .unwrap_or_else(|e| todo!("{}", e));

        let pool = DescriptorPool::decode(Bytes::from(descriptor_bytes)).unwrap_or_else(|_| todo!());
        let message_descriptor =
            pool.get_message_by_name(request_schema.as_ref().unwrap_or_else(|| todo!())).unwrap_or_else(|| todo!());

        Ok(tonic::Request::new(message.as_ref().unwrap_or_else(|| todo!()).produce(message_descriptor)))
    }
}

impl GrpcMessage {
    pub fn produce(&self, descriptor: MessageDescriptor) -> DynamicMessage {
        let mut message = DynamicMessage::new(descriptor);
        message.set_field_by_name("name", prost_reflect::Value::String("Rust".to_string()));
        message
    }
}
