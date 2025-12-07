use std::convert::Infallible;

use relentless::shot::contract::RequestSource;
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct GrpcRequest {
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    message: Option<GrpcRequestMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcRequestMessage {
    #[default]
    Empty,
    Json(serde_json::Value),
}

impl<De> RequestSource<De> for GrpcRequest {
    type Error = Infallible;
    async fn produce(&self, destination: &http::Uri, target: &str) -> Result<De, Self::Error> {
        todo!()
    }
}
