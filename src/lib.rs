//! Relentless HTTP load testing / comparison testing tool
//!
//! # Usage
//! TODO
//!
//! # Documents
//! <https://hayas1.github.io/relentless/relentless>
//!
//! # Testing
//! ## coverage
//! <https://hayas1.github.io/relentless/tarpaulin-report.html>

pub mod command;
pub mod config;
pub mod error;
pub mod evaluate;
pub mod outcome;
pub mod service;
pub mod worker;

pub use {error::RelentlessError as Error, error::RelentlessResult as Result};
