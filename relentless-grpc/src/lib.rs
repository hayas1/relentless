//! gRPC implementation of comparison testing framework [relentless].
//!
//! # Binary Usage
//! ## Install
//! ```sh
//! cargo install --git https://github.com/hayas1/relentless relentless-grpc --features cli
//! ```
//! or get binary from [GitHub Releases](https://github.com/hayas1/relentless/releases).
//!
//! ## Prepare Config
//! For example, `examples/config/compare.yaml`
//! ```yaml
//! name: basic grpc comparison test
//! destinations:
//!   actual: http://localhost:50051
//!   expect: http://localhost:50051
//! contract:
//!   proto-files:
//!     protos:
//!       - ./dev/server/grpc/proto/greeter.proto
//!       - ./dev/server/grpc/proto/echo.proto
//!     includes: [./dev/server/grpc/proto]
//!
//! testcases:
//!   - target: greeter.Greeter/SayHello
//!     profile:
//!       request:
//!         message:
//!           value:
//!             name: John Doe
//!   - target: echo.Echo/EchoValue
//!     profile:
//!       request:
//!         message:
//!           value: 100
//! ```
//!
//! ### Run API for testing
//! Optional: if there is no API for testing, `relentless-grpc-dev-server` can be used.
//! ```sh
//! cargo install --git https://github.com/hayas1/relentless relentless-grpc-dev-server
//! relentless-grpc-dev-server
//! ```
//!
//! ## Run CLI
//! ```sh
//! relentless-grpc examples/config/compare.yaml
//! ```
//! ```plaintext
//! 🚀 basic grpc comparison test 🚀
//!   actual🌐 http://localhost:50051
//!   expect🌐 http://localhost:50051
//! ✅ greeter.Greeter/SayHello
//! ✅ echo.Echo/EchoValue
//! ```
//! In this case the `actual` and `expect` are the same server, so the response equivalence check passes. ✅
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
//! use relentless::{
//!     report::ReportFormat,
//!     shot::job::{Job, JobSpec},
//! };
//! use relentless_grpc::{contract::DynamicContract, wip::JsonSerializer};
//! use tower::make::Shared;
//!
//! let config = r#"
//! name: basic grpc comparison test
//! destinations:
//!   actual: http://localhost:50051
//!   expect: http://localhost:50051
//! contract:
//!   proto-files:
//!     protos:
//!       - ./dev/server/grpc/proto/greeter.proto
//!       - ./dev/server/grpc/proto/echo.proto
//!     includes: [./dev/server/grpc/proto]
//! testcases:
//!   - target: greeter.Greeter/SayHello
//!     profile:
//!       request:
//!         message:
//!           value:
//!             name: John Doe
//!   - target: echo.Echo/EchoValue
//!     profile:
//!       request:
//!         message:
//!           value: 100
//! "#;
//! let spec = JobSpec { report_format: ReportFormat::NullDevice, base_path: Some("..".parse().unwrap()), ..Default::default() };
//! let job = Job(vec![serde_yaml::from_str(config).unwrap()]);
//!
//! let service = relentless_grpc_dev_server::runner::RunCommand::default().app().routes();
//! let make = Shared::new(service);
//! let report = job.shot::<_, _, DynamicContract<serde_json::Value, JsonSerializer>>(make.clone(), &spec).await.unwrap();
//!
//! assert!(report.evaluated.pass);
//!
//! // Configuration can be read from YAML file also.
//! let job_from_file = Job::from_files(&["examples/config/compare.yaml"]).unwrap();
//! let report = job_from_file.shot::<_, _, DynamicContract<serde_json::Value, JsonSerializer>>(make.clone(), &spec).await.unwrap();
//! assert!(report.evaluated.pass);
//!
//! # })
//! ```

pub mod codec;
pub mod contract;
pub mod interceptor;
pub mod request;
pub mod response;
pub mod service;
pub mod wip;
