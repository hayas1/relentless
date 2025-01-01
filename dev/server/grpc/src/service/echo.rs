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

#[cfg(test)]
mod tests {
    use pb::echo_server::Echo;

    use super::*;

    #[tokio::test]
    async fn test_echo() {
        let echo = EchoImpl;

        let request = Request::new(Any::from_msg(&"100".to_string()).unwrap());
        let response = echo.echo(request).await.unwrap();
        assert_eq!(response.into_inner().to_msg::<String>().unwrap(), "100");
    }

    #[tokio::test]
    async fn test_echo_value() {
        let echo = EchoImpl;

        let request = Request::new(Value::from(200));
        let response = echo.echo_value(request).await.unwrap();
        assert_eq!(response.into_inner(), Value::from(200));
    }
}
