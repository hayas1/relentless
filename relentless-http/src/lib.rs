//! HTTP implementation of comparison testing framework [relentless].
//!
//! # Binary Usage
//! ## Install
//! ```sh
//! cargo install --git https://github.com/hayas1/relentless relentless-http --features cli
//! ```
//! or get binary from [GitHub Releases](https://github.com/hayas1/relentless/releases).
//!
//! ## Prepare Config
//! For example, `examples/config/compare.yaml`
//! ```yaml
//! name: basic http comparison test
//! destinations:
//!   actual: http://localhost:3000
//!   expect: http://localhost:3000
//!
//! testcases:
//!   - target: /
//!   - target: /health
//!   - target: /healthz
//! ```
//!
//! ### Run API for testing
//! Optional: if there is no API for testing, `relentless-http-dev-server` can be used.
//! ```sh
//! cargo install --git https://github.com/hayas1/relentless relentless-http-dev-server
//! relentless-http-dev-server
//! ```
//!
//! ## Run CLI
//! ```sh
//! relentless-http examples/config/compare.yaml
//! ```
//! ```plaintext
//! 🚀 basic http comparison test 🚀
//!   actual🌐 http://localhost:3000/
//!   expect🌐 http://localhost:3000/
//! ✅ /
//! ✅ /health
//! ✅ /healthz
//! ```
//! In this case the `actual` and `expect` are the same server, so the response equivalence check passes. ✅
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
//! use relentless::{
//!     report::ReportFormat,
//!     shot::job::{Job, JobSpec},
//! };
//! use relentless_http::contract::HttpContract;
//!
//! let config = r#"
//!     name: basic http comparison test
//!     destinations:
//!       actual: http://localhost:3000
//!       expect: http://localhost:3000
//!
//!     testcases:
//!     - target: /
//!     - target: /health
//!     - target: /healthz
//! "#;
//! let spec = JobSpec { report_format: ReportFormat::NullDevice, ..Default::default() };
//! let job = Job(vec![serde_yaml::from_str(config).unwrap()]);
//!
//! let service = relentless_http_dev_server::app::AppRouter::default().service();
//! let make = axum::ServiceExt::<axum::extract::Request>::into_make_service(service);
//! let report = job.shot::<_, _, HttpContract<axum::body::Body, axum::body::Body>>(make.clone(), &spec).await.unwrap();
//!
//! assert!(report.evaluated.pass);
//!
//! // Configuration can be read from YAML file also.
//! let job_from_file = Job::from_files(&["examples/config/compare.yaml"]).unwrap();
//! let report = job_from_file.shot::<_, _, HttpContract<axum::body::Body, axum::body::Body>>(make.clone(), &spec).await.unwrap();
//! assert!(report.evaluated.pass);
//!
//! # })
//! ```

pub mod contract;
pub mod layer;
pub mod request;
pub mod response;
pub mod service;
