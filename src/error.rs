use thiserror::Error;

pub type RelentlessResult<T, E = WrapError> = Result<T, E>;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum WrapError {
    RelentlessError(#[from] RelentlessError),

    StdIoError(#[from] std::io::Error),
    YamlError(#[from] serde_yaml::Error),
    ReqwestError(#[from] reqwest::Error),

    BoxError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Error, Debug)]
pub enum RelentlessError {}
