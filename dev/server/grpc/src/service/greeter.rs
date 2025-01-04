use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("greeter");
}

#[derive(Debug, Default)]
pub struct GreeterImpl;

#[tonic::async_trait]
impl pb::greeter_server::Greeter for GreeterImpl {
    #[tracing::instrument(ret)]
    async fn say_hello(&self, request: Request<pb::HelloRequest>) -> Result<Response<pb::HelloResponse>, Status> {
        let pb::HelloRequest { name } = request.into_inner();
        let response = pb::HelloResponse { message: format!("Hello {}!", name) };

        Ok(Response::new(response))
    }
}
