use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tonic::GrpcMethod;

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

impl RequestFactory<http::Request<Value>> for GrpcRequest {
    type Error = crate::Error;
    fn produce(
        &self,
        destination: &http::Uri,
        target: &str,
        template: &Template,
    ) -> Result<http::Request<Value>, Self::Error> {
        // let mut buf = Vec::new();
        // prost::encoding::string::encode(1, &"100".to_string(), &mut buf);

        let request = http::Request::builder()
            // .uri(format!("{}{}", "http://localhost:50051", "/counter.Counter/Increment"))
            // .uri(format!("{}{}", "http://localhost:50051", "/grpc.health.v1.Health/Check"))
            .uri(format!("{}{}", "http://localhost:50051", "/echo.Echo/Echo"))
            .method(http::Method::POST)
            .header("content-type", "application/grpc")
            .header("te", "trailers")
            .body(json!({"service": "echo.Echo"}))
            // .body(json!({"service": "counter.Counter"}))
            // .body(json!(100))
            .unwrap_or_else(|e| todo!("{}", e));

        let mut r = tonic::Request::new("100".to_string());
        r.extensions_mut().insert(GrpcMethod::new("grpc.health.v1.Health", "Check"));
        Ok(request)
    }
}
