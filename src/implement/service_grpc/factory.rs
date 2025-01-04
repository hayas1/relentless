use prost_reflect::{DescriptorPool, DynamicMessage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    assault::factory::RequestFactory,
    interface::{helper::coalesce::Coalesce, template::Template},
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GrpcRequest {}
impl Coalesce for GrpcRequest {
    fn coalesce(self, other: &Self) -> Self {
        Self {}
    }
}

impl RequestFactory<tonic::Request<DynamicMessage>> for GrpcRequest {
    type Error = crate::Error;
    fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<tonic::Request<DynamicMessage>, Self::Error> {
        let pool = DescriptorPool::decode(
            include_bytes!(
                "../../../target/debug/build/relentless-dev-server-grpc-966e593a5a4fc2ae/out/file_descriptor.bin"
            )
            .as_ref(),
        )
        .unwrap_or_else(|_| todo!());

        let message_descriptor = pool.get_message_by_name("greeter.HelloRequest").unwrap_or_else(|| todo!());
        let mut hello_request = DynamicMessage::new(message_descriptor);
        hello_request.set_field_by_name("name", prost_reflect::Value::String("Rust".to_string()));

        Ok(tonic::Request::new(hello_request))
    }
}
