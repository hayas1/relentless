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
//! ```
//! # tokio_test::block_on(async {
//! use relentless::interface::{
//!     command::{Assault, Relentless},
//!     config::{Config, Format},
//! };
//! use relentless_grpc::{client::DefaultGrpcClient, command::GrpcAssault};
//! use relentless_grpc_dev_server::service;
//!
//! let assault = GrpcAssault::new(Relentless {
//!     file: vec![], // files can be specified also
//!     ..Default::default()
//! });
//! let config = r#"
//!   name: basic grpc comparison test
//!   destinations:
//!       actual: http://localhost:50051
//!       expect: http://localhost:50051
//!
//!   testcases:
//!   - target: greeter.Greeter/SayHello
//!     setting:
//!       request:
//!         message:
//!           json:
//!             name: John Doe
//! "#;
//!
//! // TODO let configs = vec![Config::read_str(config, Format::Yaml).unwrap()];
//! // TODO let service = service::app(Default::default());
//! // TODO let report = assault.assault_with(configs, service).await.unwrap();
//!
//! // TODO assert!(assault.pass(&report));
//! # })
//! ```

pub mod client;
pub mod command;
pub mod error;
pub mod evaluate;
pub mod factory;
pub mod record;

pub mod helper;
