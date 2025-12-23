use std::sync::Arc;

use tonic::transport::{server::Router, Server};
use tonic_health::{
    pb::health_server::{Health, HealthServer},
    server::HealthService,
};
use tonic_reflection::server::v1::{ServerReflection, ServerReflectionServer};
use tonic_tracing_opentelemetry::middleware::{filters, server::OtelGrpcLayer};
use tower::layer::util::{Identity, Stack};

use crate::{
    app::{
        counter::{pb::counter_server::CounterServer, CounterImpl, CounterState},
        echo::{pb::echo_server::EchoServer, EchoImpl},
        greeter::{pb::greeter_server::GreeterServer, GreeterImpl},
        random::{pb::random_server::RandomServer, RandomImpl},
        wait::{pb::wait_server::WaitServer, WaitImpl},
    },
    runner::RunCommand,
};

pub mod counter;
pub mod echo;
pub mod greeter;
pub mod random;
pub mod wait;

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("file_descriptor");

#[derive(Debug, Clone, Default)]
pub struct AppRouter {
    pub state: AppState,
}
impl AppRouter {
    pub async fn service(self) -> Router<Stack<OtelGrpcLayer, Identity>> {
        let mut builder = Server::builder().layer(OtelGrpcLayer::default().filter(filters::reject_healthcheck));
        self.router(&mut builder).add_service(Self::health_service().await).add_service(Self::reflection_service())
    }
    pub fn router<L: Clone>(self, builder: &mut Server<L>) -> Router<L> {
        builder
            .add_service(GreeterServer::new(GreeterImpl))
            .add_service(CounterServer::new(CounterImpl::new(self.state.counter)))
            .add_service(EchoServer::new(EchoImpl))
            .add_service(RandomServer::new(RandomImpl))
            .add_service(WaitServer::new(WaitImpl))
    }

    pub async fn health_service() -> HealthServer<impl Health> {
        let (health_reporter, health_service) = tonic_health::server::health_reporter();
        health_reporter.set_serving::<HealthServer<HealthService>>().await;
        health_reporter.set_serving::<GreeterServer<GreeterImpl>>().await;
        health_reporter.set_serving::<CounterServer<CounterImpl>>().await;
        health_reporter.set_serving::<EchoServer<EchoImpl>>().await;
        health_reporter.set_serving::<RandomServer<RandomImpl>>().await;
        health_reporter.set_serving::<WaitServer<WaitImpl>>().await;

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
}

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub rc: Arc<RunCommand>,
    pub counter: CounterState,
}
