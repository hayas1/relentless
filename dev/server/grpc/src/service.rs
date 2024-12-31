use num::BigInt;
use relentless_dev_server_grpc_entity::{
    counter_pb::counter_server::CounterServer, helloworld_pb::greeter_server::GreeterServer,
};
use tonic::transport::{server::Router, Server};
use tower::layer::util::Identity;

use crate::env::Env;

pub mod counter;
pub mod helloworld;

pub async fn app_with(env: Env) -> Router<Identity> {
    app(env, 0.into()).await
}
pub async fn app(env: Env, initial_count: BigInt) -> Router<Identity> {
    router(env, initial_count).await
}
pub async fn router(env: Env, initial_count: BigInt) -> Router<Identity> {
    let _ = env;

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter.set_serving::<GreeterServer<helloworld::MyGreeter>>().await;
    health_reporter.set_serving::<CounterServer<counter::CounterImpl>>().await;

    let reflection_service = tonic_reflection::server::Builder::configure().build_v1().unwrap();

    Server::builder()
        .trace_fn(|_| tracing::info_span!(env!("CARGO_PKG_NAME")))
        .add_service(health_service)
        .add_service(GreeterServer::new(helloworld::MyGreeter::default()))
        .add_service(CounterServer::new(counter::CounterImpl::new(initial_count)))
        .add_service(reflection_service)
}
