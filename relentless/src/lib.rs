//! Relentless load testing and comparison testing framework.
//!
//! # Features
//! - Available as a binary or library.
//! - If there is no API for testing, an example dev server can be used.
//! - [OpenTelemetry](https://opentelemetry.io/) support is provided (environment variables).
//!
//! # Usage
//! | | HTTP | GRPC |
//! | --- | --- | --- |
//! | Docs | [relentless-http](https://hayas1.github.io/relentless/relentless_http/) |[relentless-grpc](https://hayas1.github.io/relentless/relentless_grpc/) |
//!
//! # Documents
//! <https://hayas1.github.io/relentless/relentless>
//!
//! # Testing
//! ## Benchmarks
//! TODO
//!
//! ## Coverage
//! <https://hayas1.github.io/relentless/relentless/tarpaulin-report.html>
//!

pub mod error;
pub mod evaluator;
pub mod http_newtype_serde;
pub mod otel;
pub mod record;
pub mod report;
pub mod shot;
#[cfg(feature = "testing")]
pub mod testing;

pub use {error::RelentlessError as Error, error::RelentlessResult as Result};
