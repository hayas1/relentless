pub mod config;
pub mod error;
pub mod outcome;
pub mod service;
pub mod worker;

pub type Relentless = worker::Control<
    service::DefaultHttpClient<service::BytesBody, service::BytesBody>,
    service::BytesBody,
    service::BytesBody,
>;
