//! Relentless HTTP comparison testing tool
//!
//! # Usage
//! TODO: see [relentless]

#[cfg(feature = "default-http-client")]
pub mod client;
pub mod command;
pub mod error;
pub mod evaluate;
pub mod factory;
pub mod record;
