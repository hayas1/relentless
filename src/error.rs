use thiserror::Error;

use crate::testcase::format::FormatError;

pub type RelentlessResult<T, E = WrapError> = Result<T, E>;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum WrapError {
    RelentlessError(#[from] RelentlessError),
    FormatError(#[from] FormatError),

    ReqwestError(#[from] reqwest::Error),

    BoxError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Error, Debug)]
pub enum RelentlessError {} // TODO remove this
