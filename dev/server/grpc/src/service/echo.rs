use relentless_dev_server_grpc_entity::echo_pb::{echo_server::Echo, EchoAny, EchoAnyValue};

#[derive(Debug, Default)]
pub struct EchoImpl;

#[tonic::async_trait]
impl Echo for EchoImpl {
    #[tracing::instrument(ret)]
    async fn echo(&self, request: tonic::Request<EchoAny>) -> Result<tonic::Response<EchoAny>, tonic::Status> {
        let EchoAny { value } = request.into_inner();
        Ok(tonic::Response::new(EchoAny { value }))
    }
    #[tracing::instrument(ret)]
    async fn echo_value(
        &self,
        request: tonic::Request<EchoAnyValue>,
    ) -> Result<tonic::Response<EchoAnyValue>, tonic::Status> {
        let EchoAnyValue { value } = request.into_inner();
        Ok(tonic::Response::new(EchoAnyValue { value }))
    }
}
