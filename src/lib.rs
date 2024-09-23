#[cfg(feature = "cli")]
pub mod cli;

pub mod config;
pub mod error;
pub mod outcome;
pub mod service;
pub mod worker;

/// TODO document
pub type Relentless = worker::Control<
    service::DefaultHttpClient<service::BytesBody, service::BytesBody>,
    service::BytesBody,
    service::BytesBody,
>;
