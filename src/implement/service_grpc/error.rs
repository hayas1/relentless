use thiserror::Error;

use crate::{
    assault::result::RequestError,
    error::{IntoRelentlessError, JsonEvaluateError},
};

#[derive(Error, Debug)]
pub enum GrpcRequestError {
    #[error("cannot parse target {}", .0)]
    FailToParse(String),
    #[error("cannot find service {}", .0)]
    NoService(String),
    #[error("cannot find method {}", .0)]
    NoMethod(String),
    #[error("unexpected reflection response")]
    UnexpectedReflectionResponse,
}
impl IntoRelentlessError for GrpcRequestError {}

#[derive(Error, Debug)]
pub enum GrpcEvaluateError {
    #[error(transparent)]
    RequestError(#[from] RequestError),

    #[error("metadata map is not acceptable")]
    UnacceptableMetadataMap,
    #[error("extension is not acceptable")]
    UnacceptableExtension,

    #[cfg(feature = "json")]
    #[error(transparent)]
    JsonEvaluateError(#[from] JsonEvaluateError),
}
impl IntoRelentlessError for GrpcEvaluateError {}
