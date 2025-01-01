use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("helloworld");
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl pb::greeter_server::Greeter for MyGreeter {
    async fn say_hello(&self, request: Request<pb::HelloRequest>) -> Result<Response<pb::HelloReply>, Status> {
        println!("Got a request: {:?}", request);

        let reply = pb::HelloReply { message: format!("Hello {}!", request.into_inner().name) };

        Ok(Response::new(reply))
    }
}
