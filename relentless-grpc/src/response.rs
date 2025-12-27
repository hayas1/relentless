use std::{convert::Infallible, fmt::Debug};

use relentless::{
    evaluator::{json::JsonEvaluator, plaintext::PlaintextEvaluator},
    shot::{contract::ResponseSink, destinations::Destinations},
};
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct GrpcResponse {
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub status: Option<GrpcResponseStatus>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub metadata_map: Option<GrpcResponseMetadataMap>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    pub message: Option<GrpcResponseMessage>,
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

impl<Se: Debug + Send> ResponseSink<Result<tonic::Response<Se>, tonic::Status>> for GrpcResponse {
    type Error = Infallible;
    #[tracing::instrument(err)]
    async fn consume(&self, res: Destinations<Result<tonic::Response<Se>, tonic::Status>>) -> Result<(), Self::Error> {
        let Self { status, metadata_map, message } = self;
        Ok(())
    }
}
