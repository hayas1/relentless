use tonic::transport::{server::Router, Server};
use tower::layer::util::Identity;

use crate::{env::Env, state::AppState};

pub mod counter;
pub mod helloworld;

pub fn app_with(env: Env) -> Router<Identity> {
    let state = AppState { env, ..Default::default() };
    app(state)
}
pub fn app(state: AppState) -> Router<Identity> {
    router(state)
}
pub fn router(state: AppState) -> Router<Identity> {
    Server::builder()
        .trace_fn(|_| tracing::info_span!(env!("CARGO_PKG_NAME")))
        .add_service(helloworld::hello_world::greeter_server::GreeterServer::new(helloworld::MyGreeter::default()))
        .add_service(counter::pb::counter_server::CounterServer::new(counter::CounterImpl::default()))
}
