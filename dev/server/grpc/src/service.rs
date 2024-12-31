use num::BigInt;
use relentless_dev_server_grpc_entity::{
    counter_pb::counter_server::CounterServer, helloworld_pb::greeter_server::GreeterServer,
};
use tonic::transport::{server::Router, Server};
use tower::layer::util::Identity;

use crate::env::Env;

pub mod counter;
pub mod helloworld;

pub fn app_with(env: Env) -> Router<Identity> {
    app(env, 0.into())
}
pub fn app(env: Env, initial_count: BigInt) -> Router<Identity> {
    router(env, initial_count)
}
pub fn router(env: Env, initial_count: BigInt) -> Router<Identity> {
    let _ = env;
    Server::builder()
        .trace_fn(|_| tracing::info_span!(env!("CARGO_PKG_NAME")))
        .add_service(GreeterServer::new(helloworld::MyGreeter::default()))
        .add_service(CounterServer::new(counter::CounterImpl::new(initial_count)))
}
