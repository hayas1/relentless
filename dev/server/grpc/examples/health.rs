use relentless_dev_server_grpc::service::counter::CounterImpl;
use relentless_dev_server_grpc_entity::counter_pb::counter_server::CounterServer;
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
