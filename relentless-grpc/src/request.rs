use std::{convert::Infallible, str::FromStr};

use relentless::shot::contract::RequestSource;
use semigroup::Semigroup;
use serde::{Deserialize, Serialize};

use crate::contract::MethodPath;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Semigroup)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[semigroup(with = "semigroup::op::Coalesce")]
pub struct GrpcRequest {
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    metadata: Option<GrpcRequestMetadata>,
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    #[serde(default)]
    message: Option<GrpcRequestMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcRequestMetadata {
    #[default]
    Empty,
    // TODO
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcRequestMessage {
    #[default]
    Empty,
    Value(serde_json::Value),
}

impl RequestSource<(MethodPath, tonic::Request<serde_json::Value>)> for GrpcRequest {
    type Error = relentless::Error;
    #[tracing::instrument(ret, err)]
    async fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
    ) -> Result<(MethodPath, tonic::Request<serde_json::Value>), Self::Error> {
        let pq = MethodPath::from_str(target).map_err(relentless::Error::boxed)?;
        let request = self.message.as_ref().unwrap_or(&Default::default()).produce(destination, target).await?;
        Ok((pq, tonic::Request::from_parts(Default::default(), Default::default(), request)))
    }
}
impl RequestSource<serde_json::Value> for GrpcRequestMessage {
    type Error = Infallible;
    async fn produce(&self, _: &http::Uri, _: &str) -> Result<serde_json::Value, Self::Error> {
        match self {
            Self::Empty => Ok(serde_json::json!({})),
            Self::Value(v) => Ok(v.clone()),
        }
    }
}
