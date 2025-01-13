use relentless::interface::command::{Assault, Relentless};
use serde::{Deserialize, Serialize};

use crate::{client::DefaultGrpcRequest, evaluate::GrpcResponse, factory::GrpcRequest};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GrpcAssault {
    pub relentless: Relentless,
}
impl Assault<DefaultGrpcRequest<serde_json::Value, serde_json::value::Serializer>, tonic::Response<serde_json::Value>>
    for GrpcAssault
{
    type Request = GrpcRequest;
    type Response = GrpcResponse;
    fn command(&self) -> &Relentless {
        &self.relentless
    }
}
