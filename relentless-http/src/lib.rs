//! HTTP implementation of comparison testing framework [relentless].
//!
//! # Binary Usage
//! About the same as [relentless]
//! | step | command |
//! | --- | --- |
//! | install binary | `cargo install --git https://github.com/hayas1/relentless relentless-http --features cli` |
//! | install dev server | `cargo install --git https://github.com/hayas1/relentless relentless-http-dev-server` |
//! | run command | `relentless-http -f compare.yaml` |
//!
//! # Library Usage
//! ## Install
//! Often used in dev-dependencies.
//! ```sh
//! cargo add --dev --git https://github.com/hayas1/relentless relentless-http
//! ```
//! ```toml
//! [dev-dependencies]
//! relentless-http = { git = "https://github.com/hayas1/relentless" }
//! ```
//!
//! ## Testing
//! ```
//! # tokio_test::block_on(async {
//! use axum::body::Body;
//! use relentless::interface::{
//!     command::{Assault, Relentless},
//!     config::{Config, Format},
//! };
//! use relentless_http::command::HttpAssault;
//! use relentless_http_dev_server::route;
//!
//! let assault = HttpAssault::<Body, Body>::new(Relentless {
//!     file: vec![], // files can be specified also
//!     ..Default::default()
//! });
//! let config = r#"
//!     name: basic http comparison test
//!     destinations:
//!         actual: http://localhost:3000
//!         expect: http://localhost:3000
//!
//!     testcases:
//!     - target: /
//!     - target: /health
//!     - target: /healthz
//! "#;
//!
//! let configs = vec![Config::read_str(config, Format::Yaml).unwrap()];
//! let service = route::app_with(Default::default());
//! let report = assault.assault_with(configs, service).await.unwrap();
//!
//! assert!(assault.pass(&report));
//! # })
//! ```

#[cfg(feature = "default-http-client")]
pub mod client;
pub mod command;
pub mod error;
pub mod evaluate;
pub mod factory;
pub mod record;
