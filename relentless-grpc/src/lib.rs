//! GRPC implementation of comparison testing framework [relentless].
//!
//! # Binary Usage
//! About the same as [relentless]
//! | step | command |
//! | --- | --- |
//! | install binary | `cargo install --git https://github.com/hayas1/relentless relentless-grpc --features cli` |
//! | install dev server | `cargo install --git https://github.com/hayas1/relentless relentless-grpc-dev-server` |
//! | run command | `relentless-grpc -f compare.yaml` |
//!
//! # Library Usage
//! ## Install
//! Often used in dev-dependencies.
//! ```sh
//! cargo add --dev --git https://github.com/hayas1/relentless relentless-grpc
//! ```
//! ```toml
//! [dev-dependencies]
//! relentless-grpc = { git = "https://github.com/hayas1/relentless" }
//! ```
//!
//! ## Testing
#![cfg_attr(
    feature = "yaml",
    doc = r##"
 ```
 # tokio_test::block_on(async {
 use relentless::interface::{config::{Config, Format}, command::{Assault, Relentless}};
 use relentless_grpc::{client::GrpcClient, command::GrpcAssault};
 use relentless_grpc_dev_server::service::{
     counter::{pb::counter_server::CounterServer, CounterImpl},
     echo::{pb::echo_server::EchoServer, EchoImpl},
     greeter::{pb::greeter_server::GreeterServer, GreeterImpl},
 };
 use tonic::service::Routes;

 let assault = GrpcAssault::new(Relentless {
     file: vec![], // files can be specified also
     ..Default::default()
 });
 let config = r#"
   name: basic grpc comparison test
   destinations:
     actual: http://localhost:50051
     expect: http://localhost:50051

   testcases:
   - target: greeter.Greeter/SayHello
     setting:
       request:
         descriptor:
           protos: [../dev/server/grpc/proto/greeter.proto]
           import_path: [../dev/server/grpc/proto]
         message:
           json:
             name: John Doe
 "#;

 let configs = vec![Config::read_str(config, Format::Yaml).unwrap()];
 let destinations = assault.all_destinations(&configs);
 let mut builder = Routes::builder();
 builder
     .add_service(GreeterServer::new(GreeterImpl))
     .add_service(CounterServer::new(CounterImpl::default()))
     .add_service(EchoServer::new(EchoImpl));
 let routes = builder.routes();
 let service = GrpcClient::from_services(&destinations.into_iter().map(|d| (d, routes.clone())).collect()).await.unwrap();
 let report = assault.assault_with(configs, service).await.unwrap();

 assert!(assault.pass(&report));
 # })
```
"##
)]
pub mod client;
pub mod command;
pub mod error;
pub mod evaluate;
pub mod factory;
pub mod record;

pub mod helper;
