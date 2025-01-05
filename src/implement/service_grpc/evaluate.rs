use std::fmt::Debug;

use serde::{Deserialize, Serialize};

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

impl<T: Debug> Evaluate<tonic::Response<T>> for GrpcResponse {
    type Message = ();
    async fn evaluate(
        &self,
        res: Destinations<RequestResult<tonic::Response<T>>>,
        msg: &mut Messages<Self::Message>,
    ) -> bool {
        dbg!(res);
        true
    }
}
