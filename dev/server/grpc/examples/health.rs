use relentless_grpc_dev_server::service::counter::{pb::counter_server::CounterServer, CounterImpl};
use tonic::Request;
use tonic_health::pb::health_client::HealthClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut health_client = HealthClient::new(CounterServer::new(CounterImpl::default()));
    let request = Request::new(tonic_health::pb::HealthCheckRequest { service: "counter".into() });
    let response = health_client.check(request).await?;
    println!("RESPONSE={:?}", response);

    Ok(())
}
