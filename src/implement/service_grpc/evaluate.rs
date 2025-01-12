use std::fmt::Debug;

use bytes::Bytes;
use http::Extensions;
use serde::{Deserialize, Serialize};
use tonic::metadata::MetadataMap;

use crate::{
    assault::{
        destinations::Destinations,
        evaluate::{Acceptable, Evaluate},
        evaluator::{json::JsonEvaluator, plaintext::PlaintextEvaluator},
        messages::Messages,
        result::RequestResult,
    },
    interface::helper::{coalesce::Coalesce, is_default::IsDefault},
};

use super::error::GrpcEvaluateError;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GrpcResponse {
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub metadata_map: GrpcMetadataMap,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub extensions: GrpcExtensions,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub message: GrpcMessage,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcMetadataMap {
    #[default]
    AnyOrEqual,
    // Expect(AllOr<tonic::metadata::MetadataMap>), // TODO
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcExtensions {
    #[default]
    AnyOrEqual,
    // Expect(AllOr<http_serde_priv::Extensions>), // TODO
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcMessage {
    #[default]
    AnyOrEqual,
    Plaintext(PlaintextEvaluator),
    #[cfg(feature = "json")]
    Json(JsonEvaluator),
}
impl Coalesce for GrpcResponse {
    fn coalesce(self, other: &Self) -> Self {
        Self {
            metadata_map: self.metadata_map.coalesce(&other.metadata_map),
            extensions: self.extensions.coalesce(&other.extensions),
            message: self.message.coalesce(&other.message),
        }
    }
}
impl Coalesce for GrpcMetadataMap {
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
impl Coalesce for GrpcExtensions {
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}

impl Evaluate<tonic::Response<serde_json::Value>> for GrpcResponse {
    type Message = GrpcEvaluateError;
    async fn evaluate(
        &self,
        res: Destinations<RequestResult<tonic::Response<serde_json::Value>>>,
        msg: &mut Messages<Self::Message>,
    ) -> bool {
        let Some(responses) = msg.response_destinations_with(res, GrpcEvaluateError::RequestError) else {
            return false;
        };
        let Some(parts) = msg.push_if_err(GrpcResponse::unzip_parts(responses).await) else {
            return false;
        };

        self.accept(&parts, msg)
    }
}

impl Acceptable<(MetadataMap, serde_json::Value, Extensions)> for GrpcResponse {
    type Message = GrpcEvaluateError;
    fn accept(
        &self,
        parts: &Destinations<(MetadataMap, serde_json::Value, Extensions)>,
        msg: &mut Messages<Self::Message>,
    ) -> bool {
        let (mut metadata, mut message, mut extensions) =
            (Destinations::new(), Destinations::new(), Destinations::new());
        for (name, (d, m, e)) in parts {
            metadata.insert(name.clone(), d.clone().into_headers());
            message.insert(name.clone(), m);
            extensions.insert(name.clone(), e);
        }
        self.metadata_map.accept(&metadata, msg)
            && self.message.accept(&message, msg)
            && self.extensions.accept(&extensions, msg)
    }
}

impl GrpcResponse {
    pub async fn unzip_parts(
        responses: Destinations<tonic::Response<serde_json::Value>>,
    ) -> Result<Destinations<(MetadataMap, serde_json::Value, Extensions)>, GrpcEvaluateError> {
        let mut parts = Destinations::new();
        for (name, response) in responses {
            let (metadata, message, extensions) = response.into_parts();
            parts.insert(name, (metadata, message, extensions));
        }
        Ok(parts)
    }
}

impl Acceptable<&serde_json::Value> for GrpcMessage {
    type Message = GrpcEvaluateError;
    fn accept(&self, values: &Destinations<&serde_json::Value>, msg: &mut Messages<Self::Message>) -> bool {
        match self {
            GrpcMessage::AnyOrEqual => Self::assault_or_compare(values, |_| true),
            GrpcMessage::Plaintext(plaintext) => todo!(),
            #[cfg(feature = "json")]
            GrpcMessage::Json(json) => Self::sub_accept(json, values, msg, GrpcEvaluateError::JsonEvaluateError),
        }
    }
}

impl Acceptable<http::HeaderMap> for GrpcMetadataMap {
    type Message = GrpcEvaluateError;
    fn accept(&self, maps: &Destinations<http::HeaderMap>, msg: &mut Messages<Self::Message>) -> bool {
        let acceptable = match self {
            GrpcMetadataMap::AnyOrEqual => Self::assault_or_compare(maps, |_| true),
            GrpcMetadataMap::Ignore => true,
        };
        if !acceptable {
            msg.push_err(GrpcEvaluateError::UnacceptableMetadataMap);
        }
        acceptable
    }
}

impl Acceptable<&Extensions> for GrpcExtensions {
    type Message = GrpcEvaluateError;
    fn accept(&self, extensions: &Destinations<&Extensions>, msg: &mut Messages<Self::Message>) -> bool {
        let acceptable = match self {
            GrpcExtensions::AnyOrEqual => true, // TODO Self::assault_or_compare(extensions, |_| true),
            GrpcExtensions::Ignore => true,
        };
        if !acceptable {
            msg.push_err(GrpcEvaluateError::UnacceptableExtensions);
        }
        acceptable
    }
}
