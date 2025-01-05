use thiserror::Error;

#[derive(Error, Debug)]
pub enum GrpcRequestError {
    #[error("cannot parse target {}", .0)]
    FailToParse(String),
    #[error("cannot find service {}", .0)]
    NoService(String),
    #[error("cannot find method {}", .0)]
    NoMethod(String),
}
