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
        let response = pb::HelloResponse { greeting: format!("Hello {name}!") };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use pb::{greeter_client::GreeterClient, greeter_server::GreeterServer};

    use super::*;

    #[tokio::test]
    async fn test_greeter_basic() {
        let server = GreeterServer::new(GreeterImpl);
        let mut client = GreeterClient::new(server);

        assert_eq!(
            client.say_hello(pb::HelloRequest { name: "Rust".to_string() }).await.unwrap().into_inner(),
            pb::HelloResponse { greeting: "Hello Rust!".to_string() },
        );
    }
}
