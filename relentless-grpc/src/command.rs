use relentless::interface::command::{Assault, Relentless};
use serde::{Deserialize, Serialize};

use crate::{
    client::GrpcMethodRequest, evaluate::GrpcResponse, factory::GrpcRequest, helper::JsonSerializer,
    record::GrpcIoRecorder,
};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GrpcAssault {
    relentless: Relentless,
}
impl Assault<GrpcMethodRequest<serde_json::Value, JsonSerializer>, tonic::Response<serde_json::Value>> for GrpcAssault {
    type Request = GrpcRequest;
    type Response = GrpcResponse;
    type Recorder = GrpcIoRecorder;
    type Layer = ();

    fn command(&self) -> &Relentless {
        &self.relentless
    }
    fn recorder(&self) -> Self::Recorder {
        GrpcIoRecorder
    }
    fn layer(&self) -> Self::Layer {}
}

impl GrpcAssault {
    pub fn new(relentless: Relentless) -> Self {
        Self { relentless }
    }
}
