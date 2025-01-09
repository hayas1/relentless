use thiserror::Error;

use crate::error::IntoRelentlessError;

#[derive(Error, Debug)]
pub enum GrpcRequestError {
    #[error("cannot parse target {}", .0)]
    FailToParse(String),
    #[error("cannot find service {}", .0)]
    NoService(String),
    #[error("cannot find method {}", .0)]
    NoMethod(String),
}
impl IntoRelentlessError for GrpcRequestError {}
