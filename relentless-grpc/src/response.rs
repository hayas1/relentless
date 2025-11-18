use relentless::{
    evaluator::{json::JsonEvaluator, plaintext::PlaintextEvaluator, Evaluator},
    shot::destinations::Destinations,
};
use serde::{Deserialize, Serialize};

use crate::client::GrpcClient;

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

impl<S: Send> Evaluator<GrpcClient<S>> for GrpcResponse {
    type Response = Result<tonic::Response<S>, tonic::Status>;

    async fn evaluate(&self, res: Destinations<Self::Response>) -> bool {
        todo!()
    }
}
