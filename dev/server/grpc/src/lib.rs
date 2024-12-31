use service::helloworld::{hello_world::greeter_server::GreeterServer, MyGreeter};
use tonic::transport::Server;

pub mod env;
pub mod service;
pub mod state;

pub async fn serve(env: env::Env) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = env.bind().parse()?;
    let greeter = MyGreeter::default();

    let server = Server::builder().add_service(GreeterServer::new(greeter));
    server.serve(addr).await?;

    Ok(())
}
