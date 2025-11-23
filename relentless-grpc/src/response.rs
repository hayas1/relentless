use std::convert::Infallible;

use relentless::{
    evaluator::{json::JsonEvaluator, plaintext::PlaintextEvaluator},
    shot::{contract::ResponseSink, destinations::Destinations},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GrpcResponse {
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub status: GrpcResponseStatus,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub metadata_map: GrpcResponseMetadataMap,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    pub message: GrpcResponseMessage,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcResponseStatus {
    #[default]
    OkOrEqual,
    // Expect(ExpectEvaluator<tonic::Code>), // TODO serde
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcResponseMetadataMap {
    #[default]
    AnyOrEqual,
    // Expect(ExpectEvaluator<tonic::metadata::MetadataMap>), // TODO serde
    Ignore,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcResponseMessage {
    #[default]
    AnyOrEqual,
    Plaintext(PlaintextEvaluator),
    Json(JsonEvaluator),
}

impl<Se: Send> ResponseSink<Result<tonic::Response<Se>, tonic::Status>> for GrpcResponse {
    type Error = Infallible;
    async fn consume(&self, res: Destinations<Result<tonic::Response<Se>, tonic::Status>>) -> Result<(), Self::Error> {
        todo!()
    }
}
