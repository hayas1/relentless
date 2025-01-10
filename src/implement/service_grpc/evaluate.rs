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
    pub header: GrpcHeaders,
    #[serde(default, skip_serializing_if = "IsDefault::is_default")]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub body: GrpcBody,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcHeaders {
    #[default]
    AnyOrEqual,
    // Expect(AllOr<tonic::metadata::MetadataMap>), // TODO
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcBody {
    #[default]
    AnyOrEqual,
    Plaintext(PlaintextEvaluator),
    #[cfg(feature = "json")]
    Json(JsonEvaluator),
}
impl Coalesce for GrpcResponse {
    fn coalesce(self, other: &Self) -> Self {
        Self { header: self.header.coalesce(&other.header), body: self.body.coalesce(&other.body) }
    }
}
impl Coalesce for GrpcHeaders {
    fn coalesce(self, other: &Self) -> Self {
        if self.is_default() {
            other.clone()
        } else {
            self
        }
    }
}
impl Coalesce for GrpcBody {
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
        let (mut metadata, mut body, mut extension) = (Destinations::new(), Destinations::new(), Destinations::new());
        for (name, (m, b, e)) in parts {
            metadata.insert(name.clone(), m);
            body.insert(name.clone(), b);
            extension.insert(name.clone(), e);
        }
        true && self.body.accept(&body, msg) && true
    }
}

impl GrpcResponse {
    pub async fn unzip_parts(
        responses: Destinations<tonic::Response<serde_json::Value>>,
    ) -> Result<Destinations<(MetadataMap, serde_json::Value, Extensions)>, GrpcEvaluateError> {
        let mut parts = Destinations::new();
        for (name, response) in responses {
            let (metadata, body, extensions) = response.into_parts();
            parts.insert(name, (metadata, body, extensions));
        }
        Ok(parts)
    }
}

impl Acceptable<&serde_json::Value> for GrpcBody {
    type Message = GrpcEvaluateError;
    fn accept(&self, parts: &Destinations<&serde_json::Value>, msg: &mut Messages<Self::Message>) -> bool {
        match self {
            GrpcBody::AnyOrEqual => true,
            GrpcBody::Plaintext(plaintext) => todo!(),
            #[cfg(feature = "json")]
            GrpcBody::Json(json) => {
                let dst: Destinations<_> = parts
                    .iter()
                    .map(|(d, v)| (d, Bytes::from(serde_json::to_vec(v).unwrap_or_else(|_| todo!()))))
                    .collect();
                let dst_ref = dst.iter().map(|(d, v)| (d, v)).collect();
                // TODO Value -> Bytes -> Value conversion occurs here
                Self::sub_accept(json, &dst_ref, msg, GrpcEvaluateError::JsonEvaluateError)
            }
        }
    }
}
