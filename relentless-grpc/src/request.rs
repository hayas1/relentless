use std::{marker::PhantomData, path::PathBuf};

use http::uri::PathAndQuery;
use relentless::generator::Generator;
use serde::{Deserialize, Serialize};

use crate::codec::DynamicCodec;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct GrpcRequest<D, S> {
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    descriptor: GrpcDescriptor,
    #[serde(default)]
    #[cfg_attr(feature = "yaml", serde(with = "serde_yaml::with::singleton_map_recursive"))]
    message: GrpcRequestMessage,

    #[serde(skip_serializing, skip_deserializing)]
    phantom: PhantomData<(D, S)>,
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
    Plaintext(String),
    Json(serde_json::Value),
}

impl<D: Send + Sync, S: Send + Sync> Generator for GrpcRequest<D, S> {
    type Request = (tonic::Request<D>, PathAndQuery, DynamicCodec<D, S>);
    type Error = tonic::Status;

    async fn generate(&self, _destination: &http::Uri, _target: &str) -> Result<Self::Request, Self::Error> {
        todo!()
    }
}
