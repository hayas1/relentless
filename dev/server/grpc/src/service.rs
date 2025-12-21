use counter::pb::counter_server::CounterServer;
use echo::pb::echo_server::EchoServer;
use greeter::pb::greeter_server::GreeterServer;
use num::BigInt;
use tonic::transport::{server::Router, Server};
use tonic_health::{
    pb::health_server::{Health, HealthServer},
    server::HealthService,
};
use tonic_reflection::server::v1::{ServerReflection, ServerReflectionServer};
use tonic_tracing_opentelemetry::middleware::{filters, server::OtelGrpcLayer};
use tower::layer::util::{Identity, Stack};

use crate::{env::Env, middleware::logging::LoggingLayer};

pub mod counter;
pub mod echo;
pub mod greeter;

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("file_descriptor");

pub async fn app(env: Env) -> Router<Stack<OtelGrpcLayer, Identity>> {
    app_with(env, 0.into()).await
}
pub async fn app_with(env: Env, initial_count: BigInt) -> Router<Stack<OtelGrpcLayer, Identity>> {
    let health_service = health_service().await;
    let reflection_service = reflection_service();

    let mut builder = Server::builder().layer(OtelGrpcLayer::default().filter(filters::reject_healthcheck));
    // .trace_fn(|_| tracing::info_span!(env!("CARGO_PKG_NAME")))
    // .layer(LoggingLayer);
    router(&mut builder, env, initial_count).add_service(health_service).add_service(reflection_service)
}
pub async fn health_service() -> HealthServer<impl Health> {
    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter.set_serving::<HealthServer<HealthService>>().await;
    health_reporter.set_serving::<GreeterServer<greeter::GreeterImpl>>().await;
    health_reporter.set_serving::<CounterServer<counter::CounterImpl>>().await;
    health_reporter.set_serving::<EchoServer<echo::EchoImpl>>().await;

    health_service
}
pub fn reflection_service() -> ServerReflectionServer<impl ServerReflection> {
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_reflection::pb::v1::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    reflection_service
}
pub fn router(
    builder: &mut Server<Stack<OtelGrpcLayer, Identity>>,
    env: Env,
    initial_count: BigInt,
) -> Router<Stack<OtelGrpcLayer, Identity>> {
    let _ = env;

    builder
        .add_service(GreeterServer::new(greeter::GreeterImpl))
        .add_service(CounterServer::new(counter::CounterImpl::new(initial_count)))
        .add_service(EchoServer::new(echo::EchoImpl))
}
