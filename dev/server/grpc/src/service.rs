use counter::pb::counter_server::CounterServer;
use echo::pb::echo_server::EchoServer;
use helloworld::pb::greeter_server::GreeterServer;
use num::BigInt;
use tonic::transport::{server::Router, Server};
use tonic_health::{pb::health_server::HealthServer, server::HealthService};
use tower::layer::util::Identity;

use crate::env::Env;

pub mod counter;
pub mod echo;
pub mod helloworld;

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("file_descriptor");

pub async fn app(env: Env) -> Router<Identity> {
    app_with(env, 0.into()).await
}
pub async fn app_with(env: Env, initial_count: BigInt) -> Router<Identity> {
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter.set_serving::<HealthServer<HealthService>>().await;
    health_reporter.set_serving::<GreeterServer<helloworld::MyGreeter>>().await;
    health_reporter.set_serving::<CounterServer<counter::CounterImpl>>().await;
    health_reporter.set_serving::<EchoServer<echo::EchoImpl>>().await;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_reflection::pb::v1::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    router(env, initial_count).add_service(health_service).add_service(reflection_service)
}
pub fn router(env: Env, initial_count: BigInt) -> Router<Identity> {
    let _ = env;

    Server::builder()
        .trace_fn(|_| tracing::info_span!(env!("CARGO_PKG_NAME")))
        .add_service(GreeterServer::new(helloworld::MyGreeter::default()))
        .add_service(CounterServer::new(counter::CounterImpl::new(initial_count)))
        .add_service(EchoServer::new(echo::EchoImpl))
}
