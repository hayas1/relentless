//! Relentless HTTP load testing / comparison testing tool
//!
//! # Binary Usage
//! ## Install
//! ```sh
//! cargo install --git https://github.com/hayas1/relentless relentless
//! ```
//!
//! ## Prepare Config
//! ```yaml
//! name: basic comparison test
//! destinations:
//!   actual: http://localhost:3000
//!   expect: http://localhost:3000
//!
//! testcases:
//!   - target: /
//!   - target: /health
//!   - target: /healthz
//! ```
//! ...more examples in <https://github.com/hayas1/relentless/tree/master/examples/config>
//!
//! ### Run API for testing
//! If you have no API for testing, you can use `relentless-dev-server-http`
//! ```sh
//! cargo install --git https://github.com/hayas1/relentless relentless-dev-server-http
//! relentless-dev-server-http
//! ```
//!
//! ## Run CLI
//! ```sh
//! relentless -f examples/config/compare.yaml
//! ```
//! ```sh
//! 🚀 basic comparison test 🚀
//!   actual🌐 http://localhost:3000/
//!   expect🌐 http://localhost:3000/
//!   ✅ /
//!   ✅ /health
//!   ✅ /healthz
//!
//! 💥 summery of all requests in configs 💥
//!   pass-rt: 3/3=100.00%    rps: 6req/22.37ms=268.23req/s
//!   latency: min=2.774ms mean=8.194ms p50=5.219ms p90=22.127ms p99=22.127ms max=22.127ms
//! ```
//! In this case the actual and expected are the same server, so the request gets the same response and the test passes.
//! - Each request is done **concurrently** by default.
//!
//! # Library Usage
//! ## Install
//! TODO (feature)
//!
//! ## Prepare Config
//! Same config can be used in both binary and library. See [Binary section](#prepare-config).
//!
//! ### Run API for testing
//! Same `relentless-dev-server-http` can be used in both binary and library. See [Binary section](#run-api-for-testing).
//!
//! ## Run Testing
//! TODO <https://github.com/hayas1/relentless/blob/master/tests/tests.rs>
//!
//! # Documents
//! <https://hayas1.github.io/relentless/relentless>
//!
//! # Testing
//! ## coverage
//! <https://hayas1.github.io/relentless/tarpaulin-report.html>

pub mod assault;
pub mod error;
pub mod interface;

pub use {error::RelentlessError as Error, error::RelentlessResult as Result};
