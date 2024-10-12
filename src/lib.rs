pub mod command;
pub mod config;
pub mod error;
pub mod outcome;
pub mod service;
pub mod worker;

pub use error::RelentlessError_ as Error;
pub type Result<T> = error::WrappedResult<T, Error>;
