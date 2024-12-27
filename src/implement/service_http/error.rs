use thiserror::Error;

use crate::{
    assault::result::RequestError,
    error2::{JsonEvaluateError, PlaintextEvaluateError},
};

#[derive(Error, Debug)]
pub enum HttpEvaluateError {
    #[error(transparent)]
    RequestError(#[from] RequestError),

    #[error("status is not acceptable")]
    UnacceptableStatus,
    #[error("header map is not acceptable")]
    UnacceptableHeaderMap,

    #[error(transparent)]
    FailToCollectBody(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    PlaintextEvaluateError(#[from] PlaintextEvaluateError),
    #[cfg(feature = "json")]
    #[error(transparent)]
    JsonEvaluateError(#[from] JsonEvaluateError),
}