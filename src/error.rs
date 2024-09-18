use thiserror::Error;

pub type RelentlessResult<T, E = RelentlessError> = Result<T, E>;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum RelentlessError {
    FormatError(#[from] FormatError),
    HttpError(#[from] HttpError),
    CaseError(#[from] CaseError),

    ReqwestError(#[from] reqwest::Error),
    TokioTaskJoinError(#[from] tokio::task::JoinError),
    StdIoError(#[from] std::io::Error),
    Infallible(#[from] std::convert::Infallible),

    BoxError(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("unknown format extension: {0}")]
    UnknownFormatExtension(String),
    #[error("cannot specify format")]
    CannotSpecifyFormat,

    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[cfg(feature = "json")]
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[cfg(feature = "yaml")]
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
    #[cfg(feature = "toml")]
    #[error(transparent)]
    TomlError(#[from] toml::de::Error),
}

#[derive(Error, Debug)]
pub enum HttpError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    InvalidMethod(#[from] http::method::InvalidMethod),

    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),
}

#[derive(Error, Debug)]
pub enum CaseError {
    #[error("fail to clone request")]
    FailCloneRequest,
}
