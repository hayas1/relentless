use thiserror::Error;

use crate::testcase::{config::FormatError, http::HttpError};

pub type RelentlessResult<T, E = RelentlessError> = Result<T, E>;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RelentlessError {
    FormatError(#[from] FormatError),
    HttpError(#[from] HttpError),

    ReqwestError(#[from] reqwest::Error),
    TokioTaskJoinError(#[from] tokio::task::JoinError),

    BoxError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}
