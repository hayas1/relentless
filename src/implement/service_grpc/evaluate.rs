use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    assault::{destinations::Destinations, evaluate::Evaluate, messages::Messages, result::RequestResult},
    interface::helper::coalesce::Coalesce,
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GrpcResponse {}
impl Coalesce for GrpcResponse {
    fn coalesce(self, other: &Self) -> Self {
        Self {}
    }
}

impl Evaluate<tonic::Response<Value>> for GrpcResponse {
    type Message = ();
    async fn evaluate(
        &self,
        res: Destinations<RequestResult<tonic::Response<Value>>>,
        msg: &mut Messages<Self::Message>,
    ) -> bool {
        true
    }
}
