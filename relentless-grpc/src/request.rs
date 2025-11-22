use std::path::PathBuf;

use http::uri::PathAndQuery;
use relentless::shot::contract::RequestSource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GrpcRequest {
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    descriptor: GrpcDescriptor,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    message: GrpcRequestMessage,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", untagged)]
pub enum GrpcDescriptor {
    Protos {
        #[serde(default)]
        protos: Vec<PathBuf>,
        #[serde(default)]
        import_path: Vec<PathBuf>,
    },
    Bin(PathBuf),
    #[default]
    Reflection,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub enum GrpcRequestMessage {
    #[default]
    Empty,
    Json(serde_json::Value),
}

impl GrpcRequest {
    pub fn produce<'a, D>(s: RequestSource<'a, Self>) -> (PathAndQuery, tonic::Request<D>) {
        todo!()
    }
}
