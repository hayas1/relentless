use prost_types::{Any, Value};
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("echo");
}

#[derive(Debug, Default)]
pub struct EchoImpl;

#[tonic::async_trait]
impl pb::echo_server::Echo for EchoImpl {
    #[tracing::instrument(ret)]
    async fn echo(&self, request: Request<Any>) -> Result<Response<Any>, Status> {
        let value = request.into_inner();
        Ok(Response::new(value))
    }
    #[tracing::instrument(ret)]
    async fn echo_value(&self, request: Request<Value>) -> Result<Response<Value>, Status> {
        let value = request.into_inner();
        Ok(Response::new(value))
    }
}
